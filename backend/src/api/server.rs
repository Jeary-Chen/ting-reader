//! HTTP Server implementation
//!
//! This module provides the HTTP server using Axum framework with:
//! - Configurable host/port binding
//! - Graceful shutdown handling
//! - Connection limits and request timeouts
//! - Health check endpoint
//! - CORS support

use crate::api::handlers::AppState;
use crate::api::middleware::{
    auth_middleware, security_headers_middleware, trace_id_middleware, ApiKey,
    SecurityHeadersConfig,
};
use crate::api::routes::build_api_routes;
use crate::core::config::ServerConfig;
use crate::core::Config;
use crate::db::manager::DatabaseManager;
use crate::db::repository::BookRepository;
use axum::{
    body::{to_bytes, Body},
    extract::{ConnectInfo, Request, State},
    http::{header, uri::PathAndQuery, Uri},
    middleware,
    middleware::Next,
    response::{IntoResponse, Json, Response},
    routing::{any, get},
    Router,
};
use serde_json::{json, Value};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{
    classify::ServerErrorsFailureClass,
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;

#[cfg(unix)]
use hyper::body::Incoming;
#[cfg(unix)]
use hyper::server::conn::http1;
#[cfg(unix)]
use hyper_util::rt::TokioIo;

/// HTTP API Server
pub struct ApiServer {
    router: Router,
    config: ServerConfig,
}

#[derive(Clone)]
struct GatewayProxyState {
    router: Router,
    prefix: String,
}

impl ApiServer {
    /// Create a new API server with the given configuration and database manager
    pub fn new(
        config: Config,
        db: Arc<DatabaseManager>,
        plugin_manager: Arc<crate::plugin::manager::PluginManager>,
        config_manager: Arc<crate::plugin::config::PluginConfigManager>,
        encryption_key: [u8; 32],
    ) -> anyhow::Result<Self> {
        let server_config = config.server.clone();

        // Build the router with all routes and middleware
        let router =
            Self::build_router(config, db, plugin_manager, config_manager, encryption_key)?;

        Ok(Self {
            router,
            config: server_config,
        })
    }

    /// Build the Axum router with all routes and middleware
    fn build_router(
        config: Config,
        db: Arc<DatabaseManager>,
        plugin_manager: Arc<crate::plugin::manager::PluginManager>,
        config_manager: Arc<crate::plugin::config::PluginConfigManager>,
        encryption_key: [u8; 32],
    ) -> anyhow::Result<Router> {
        // Create API key configuration for authentication
        let api_key = ApiKey::new(config.security.enable_auth, config.security.api_key.clone());

        // Create security headers configuration
        let security_headers_config =
            SecurityHeadersConfig::new(config.security.enable_hsts, config.security.hsts_max_age);

        // Create repositories
        let book_repo = Arc::new(BookRepository::new(db.clone()));
        let user_repo = Arc::new(crate::db::repository::UserRepository::new(db.clone()));
        let progress_repo = Arc::new(crate::db::repository::ProgressRepository::new(db.clone()));
        let favorite_repo = Arc::new(crate::db::repository::FavoriteRepository::new(db.clone()));
        let settings_repo = Arc::new(crate::db::repository::UserSettingsRepository::new(
            db.clone(),
        ));
        let library_repo = Arc::new(crate::db::repository::LibraryRepository::new(db.clone()));
        let chapter_repo = Arc::new(crate::db::repository::ChapterRepository::new(db.clone()));
        let series_repo = Arc::new(crate::db::repository::SeriesRepository::new(db.clone()));
        let playlist_repo = Arc::new(crate::db::repository::PlaylistRepository::new(db.clone()));
        let notification_repo = Arc::new(
            crate::db::repository::NotificationWebhookRepository::new(db.clone()),
        );

        // Initialize JWT key manager (auto-generates and rotates keys)
        let jwt_key_manager = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match crate::auth::JwtKeyManager::new(db.get_pool(), encryption_key).await {
                    Ok(manager) => {
                        tracing::info!(
                            message_key = "system.jwt_key.initialized",
                            "JWT key manager initialized"
                        );
                        let manager_arc = Arc::new(manager);
                        // 启动后台轮换任务
                        manager_arc.clone().start_rotation_task();
                        Some(manager_arc)
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            message_key = "system.jwt_key.init_failed",
                            message_params = %serde_json::json!({ "error": e.to_string() }),
                            "JWT key manager initialization failed; using configured secret"
                        );
                        None
                    }
                }
            })
        });

        // Get JWT secret from config (fallback)
        let jwt_secret = Arc::new(config.security.jwt_secret.clone());

        // Keep API state and plugin lifecycle on the same config manager instance.
        plugin_manager.set_config_manager(config_manager.clone());

        // Create services
        let book_service = Arc::new(crate::core::services::BookService::new(book_repo.clone()));
        let scraper_service = Arc::new(crate::core::services::ScraperService::new(
            plugin_manager.clone(),
        ));
        let merge_service = Arc::new(crate::core::merge_service::MergeService::new(
            book_repo.clone(),
            chapter_repo.clone(),
        ));

        // Create helpers
        let cleaner_config = crate::core::text_cleaner::CleanerConfig::default();
        let text_cleaner = Arc::new(crate::core::text_cleaner::TextCleaner::new(cleaner_config));

        let nfo_manager = Arc::new(crate::core::nfo_manager::NfoManager::new(
            config.storage.data_dir.clone(),
        ));

        // Create audio streamer with configuration
        let streamer_config = crate::core::audio_streamer::StreamerConfig {
            cache_enabled: config.audio.cache_enabled,
            cache_size: config.audio.cache_size,
            buffer_size: config.audio.buffer_size,
            supported_formats: vec![
                crate::core::audio_streamer::AudioFormat::Mp3,
                crate::core::audio_streamer::AudioFormat::M4a,
                crate::core::audio_streamer::AudioFormat::Aac,
                crate::core::audio_streamer::AudioFormat::Flac,
                crate::core::audio_streamer::AudioFormat::Wma,
            ],
        };
        let audio_streamer = Arc::new(crate::core::audio_streamer::AudioStreamer::new(
            streamer_config,
        ));

        // Create StorageService
        let storage_service = Arc::new(crate::core::StorageService::new());

        // Create Preload Cache
        let preload_cache = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        // Create task queue
        let task_queue = Arc::new(
            crate::core::task_queue::TaskQueue::new(
                config.task_queue.clone(),
                db.clone(),
                config.storage.temp_dir.clone(),
            )
            .with_repositories(book_repo.clone(), chapter_repo.clone(), series_repo.clone())
            .with_library_repo(library_repo.clone())
            .with_scraper_service(scraper_service.clone())
            .with_text_cleaner(text_cleaner.clone())
            .with_nfo_manager(nfo_manager.clone())
            .with_audio_streamer(audio_streamer.clone())
            .with_plugin_manager(plugin_manager.clone())
            .with_storage_service(storage_service.clone())
            .with_merge_service(merge_service.clone())
            .with_notification_repo(notification_repo.clone())
            .with_encryption_key(Arc::new(encryption_key)),
        );

        // Start task queue executor
        let task_queue_clone = task_queue.clone();
        tokio::spawn(async move {
            if let Err(e) = task_queue_clone.recover_tasks().await {
                tracing::error!(
                    error = %e,
                    message_key = "task.recovery.failed",
                    message_params = %serde_json::json!({ "error": e.to_string() }),
                    "Task recovery failed"
                );
            }
            task_queue_clone.start().await;
        });

        // Wrap config in Arc<RwLock> for shared mutable access
        let config_arc = Arc::new(tokio::sync::RwLock::new(config.clone()));

        // Create cache manager
        let cache_manager = Arc::new(
            crate::cache::CacheManager::new(config.storage.temp_dir.clone())
                .map_err(|e| anyhow::anyhow!("Failed to create cache manager: {}", e))?,
        );
        let plugin_cache = Arc::new(
            crate::plugin::PluginCache::new(config.storage.data_dir.join("plugin-cache"))
                .map_err(|e| anyhow::anyhow!("Failed to create plugin cache: {}", e))?,
        );
        let plugin_host_gateway = Arc::new(crate::plugin::PluginHostGateway::new(
            book_repo.clone(),
            library_repo.clone(),
            chapter_repo.clone(),
            progress_repo.clone(),
            playlist_repo.clone(),
            favorite_repo.clone(),
            settings_repo.clone(),
            task_queue.clone(),
            plugin_manager.clone(),
            plugin_cache.clone(),
            Arc::new(encryption_key),
            config.clone(),
        ));
        plugin_manager.set_host_gateway(&plugin_host_gateway);

        // Create library watcher
        let library_watcher = Arc::new(crate::core::library_watcher::LibraryWatcher::new(
            library_repo.clone(),
            task_queue.clone(),
            config.clone(),
        ));

        // Start watching all local libraries
        let watcher_clone = library_watcher.clone();
        tokio::spawn(async move {
            if let Err(e) = watcher_clone.start_all().await {
                tracing::warn!(
                    error = %e,
                    message_key = "library.watcher.start_failed",
                    message_params = %serde_json::json!({ "error": e.to_string() }),
                    "Library watcher failed to start"
                );
            }
        });

        // Create WebSocket session manager
        let ws_manager = crate::api::ws::manager::WsSessionManager::new();

        // Create HLS session manager
        let hls_temp_dir = config.storage.temp_dir.join("ting_hls_sessions");
        std::fs::create_dir_all(&hls_temp_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create HLS temp directory: {}", e))?;
        let hls_session_manager = Arc::new(
            crate::api::handlers::media::stream::HlsSessionManager::new(hls_temp_dir),
        );

        // Start HLS session cleanup task (runs every 10 minutes)
        let hls_manager_clone = hls_session_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                interval.tick().await;
                hls_manager_clone.cleanup_expired().await;
            }
        });

        // Create application state
        let app_state = AppState {
            book_repo,
            user_repo,
            progress_repo,
            favorite_repo,
            settings_repo,
            library_repo,
            chapter_repo,
            series_repo,
            playlist_repo,
            notification_repo,
            book_service,
            scraper_service,
            plugin_manager,
            plugin_cache,
            plugin_host_gateway,
            config_manager,
            task_queue,
            config: config_arc,
            jwt_secret,
            jwt_key_manager, // 新增密钥管理器
            cache_manager,
            encryption_key: Arc::new(encryption_key),
            storage_service,
            preload_cache,
            audio_streamer,
            merge_service,
            nfo_manager,
            active_preload_tasks: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            library_watcher,
            ws_manager,
            hls_session_manager,
        };

        // Create public routes (no authentication required)
        let public_router = Router::new()
            .route("/health", get(health_check))
            .route(
                "/api/auth/login",
                axum::routing::post(crate::auth::handlers::login),
            )
            .route(
                "/api/auth/token-login",
                axum::routing::post(crate::auth::handlers::token_login),
            )
            .route(
                "/api/auth/session-restore",
                axum::routing::post(crate::auth::handlers::session_restore),
            )
            .route(
                "/api/v1/auth/token-login",
                axum::routing::post(crate::auth::handlers::token_login),
            )
            .route(
                "/api/v1/auth/session-restore",
                axum::routing::post(crate::auth::handlers::session_restore),
            )
            .route(
                "/api/auth/register",
                axum::routing::post(crate::auth::handlers::register),
            )
            // WebSocket endpoint — handles auth internally via query param token
            .route("/api/ws", get(crate::api::ws::handler::ws_handler))
            .route("/api/v1/ws", get(crate::api::ws::handler::ws_handler))
            .with_state(app_state.clone());

        // Create protected routes (authentication required)
        let protected_router = build_api_routes(app_state.clone()).layer(middleware::from_fn(
            move |mut req: Request, next: Next| {
                let api_key = api_key.clone();
                async move {
                    // Inject API key into request extensions
                    req.extensions_mut().insert(api_key);
                    // Call auth middleware
                    auth_middleware(req, next).await
                }
            },
        ));

        // Combine public and protected routes
        let api_router = Router::new().merge(public_router).merge(protected_router);

        // Static file serving for SPA
        let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string());
        let index_path = std::path::PathBuf::from(&static_dir).join("index.html");
        let serve_dir = ServeDir::new(&static_dir).fallback(ServeFile::new(index_path));

        // Apply global middleware layers
        let router = api_router.fallback_service(serve_dir).layer(
            ServiceBuilder::new()
                // Add security headers middleware
                .layer(middleware::from_fn(move |mut req: Request, next: Next| {
                    let config = security_headers_config.clone();
                    async move {
                        req.extensions_mut().insert(config);
                        security_headers_middleware(req, next).await
                    }
                }))
                // Add trace ID middleware for request tracking
                .layer(middleware::from_fn(trace_id_middleware))
                // Add tracing for all requests
                .layer(TraceLayer::new_for_http().on_failure(
                    |classification: ServerErrorsFailureClass,
                     latency: Duration,
                     span: &tracing::Span| {
                        let latency_ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);
                        match classification {
                            ServerErrorsFailureClass::StatusCode(status_code) => {
                                tracing::error!(
                                    parent: span,
                                    status_code = status_code.as_u16(),
                                    latency_ms = latency_ms,
                                    message_key = "http.response.status_failed",
                                    message_params = %serde_json::json!({
                                        "status_code": status_code.as_u16(),
                                        "latency_ms": latency_ms,
                                    }),
                                    "HTTP response returned failure status"
                                );
                            }
                            ServerErrorsFailureClass::Error(error) => {
                                tracing::error!(
                                    parent: span,
                                    error = %error,
                                    latency_ms = latency_ms,
                                    message_key = "http.response.service_failed",
                                    message_params = %serde_json::json!({
                                        "latency_ms": latency_ms,
                                    }),
                                    "HTTP service failed"
                                );
                            }
                        }
                    },
                ))
                // Add CORS support
                .layer(Self::build_cors_layer(&config.security.allowed_origins)),
        );

        // The same route tree serves both the direct TCP endpoint and the
        // fnOS gateway endpoint. The gateway forwards the registered prefix
        // to this router, so the existing direct URLs remain unchanged.
        let router = if let Some(prefix) =
            normalize_gateway_prefix(config.server.gateway_prefix.as_deref())
        {
            let gateway_state = GatewayProxyState {
                router: router.clone(),
                prefix: prefix.clone(),
            };
            let gateway_path = prefix.clone();
            let gateway_wildcard_path = format!("{prefix}/*path");

            Router::new()
                .route(&gateway_path, any(gateway_proxy))
                .route(&gateway_wildcard_path, any(gateway_proxy))
                .with_state(gateway_state)
                .merge(router)
        } else {
            router
        };

        Ok(router)
    }

    /// Build CORS layer from allowed origins configuration
    fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
        use tower_http::cors::Any;

        let cors = CorsLayer::new();

        // If allowed_origins contains "*", allow any origin
        if allowed_origins.contains(&"*".to_string()) {
            cors.allow_origin(Any).allow_methods(Any).allow_headers(Any)
        } else {
            // Parse allowed origins
            let origins: Vec<_> = allowed_origins
                .iter()
                .filter_map(|origin| origin.parse().ok())
                .collect();

            cors.allow_origin(origins)
                .allow_methods(Any)
                .allow_headers(Any)
        }
    }

    /// Start the HTTP server and listen for requests
    ///
    /// This method will block until the server is shut down gracefully.
    pub async fn serve(self) -> anyhow::Result<()> {
        let socket_addr = match self.config.host.parse::<IpAddr>() {
            Ok(ip) => SocketAddr::new(ip, self.config.port),
            Err(_) => format!("{}:{}", self.config.host, self.config.port).parse()?,
        };

        info!(
            host = %self.config.host,
            port = self.config.port,
            max_connections = self.config.max_connections,
            request_timeout = self.config.request_timeout,
            message_key = "system.http.starting",
            message_params = %serde_json::json!({
                "host": self.config.host,
                "port": self.config.port,
                "max_connections": self.config.max_connections,
                "request_timeout": self.config.request_timeout,
            }),
            "Starting HTTP server"
        );

        // Create TCP listener. This is intentionally kept enabled even when
        // the fnOS Unix Socket gateway is configured for backward-compatible
        // direct port access.
        let listener = tokio::net::TcpListener::bind(socket_addr).await?;

        info!(
            addr = %socket_addr,
            message_key = "system.http.listening",
            message_params = %serde_json::json!({ "addr": socket_addr.to_string() }),
            "HTTP server listening"
        );

        #[cfg(unix)]
        let gateway_socket = self.config.gateway_socket.clone();

        #[cfg(unix)]
        let gateway_listener = if let Some(path) = gateway_socket.as_ref() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if path.exists() {
                std::fs::remove_file(path)?;
            }

            let listener = tokio::net::UnixListener::bind(path)?;

            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;

            info!(
                socket = %path.display(),
                prefix = ?self.config.gateway_prefix,
                "fnOS unified gateway socket listening"
            );
            Some(listener)
        } else {
            None
        };

        #[cfg(unix)]
        if let Some(gateway_listener) = gateway_listener {
            let tcp_server = axum::serve(
                listener,
                self.router
                    .clone()
                    .into_make_service_with_connect_info::<SocketAddr>(),
            );
            let gateway_server = serve_unix_socket(gateway_listener, self.router.clone());

            tokio::select! {
                result = tcp_server => result?,
                result = gateway_server => result?,
                _ = shutdown_signal() => {},
            }

            if let Some(path) = gateway_socket.as_ref() {
                let _ = std::fs::remove_file(path);
            }
        } else {
            axum::serve(
                listener,
                self.router
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        }

        #[cfg(not(unix))]
        {
            if self.config.gateway_socket.is_some() {
                tracing::warn!(
                    "fnOS unified gateway socket is configured but Unix sockets are unavailable on this platform"
                );
            }

            axum::serve(
                listener,
                self.router
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        }

        info!("HTTP server shut down gracefully");

        Ok(())
    }

    /// Get a reference to the router
    pub fn router(&self) -> &Router {
        &self.router
    }
}

#[cfg(unix)]
async fn serve_unix_socket(
    listener: tokio::net::UnixListener,
    router: Router,
) -> anyhow::Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let router = router.clone();

        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let service = hyper::service::service_fn(move |request: hyper::Request<Incoming>| {
                let router = router.clone();
                async move { router.oneshot(request.map(Body::new)).await }
            });

            if let Err(error) = http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                tracing::debug!(error = %error, "Unix socket HTTP connection closed with error");
            }
        });
    }
}

fn normalize_gateway_prefix(prefix: Option<&str>) -> Option<String> {
    let prefix = prefix?.trim();
    if prefix.is_empty() {
        return None;
    }

    let mut normalized = if prefix.starts_with('/') {
        prefix.to_string()
    } else {
        format!("/{prefix}")
    };
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    Some(normalized)
}

async fn gateway_proxy(State(state): State<GatewayProxyState>, mut request: Request) -> Response {
    let original_uri = request.uri().clone();
    let stripped_path = original_uri
        .path()
        .strip_prefix(&state.prefix)
        .filter(|path| !path.is_empty())
        .unwrap_or("/");
    let path_and_query = match original_uri.query() {
        Some(query) => format!("{stripped_path}?{query}"),
        None => stripped_path.to_string(),
    };

    let mut parts = original_uri.into_parts();
    parts.path_and_query = match PathAndQuery::try_from(path_and_query) {
        Ok(path_and_query) => Some(path_and_query),
        Err(_) => {
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid gateway path").into_response()
        }
    };
    let rewritten_uri = match Uri::from_parts(parts) {
        Ok(uri) => uri,
        Err(_) => {
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid gateway URI").into_response()
        }
    };
    *request.uri_mut() = rewritten_uri;

    // The outer `/*path` route has already inserted its wildcard into Axum's
    // private URL-parameter extension. Re-dispatching the same request through
    // the inner router would append the inner route parameter, so handlers such
    // as `Path<String>` would see two values for a one-parameter route.
    // Rebuild the routing extensions before the second dispatch while retaining
    // the connection address used by login auditing and Hyper's pending upgrade.
    let connect_info = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .copied();
    let on_upgrade = request
        .extensions_mut()
        .remove::<hyper::upgrade::OnUpgrade>();
    request.extensions_mut().clear();
    if let Some(connect_info) = connect_info {
        request.extensions_mut().insert(connect_info);
    }
    if let Some(on_upgrade) = on_upgrade {
        request.extensions_mut().insert(on_upgrade);
    }

    match state.router.oneshot(request).await {
        Ok(response) => rewrite_gateway_html(response, &state.prefix).await,
        Err(error) => match error {},
    }
}

async fn rewrite_gateway_html(response: Response, prefix: &str) -> Response {
    let is_html = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("text/html"))
        .unwrap_or(false);

    if !is_html {
        return response;
    }

    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = to_bytes(body, 8 * 1024 * 1024).await else {
        return Response::from_parts(parts, Body::empty());
    };
    let Ok(mut html) = String::from_utf8(bytes.to_vec()) else {
        return Response::from_parts(parts, Body::from(bytes));
    };

    // The Docker image is also used for direct port access, so its Vite build
    // keeps root-relative assets. Rewrite only gateway HTML responses to the
    // registered prefix, keeping the direct TCP page fully compatible.
    let asset_prefix = format!("{prefix}/");
    html = html.replace("src=\"/", &format!("src=\"{asset_prefix}"));
    html = html.replace("href=\"/", &format!("href=\"{asset_prefix}"));
    html = html.replace("<head>", &format!("<head><base href=\"{asset_prefix}\">"));

    parts.headers.remove(header::CONTENT_LENGTH);
    Response::from_parts(parts, Body::from(html))
}

/// Health check endpoint handler
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().timestamp(),
    }))
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received SIGTERM signal");
        },
    }

    info!("Initiating graceful shutdown...");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_server_creation() {
        // Test disabled due to complexity of mocking PluginManager
        /*
        let config = Config::from_file(std::path::Path::new("config.test.toml"))
            .expect("Failed to load test config");

        // Create an in-memory database for testing
        let db = Arc::new(DatabaseManager::new_in_memory().expect("Failed to create test database"));

        let server = ApiServer::new(config, db);
        assert!(server.is_ok());
        */
    }

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        let value = response.0;

        assert_eq!(value["status"], "ok");
        assert!(value["version"].is_string());
        assert!(value["timestamp"].is_number());
    }

    #[tokio::test]
    async fn gateway_proxy_does_not_leak_outer_wildcard_path_param() {
        use axum::extract::Path;

        async fn get_library(Path(id): Path<String>) -> String {
            id
        }

        let inner_router = Router::new().route("/api/libraries/:id", get(get_library));
        let state = GatewayProxyState {
            router: inner_router,
            prefix: "/app/ting-reader".to_string(),
        };
        let app = Router::new()
            .route("/app/ting-reader", any(gateway_proxy))
            .route("/app/ting-reader/*path", any(gateway_proxy))
            .with_state(state);

        let request = Request::builder()
            .uri("/app/ting-reader/api/libraries/library-1")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        assert_eq!(body.as_ref(), b"library-1");
    }
}
