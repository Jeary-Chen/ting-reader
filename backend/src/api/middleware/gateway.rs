//! Middleware to strip fnOS unified gateway prefix from request paths.

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Strip `/app/ting-reader` prefix from request URI.
/// This allows the backend to serve both direct TCP and gateway requests.
pub async fn strip_gateway_prefix(mut req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    if let Some(new_path) = path.strip_prefix("/app/ting-reader") {
        let new_path = if new_path.is_empty() { "/" } else { new_path };
        let new_path_and_query = match req.uri().query() {
            Some(q) => format!("{}?{}", new_path, q),
            None => new_path.to_string(),
        };
        if let Ok(new_uri) = new_path_and_query.parse() {
            *req.uri_mut() = new_uri;
        }
    }
    next.run(req).await
}
