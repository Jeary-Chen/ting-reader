#[cfg(test)]
use super::*;

#[cfg(test)]
use tempfile::TempDir;

fn write_fake_npm(temp_dir: &TempDir) -> PathBuf {
    #[cfg(target_family = "windows")]
    {
        let path = temp_dir.path().join("fake-npm.cmd");
        std::fs::write(
            &path,
            r#"@echo off
if "%1"=="--version" (
  echo 10.0.0
  exit /b 0
)
if not exist node_modules\cache-a mkdir node_modules\cache-a
if not exist node_modules\cache-b mkdir node_modules\cache-b
if not exist node_modules\transitive-helper mkdir node_modules\transitive-helper
echo module.exports = {};>node_modules\cache-a\index.js
echo module.exports = {};>node_modules\cache-b\index.js
echo module.exports = {};>node_modules\transitive-helper\index.js
echo ran>npm-install-ran.txt
exit /b 0
"#,
        )
        .unwrap();
        path
    }

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_dir.path().join("fake-npm");
        std::fs::write(
            &path,
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  echo 10.0.0
  exit 0
fi
mkdir -p node_modules/cache-a node_modules/cache-b node_modules/transitive-helper
printf 'module.exports = {};\n' > node_modules/cache-a/index.js
printf 'module.exports = {};\n' > node_modules/cache-b/index.js
printf 'module.exports = {};\n' > node_modules/transitive-helper/index.js
printf 'ran\n' > npm-install-ran.txt
"#,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }
}

#[test]
fn test_command_timeout_terminates_the_process_tree() {
    #[cfg(target_family = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "ping -n 6 127.0.0.1 >nul"]);
        command
    };
    #[cfg(target_family = "unix")]
    let mut command = {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 5"]);
        command
    };

    let started = Instant::now();
    let error = run_command_with_timeout(&mut command, Duration::from_millis(100), "timeout test")
        .unwrap_err();

    assert!(error.to_string().contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(4));
}

#[test]
fn test_npm_dependency_creation() {
    let dep = NpmDependency::new("axios".to_string(), "1.6.0".to_string());
    assert_eq!(dep.name, "axios");
    assert_eq!(dep.version, "1.6.0");
}

#[test]
fn test_registry_dependency_validation_accepts_safe_specs() {
    for (name, version) in [
        ("axios", "1.6.0"),
        ("@types/node", "22.0.0"),
        ("package-with.dots", "2.1.0"),
        ("package_underscore", "0.0.0-beta.1"),
    ] {
        NpmDependency::new(name.to_string(), version.to_string())
            .validate()
            .unwrap();
    }
}

#[test]
fn test_registry_dependency_validation_rejects_non_registry_specs() {
    for version in [
        "file:../package",
        "link:../package",
        "git:https://example.com/package.git",
        "git+ssh://git@example.com/package.git",
        "http://example.com/package.tgz",
        "https://example.com/package.tgz",
        "workspace:*",
        "npm:other-package@1.0.0",
        "../package",
        "C:\\package",
        "latest",
        "^1.6.0",
        "~2.1.0",
        "*",
        ">=18.0.0 <23.0.0",
        "1.2.3 || 2.0.0",
        "1.2.3 - 2.3.4",
    ] {
        assert!(
            NpmDependency::new("axios".to_string(), version.to_string())
                .validate()
                .is_err(),
            "unexpectedly accepted {version}"
        );
    }
}

#[test]
fn test_registry_dependency_validation_rejects_unsafe_names() {
    for name in [
        "Axios",
        "../axios",
        "@scope/../axios",
        "@scope",
        "@scope/pkg/extra",
        "_hidden",
        "trailing-",
        "space package",
        "https://example.com/pkg",
    ] {
        assert!(
            NpmDependency::new(name.to_string(), "1.0.0".to_string())
                .validate()
                .is_err(),
            "unexpectedly accepted {name}"
        );
    }
}

#[test]
fn test_vulnerability_severity_ordering() {
    assert!(VulnerabilitySeverity::Low < VulnerabilitySeverity::Moderate);
    assert!(VulnerabilitySeverity::Moderate < VulnerabilitySeverity::High);
    assert!(VulnerabilitySeverity::High < VulnerabilitySeverity::Critical);
}

#[test]
fn test_vulnerability_severity_from_str() {
    assert_eq!(
        VulnerabilitySeverity::parse("low"),
        Some(VulnerabilitySeverity::Low)
    );
    assert_eq!(
        VulnerabilitySeverity::parse("moderate"),
        Some(VulnerabilitySeverity::Moderate)
    );
    assert_eq!(
        VulnerabilitySeverity::parse("high"),
        Some(VulnerabilitySeverity::High)
    );
    assert_eq!(
        VulnerabilitySeverity::parse("critical"),
        Some(VulnerabilitySeverity::Critical)
    );
    assert_eq!(VulnerabilitySeverity::parse("invalid"), None);
}

#[test]
fn test_security_config_default() {
    let config = NpmSecurityConfig::default();
    assert!(config.whitelist.is_empty());
    assert!(config.enforce_version_lock);
    assert!(!config.enable_audit);
    assert!(!config.fail_on_audit_vulnerabilities);
    assert_eq!(
        config.max_vulnerability_severity,
        VulnerabilitySeverity::High
    );
}

#[test]
fn test_security_config_with_whitelist() {
    use std::collections::HashSet;
    let mut whitelist = HashSet::new();
    whitelist.insert("axios".to_string());
    whitelist.insert("cheerio".to_string());

    let config = NpmSecurityConfig {
        whitelist,
        enforce_version_lock: true,
        enable_audit: true,
        fail_on_audit_vulnerabilities: true,
        max_vulnerability_severity: VulnerabilitySeverity::Moderate,
    };

    assert_eq!(config.whitelist.len(), 2);
    assert!(config.whitelist.contains("axios"));
    assert!(config.whitelist.contains("cheerio"));
}

#[test]
fn test_parse_dependencies_from_json() {
    let plugin_json = serde_json::json!({
        "name": "test-plugin",
        "npm_dependencies": {
            "axios": "1.6.0",
            "cheerio": "1.0.0"
        }
    });

    let deps = NpmManager::parse_dependencies(&plugin_json);
    assert_eq!(deps.len(), 2);
    let dep_names: Vec<String> = deps.iter().map(|d| d.name.clone()).collect();
    assert!(dep_names.contains(&"axios".to_string()));
    assert!(dep_names.contains(&"cheerio".to_string()));
}

#[test]
fn test_parse_dependencies_empty() {
    let plugin_json = serde_json::json!({ "name": "test-plugin" });
    let deps = NpmManager::parse_dependencies(&plugin_json);
    assert_eq!(deps.len(), 0);
}

#[test]
fn test_package_json_creation() {
    let deps = vec![
        NpmDependency::new("axios".to_string(), "1.6.0".to_string()),
        NpmDependency::new("cheerio".to_string(), "1.0.0".to_string()),
    ];

    let package_json = PackageJson::from_plugin_metadata(
        "test-plugin",
        "1.0.0",
        Some("Test plugin"),
        Some("Test Author"),
        Some("MIT"),
        &deps,
    );

    assert_eq!(package_json.name, "test-plugin");
    assert_eq!(package_json.version, "1.0.0");
    assert_eq!(package_json.description, Some("Test plugin".to_string()));
    assert_eq!(package_json.author, Some("Test Author".to_string()));
    assert_eq!(package_json.license, Some("MIT".to_string()));
    assert_eq!(package_json.dependencies.len(), 2);
    assert_eq!(
        package_json.dependencies.get("axios"),
        Some(&"1.6.0".to_string())
    );
    assert_eq!(
        package_json.dependencies.get("cheerio"),
        Some(&"1.0.0".to_string())
    );
    assert!(package_json.private);
}

#[test]
fn test_package_json_write_and_read() {
    let temp_dir = TempDir::new().unwrap();
    let package_json_path = temp_dir.path().join("package.json");

    let deps = vec![NpmDependency::new("axios".to_string(), "1.6.0".to_string())];
    let package_json = PackageJson::from_plugin_metadata(
        "test-plugin",
        "1.0.0",
        Some("Test plugin"),
        Some("Test Author"),
        Some("MIT"),
        &deps,
    );

    package_json.write_to_file(&package_json_path).unwrap();
    assert!(package_json_path.exists());

    let read_package_json = PackageJson::read_from_file(&package_json_path).unwrap();
    assert_eq!(read_package_json.name, "test-plugin");
    assert_eq!(read_package_json.version, "1.0.0");
    assert_eq!(read_package_json.dependencies.len(), 1);
}

#[test]
fn test_npm_manager_creation() {
    let manager = NpmManager::default();
    // Can't access private fields directly; test public API
    assert!(!manager.is_cached("axios", "1.6.0"));
}

#[test]
fn test_npm_install_disables_lifecycle_scripts() {
    let temp_dir = TempDir::new().unwrap();
    let manager = NpmManager::default();
    let command = manager.build_install_command(temp_dir.path());
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert!(args.iter().any(|arg| arg == "--ignore-scripts=true"));
    assert!(args.iter().any(|arg| arg == "--package-lock=false"));
    assert!(args
        .iter()
        .any(|arg| arg == "--registry=https://registry.npmjs.org/"));
    assert_eq!(command.get_current_dir(), Some(temp_dir.path()));
}

#[test]
fn test_npm_install_rejects_project_config_and_lockfiles() {
    for filename in [".npmrc", "package-lock.json", "npm-shrinkwrap.json"] {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join(filename), "untrusted").unwrap();
        let manager = NpmManager::default();
        let package_json = PackageJson::from_plugin_metadata(
            "test-plugin",
            "1.0.0",
            None,
            None,
            None,
            &[NpmDependency::new("axios".to_string(), "1.6.0".to_string())],
        );

        let error = manager
            .prepare_trusted_install_files(temp_dir.path(), &package_json)
            .unwrap_err();
        assert!(error.to_string().contains(filename));
    }
}

#[test]
fn test_npm_install_rewrites_package_json_without_scripts() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("package.json");
    std::fs::write(
        &path,
        r#"{
          "name": "test-plugin",
          "version": "1.0.0",
          "private": true,
          "dependencies": { "axios": "1.6.0" },
          "scripts": { "preinstall": "exit 1", "postinstall": "exit 1" }
        }"#,
    )
    .unwrap();
    let package_json = PackageJson::read_from_file(&path).unwrap();

    NpmManager::default()
        .prepare_trusted_install_files(temp_dir.path(), &package_json)
        .unwrap();

    let rewritten = std::fs::read_to_string(path).unwrap();
    assert!(!rewritten.contains("scripts"));
    assert!(!rewritten.contains("preinstall"));
    assert!(rewritten.contains("axios"));
}

#[test]
fn test_npm_install_rejects_non_directory_node_modules() {
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("node_modules"), "not a directory").unwrap();
    let package_json = PackageJson::from_plugin_metadata(
        "test-plugin",
        "1.0.0",
        None,
        None,
        None,
        &[NpmDependency::new("axios".to_string(), "1.6.0".to_string())],
    );

    let error = NpmManager::default()
        .prepare_trusted_install_files(temp_dir.path(), &package_json)
        .unwrap_err();
    assert!(error.to_string().contains("node_modules"));
}

#[test]
fn test_cached_install_still_builds_complete_dependency_graph() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("plugin");
    let cache_dir = temp_dir.path().join("cache");
    let cached_source = temp_dir.path().join("cached-source");
    std::fs::create_dir_all(&plugin_dir).unwrap();
    std::fs::create_dir_all(&cached_source).unwrap();
    std::fs::write(cached_source.join("index.js"), "module.exports = {};").unwrap();

    let dependencies = vec![
        NpmDependency::new("cache-a".to_string(), "1.0.0".to_string()),
        NpmDependency::new("cache-b".to_string(), "1.0.0".to_string()),
    ];
    PackageJson::from_plugin_metadata("test-plugin", "1.0.0", None, None, None, &dependencies)
        .write_to_file(&plugin_dir.join("package.json"))
        .unwrap();

    let manager = NpmManager::new(Some(write_fake_npm(&temp_dir)), Some(cache_dir));
    cache::add_to_cache(
        &manager.cache_dir,
        &manager.cache_registry,
        &manager.cache_stats,
        "cache-a",
        "1.0.0",
        "first-plugin",
        &cached_source,
    )
    .unwrap();

    manager
        .install_dependencies_with_cache(&plugin_dir, "test-plugin", &dependencies)
        .unwrap();

    assert!(plugin_dir.join("node_modules/cache-a/index.js").is_file());
    assert!(plugin_dir.join("node_modules/cache-b/index.js").is_file());
    assert!(plugin_dir
        .join("node_modules/transitive-helper/index.js")
        .is_file());
    assert!(plugin_dir.join("npm-install-ran.txt").is_file());
    let package_json = PackageJson::read_from_file(&plugin_dir.join("package.json")).unwrap();
    assert_eq!(package_json.dependencies.len(), 2);
    assert_eq!(
        package_json.dependencies.get("cache-a"),
        Some(&"1.0.0".to_string())
    );
    assert_eq!(
        package_json.dependencies.get("cache-b"),
        Some(&"1.0.0".to_string())
    );
}

#[test]
fn test_npm_manager_with_security() {
    use std::collections::HashSet;
    let mut whitelist = HashSet::new();
    whitelist.insert("axios".to_string());

    let security_config = NpmSecurityConfig {
        whitelist,
        enforce_version_lock: true,
        enable_audit: true,
        fail_on_audit_vulnerabilities: false,
        max_vulnerability_severity: VulnerabilitySeverity::High,
    };

    let log_dir = PathBuf::from("/tmp/npm_logs");
    let manager = NpmManager::with_security(None, None, security_config, Some(log_dir));
    // Test public API
    assert!(!manager.is_cached("axios", "1.6.0"));
}

#[test]
fn test_get_node_modules_path() {
    let manager = NpmManager::default();
    let plugin_dir = PathBuf::from("/path/to/plugin");
    let node_modules_path = manager.get_node_modules_path(&plugin_dir);
    assert_eq!(
        node_modules_path,
        PathBuf::from("/path/to/plugin/node_modules")
    );
}

#[test]
fn test_has_node_modules() {
    let temp_dir = TempDir::new().unwrap();
    let manager = NpmManager::default();
    assert!(!manager.has_node_modules(temp_dir.path()));

    let node_modules_path = temp_dir.path().join("node_modules");
    std::fs::create_dir(&node_modules_path).unwrap();
    assert!(manager.has_node_modules(temp_dir.path()));
}

#[test]
fn test_clean_node_modules() {
    let temp_dir = TempDir::new().unwrap();
    let manager = NpmManager::default();

    let node_modules_path = temp_dir.path().join("node_modules");
    std::fs::create_dir(&node_modules_path).unwrap();
    std::fs::write(node_modules_path.join("test.txt"), "test").unwrap();

    let result = manager.clean_node_modules(temp_dir.path());
    assert!(result.is_ok());
    assert!(!manager.has_node_modules(temp_dir.path()));
}

#[test]
fn test_clean_node_modules_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let manager = NpmManager::default();
    let result = manager.clean_node_modules(temp_dir.path());
    assert!(result.is_ok());
}

#[test]
fn test_dependency_install_log_serialization() {
    let deps = vec![NpmDependency::new("axios".to_string(), "1.6.0".to_string())];
    let log = DependencyInstallLog {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        plugin_name: "test-plugin".to_string(),
        dependencies: deps,
        success: true,
        error: None,
        audit_result: None,
    };

    let json = serde_json::to_string(&log).unwrap();
    assert!(json.contains("test-plugin"));
    assert!(json.contains("axios"));

    let deserialized: DependencyInstallLog = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.plugin_name, "test-plugin");
    assert!(deserialized.success);
}

#[test]
fn test_cache_key_generation() {
    let key = cache::get_cache_key("axios", "1.6.0");
    assert_eq!(key.len(), 71);
    assert!(key.starts_with("npm-v1-"));
    assert!(!key.contains("axios"));
    assert_eq!(key, cache::get_cache_key("axios", "1.6.0"));
    assert_ne!(key, cache::get_cache_key("axios", "1.6.1"));
}

#[test]
fn test_cache_cleanup_rejects_registry_path_outside_cache_root() {
    use std::collections::HashSet;

    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let outside = temp_dir.path().join("outside");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    let manager = NpmManager::new(None, Some(cache_dir));
    let key = cache::get_cache_key("axios", "1.6.0");
    manager.cache_registry.write().unwrap().insert(
        key,
        CacheEntry {
            package_name: "axios".to_string(),
            version: "1.6.0".to_string(),
            cache_path: outside.clone(),
            used_by: HashSet::new(),
            last_accessed: "2026-08-22T00:00:00Z".to_string(),
            size_bytes: 0,
        },
    );

    assert!(manager.cleanup_all_unused().is_err());
    assert!(outside.exists());
}

#[test]
fn test_cache_link_rejects_target_outside_node_modules_root() {
    use std::collections::HashSet;
    use std::sync::{Arc, RwLock};

    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let cache_root = std::fs::canonicalize(&cache_dir).unwrap();
    let key = cache::get_cache_key("axios", "1.6.0");
    let cache_path = cache_root.join(&key);
    std::fs::create_dir(&cache_path).unwrap();
    std::fs::write(
        cache_path.join("index.js"),
        "module.exports = {};".as_bytes(),
    )
    .unwrap();

    let registry = Arc::new(RwLock::new(std::collections::HashMap::new()));
    registry.write().unwrap().insert(
        key,
        CacheEntry {
            package_name: "axios".to_string(),
            version: "1.6.0".to_string(),
            cache_path,
            used_by: HashSet::new(),
            last_accessed: "2026-08-22T00:00:00Z".to_string(),
            size_bytes: 20,
        },
    );
    let stats = Arc::new(RwLock::new(CacheStatistics::default()));
    let node_modules = temp_dir.path().join("plugin").join("node_modules");
    let outside = temp_dir.path().join("outside").join("axios");

    assert!(cache::link_from_cache(
        cache::CacheLinkContext {
            cache_dir: &Some(cache_dir),
            cache_registry: &registry,
            cache_stats: &stats,
        },
        "axios",
        "1.6.0",
        "test-plugin",
        &node_modules,
        &outside,
    )
    .is_err());
    assert!(!outside.exists());
}

#[test]
fn test_cache_entry_creation() {
    use std::collections::HashSet;
    let mut used_by = HashSet::new();
    used_by.insert("plugin1".to_string());

    let entry = CacheEntry {
        package_name: "axios".to_string(),
        version: "1.6.0".to_string(),
        cache_path: PathBuf::from("/cache/axios@1.6.0"),
        used_by,
        last_accessed: "2024-01-01T00:00:00Z".to_string(),
        size_bytes: 1024,
    };

    assert_eq!(entry.package_name, "axios");
    assert_eq!(entry.version, "1.6.0");
    assert_eq!(entry.used_by.len(), 1);
    assert!(entry.used_by.contains("plugin1"));
}

#[test]
fn test_cache_statistics_default() {
    let stats = CacheStatistics::default();
    assert_eq!(stats.total_packages, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

#[test]
fn test_cache_statistics_hit_rate() {
    let stats = CacheStatistics {
        total_packages: 5,
        total_size_bytes: 5120,
        cache_hits: 8,
        cache_misses: 2,
        hit_rate: 0.8,
        plugins_count: 3,
        last_cleanup: None,
    };
    assert_eq!(stats.cache_hits, 8);
    assert_eq!(stats.cache_misses, 2);
    assert_eq!(stats.hit_rate, 0.8);
}

#[test]
fn test_get_cache_statistics() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    let manager = NpmManager::new(None, Some(cache_dir));

    let stats = manager.get_cache_statistics();
    assert_eq!(stats.total_packages, 0);
    assert_eq!(stats.cache_hits, 0);
    assert_eq!(stats.cache_misses, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

#[test]
fn test_clear_cache() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let manager = NpmManager::new(None, Some(cache_dir.clone()));
    let result = manager.clear_cache();
    assert!(result.is_ok());

    let stats = manager.get_cache_statistics();
    assert_eq!(stats.total_packages, 0);
    assert!(stats.last_cleanup.is_some());
}

#[test]
fn test_clear_cache_rejects_non_directory_cache_root() {
    let temp_dir = TempDir::new().unwrap();
    let cache_path = temp_dir.path().join("cache");
    std::fs::write(&cache_path, "not a directory").unwrap();

    let manager = NpmManager::new(None, Some(cache_path));
    assert!(manager.clear_cache().is_err());
}

#[test]
fn test_cleanup_all_unused() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().join("cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let manager = NpmManager::new(None, Some(cache_dir));
    let removed = manager.cleanup_all_unused().unwrap();
    assert_eq!(removed, 0);
}

#[test]
fn test_cache_hit_rate_update() {
    let mut stats = CacheStatistics {
        cache_hits: 8,
        cache_misses: 2,
        ..Default::default()
    };
    cache::update_hit_rate(&mut stats);
    assert_eq!(stats.hit_rate, 0.8);

    let mut stats2 = CacheStatistics::default();
    cache::update_hit_rate(&mut stats2);
    assert_eq!(stats2.hit_rate, 0.0);
}

#[test]
fn command_output_reader_drains_but_bounds_retained_bytes() {
    let input = vec![b'x'; MAX_NPM_COMMAND_OUTPUT_BYTES + 1024];
    let output = read_bounded_command_output(std::io::Cursor::new(input));

    assert_eq!(output.len(), MAX_NPM_COMMAND_OUTPUT_BYTES);
    assert!(output.ends_with(NPM_OUTPUT_TRUNCATED_MARKER));
}
