//! npm dependency cache management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::info;

#[cfg(test)]
use std::path::Component;
#[cfg(test)]
use tracing::debug;

use sha2::{Digest, Sha256};

#[cfg(test)]
use crate::plugin::fs_utils;

/// Cache entry for a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub package_name: String,
    pub version: String,
    pub cache_path: PathBuf,
    pub used_by: HashSet<String>,
    pub last_accessed: String,
    pub size_bytes: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub total_packages: usize,
    pub total_size_bytes: u64,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub hit_rate: f64,
    pub plugins_count: usize,
    pub last_cleanup: Option<String>,
}

impl Default for CacheStatistics {
    fn default() -> Self {
        Self {
            total_packages: 0,
            total_size_bytes: 0,
            cache_hits: 0,
            cache_misses: 0,
            hit_rate: 0.0,
            plugins_count: 0,
            last_cleanup: None,
        }
    }
}

/// Cache registry (package_name@version -> CacheEntry)
pub type CacheRegistry = Arc<RwLock<HashMap<String, CacheEntry>>>;
pub type CacheStatsLock = Arc<RwLock<CacheStatistics>>;

pub fn get_cache_key(package_name: &str, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ting-reader-npm-cache-v1\0");
    hasher.update(package_name.as_bytes());
    hasher.update(b"\0");
    hasher.update(version.as_bytes());
    format!("npm-v1-{:x}", hasher.finalize())
}

fn canonical_cache_root(cache_dir: &Path) -> Result<PathBuf> {
    if std::fs::symlink_metadata(cache_dir).is_ok() {
        let metadata = std::fs::symlink_metadata(cache_dir)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            anyhow::bail!("npm cache root must be a real directory");
        }
    } else {
        std::fs::create_dir_all(cache_dir).context("Failed to create cache directory")?;
    }
    std::fs::canonicalize(cache_dir).context("Failed to canonicalize cache directory")
}

#[cfg(test)]
fn cache_entry_path(cache_root: &Path, cache_key: &str) -> Result<PathBuf> {
    if !cache_key.starts_with("npm-v1-")
        || cache_key.len() != 71
        || !cache_key[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        anyhow::bail!("Invalid npm cache key");
    }
    Ok(cache_root.join(cache_key))
}

fn ensure_existing_cache_entry(cache_root: &Path, cache_path: &Path) -> Result<PathBuf> {
    if cache_path.parent() != Some(cache_root) {
        anyhow::bail!(
            "npm cache path escapes cache root: {}",
            cache_path.display()
        );
    }
    let metadata = std::fs::symlink_metadata(cache_path)
        .with_context(|| format!("Failed to inspect npm cache path {}", cache_path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        anyhow::bail!("npm cache entry must be a real directory");
    }
    let canonical = std::fs::canonicalize(cache_path).with_context(|| {
        format!(
            "Failed to canonicalize npm cache path {}",
            cache_path.display()
        )
    })?;
    if canonical.parent() != Some(cache_root) {
        anyhow::bail!("npm cache entry escapes cache root");
    }
    Ok(canonical)
}

fn remove_cache_entry(cache_root: &Path, cache_path: &Path) -> Result<()> {
    let canonical = ensure_existing_cache_entry(cache_root, cache_path)?;
    std::fs::remove_dir_all(&canonical)
        .with_context(|| format!("Failed to remove cached package at {}", canonical.display()))
}

#[cfg(test)]
fn copy_directory_without_symlinks(source: &Path, destination: &Path) -> Result<()> {
    let source_metadata = std::fs::symlink_metadata(source)
        .with_context(|| format!("Failed to inspect npm dependency {}", source.display()))?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_dir() {
        anyhow::bail!("npm dependency cache source must be a real directory");
    }

    for entry in std::fs::read_dir(source)
        .with_context(|| format!("Failed to read npm dependency {}", source.display()))?
    {
        let entry = entry.context("Failed to inspect npm dependency entry")?;
        let metadata = entry.metadata()?;
        if entry.file_type()?.is_symlink() {
            anyhow::bail!(
                "npm dependency contains unsupported symbolic link: {}",
                entry.path().display()
            );
        }
        let target = destination.join(entry.file_name());
        if metadata.is_dir() {
            std::fs::create_dir(&target)?;
            copy_directory_without_symlinks(&entry.path(), &target)?;
        } else if metadata.is_file() {
            std::fs::copy(entry.path(), target)?;
        } else {
            anyhow::bail!("npm dependency contains unsupported filesystem entry");
        }
    }
    Ok(())
}

#[cfg(test)]
fn prepare_copy_target(target_root: &Path, target_path: &Path) -> Result<PathBuf> {
    let relative = target_path
        .strip_prefix(target_root)
        .context("npm dependency target escapes node_modules root")?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        anyhow::bail!("Invalid npm dependency target path");
    }

    if !target_root.exists() {
        std::fs::create_dir_all(target_root).context("Failed to create node_modules root")?;
    }
    let root_metadata = std::fs::symlink_metadata(target_root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        anyhow::bail!("node_modules root must be a real directory");
    }
    let canonical_root = std::fs::canonicalize(target_root)?;

    let mut current = canonical_root.clone();
    let mut components = relative.components().peekable();
    while let Some(Component::Normal(component)) = components.next() {
        if components.peek().is_none() {
            let target = current.join(component);
            if std::fs::symlink_metadata(&target).is_ok() {
                anyhow::bail!("Refusing to overwrite existing npm dependency target");
            }
            return Ok(target);
        }

        current.push(component);
        if !current.exists() {
            std::fs::create_dir(&current)?;
        }
        let metadata = std::fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            anyhow::bail!("npm dependency target parent must be a real directory");
        }
        let canonical = std::fs::canonicalize(&current)?;
        if !canonical.starts_with(&canonical_root) {
            anyhow::bail!("npm dependency target escapes node_modules root");
        }
        current = canonical;
    }

    anyhow::bail!("Invalid npm dependency target path")
}

pub fn is_cached(
    cache_dir: &Option<PathBuf>,
    cache_registry: &CacheRegistry,
    package_name: &str,
    version: &str,
) -> bool {
    if cache_dir.is_none() {
        return false;
    }
    let cache_key = get_cache_key(package_name, version);
    let registry = cache_registry.read().unwrap();
    registry.contains_key(&cache_key)
}

#[cfg(test)]
pub fn update_hit_rate(stats: &mut CacheStatistics) {
    let total = stats.cache_hits + stats.cache_misses;
    stats.hit_rate = if total > 0 {
        stats.cache_hits as f64 / total as f64
    } else {
        0.0
    };
}

#[cfg(test)]
pub fn add_to_cache(
    cache_dir: &Option<PathBuf>,
    cache_registry: &CacheRegistry,
    cache_stats: &CacheStatsLock,
    package_name: &str,
    version: &str,
    plugin_name: &str,
    source_path: &Path,
) -> Result<()> {
    let cache_dir = match cache_dir {
        Some(dir) => dir,
        None => {
            debug!("Cache directory not configured, skipping cache");
            return Ok(());
        }
    };

    let cache_root = canonical_cache_root(cache_dir)?;
    let cache_key = get_cache_key(package_name, version);
    let cache_path = cache_entry_path(&cache_root, &cache_key)?;
    let registered = cache_registry.read().unwrap().contains_key(&cache_key);
    if cache_path.exists() && !registered {
        remove_cache_entry(&cache_root, &cache_path)?;
    }

    if !cache_path.exists() {
        info!("Caching dependency: {}", cache_key);
        std::fs::create_dir(&cache_path).context("Failed to create npm cache entry")?;
        let cache_path = ensure_existing_cache_entry(&cache_root, &cache_path)?;
        if let Err(error) = copy_directory_without_symlinks(source_path, &cache_path) {
            let _ = remove_cache_entry(&cache_root, &cache_path);
            return Err(error).context("Failed to copy dependency to cache");
        }
        let size_bytes =
            fs_utils::calculate_dir_size(&cache_path).context("Failed to calculate cache size")?;

        let mut used_by = HashSet::new();
        used_by.insert(plugin_name.to_string());

        let entry = CacheEntry {
            package_name: package_name.to_string(),
            version: version.to_string(),
            cache_path: cache_path.clone(),
            used_by,
            last_accessed: chrono::Utc::now().to_rfc3339(),
            size_bytes,
        };

        let mut registry = cache_registry.write().unwrap();
        registry.insert(cache_key.clone(), entry);

        let mut stats = cache_stats.write().unwrap();
        stats.total_packages += 1;
        stats.total_size_bytes += size_bytes;
        stats.cache_misses += 1;
        update_hit_rate(&mut stats);

        info!("Dependency cached successfully: {}", cache_key);
    } else {
        ensure_existing_cache_entry(&cache_root, &cache_path)?;
        let mut registry = cache_registry.write().unwrap();
        if let Some(entry) = registry.get_mut(&cache_key) {
            entry.used_by.insert(plugin_name.to_string());
            entry.last_accessed = chrono::Utc::now().to_rfc3339();
            let mut stats = cache_stats.write().unwrap();
            stats.cache_hits += 1;
            update_hit_rate(&mut stats);
            info!("Using cached dependency: {}", cache_key);
        }
    }

    Ok(())
}

#[cfg(test)]
pub(super) struct CacheLinkContext<'a> {
    pub(super) cache_dir: &'a Option<PathBuf>,
    pub(super) cache_registry: &'a CacheRegistry,
    pub(super) cache_stats: &'a CacheStatsLock,
}

#[cfg(test)]
pub(super) fn link_from_cache(
    context: CacheLinkContext<'_>,
    package_name: &str,
    version: &str,
    plugin_name: &str,
    target_root: &Path,
    target_path: &Path,
) -> Result<()> {
    let cache_dir = context
        .cache_dir
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Cache directory not configured"))?;
    let cache_root = canonical_cache_root(cache_dir)?;
    let cache_key = get_cache_key(package_name, version);
    let registry = context.cache_registry.read().unwrap();
    let entry = registry
        .get(&cache_key)
        .ok_or_else(|| anyhow::anyhow!("Dependency not found in cache: {}", cache_key))?;
    let cache_path = ensure_existing_cache_entry(&cache_root, &entry.cache_path)?;

    info!(
        "Linking cached dependency {} to {}",
        cache_key,
        target_path.display()
    );

    let target_path = prepare_copy_target(target_root, target_path)?;

    fs_utils::copy_dir_recursive(&cache_path, &target_path).context("Failed to copy from cache")?;

    drop(registry);
    let mut registry = context.cache_registry.write().unwrap();
    if let Some(entry) = registry.get_mut(&cache_key) {
        entry.used_by.insert(plugin_name.to_string());
        entry.last_accessed = chrono::Utc::now().to_rfc3339();
    }

    let mut stats = context.cache_stats.write().unwrap();
    stats.cache_hits += 1;
    update_hit_rate(&mut stats);

    Ok(())
}

pub fn get_cache_statistics(
    cache_registry: &CacheRegistry,
    cache_stats: &CacheStatsLock,
) -> CacheStatistics {
    let stats = cache_stats.read().unwrap();
    let registry = cache_registry.read().unwrap();
    let mut all_plugins = HashSet::new();
    for entry in registry.values() {
        all_plugins.extend(entry.used_by.iter().cloned());
    }
    CacheStatistics {
        total_packages: stats.total_packages,
        total_size_bytes: stats.total_size_bytes,
        cache_hits: stats.cache_hits,
        cache_misses: stats.cache_misses,
        hit_rate: stats.hit_rate,
        plugins_count: all_plugins.len(),
        last_cleanup: stats.last_cleanup.clone(),
    }
}

pub fn clear_cache(
    cache_dir: &Option<PathBuf>,
    cache_registry: &CacheRegistry,
    cache_stats: &CacheStatsLock,
) -> Result<()> {
    let cache_dir = match cache_dir {
        Some(dir) => dir,
        None => return Ok(()),
    };
    info!("Clearing all cache");

    let cache_root = canonical_cache_root(cache_dir)?;
    for entry in std::fs::read_dir(&cache_root).context("Failed to read cache directory")? {
        let entry = entry.context("Failed to inspect cache directory entry")?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || metadata.is_file() {
            std::fs::remove_file(&path)?;
        } else if metadata.is_dir() {
            remove_cache_entry(&cache_root, &path)?;
        }
    }

    cache_registry.write().unwrap().clear();
    let mut stats = cache_stats.write().unwrap();
    *stats = CacheStatistics {
        last_cleanup: Some(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    };

    info!("Cache cleared successfully");
    Ok(())
}

pub fn cleanup_cache_for_plugin(
    cache_dir: &Option<PathBuf>,
    cache_registry: &CacheRegistry,
    cache_stats: &CacheStatsLock,
    plugin_name: &str,
) -> Result<usize> {
    let cache_dir = match cache_dir {
        Some(cache_dir) => cache_dir,
        None => return Ok(0),
    };
    let cache_root = canonical_cache_root(cache_dir)?;
    info!("Cleaning up cache for plugin: {}", plugin_name);
    let mut removed_count = 0;
    let mut packages_to_remove = Vec::new();

    {
        let mut registry = cache_registry.write().unwrap();
        for (cache_key, entry) in registry.iter_mut() {
            entry.used_by.remove(plugin_name);
            if entry.used_by.is_empty() {
                packages_to_remove.push((
                    cache_key.clone(),
                    entry.cache_path.clone(),
                    entry.size_bytes,
                ));
            }
        }
    }

    for (cache_key, cache_path, size_bytes) in packages_to_remove {
        info!("Removing unused cached package: {}", cache_key);
        if cache_path.exists() {
            remove_cache_entry(&cache_root, &cache_path)?;
        }
        cache_registry.write().unwrap().remove(&cache_key);
        let mut stats = cache_stats.write().unwrap();
        stats.total_packages = stats.total_packages.saturating_sub(1);
        stats.total_size_bytes = stats.total_size_bytes.saturating_sub(size_bytes);
        removed_count += 1;
    }

    if removed_count > 0 {
        cache_stats.write().unwrap().last_cleanup = Some(chrono::Utc::now().to_rfc3339());
        info!("Removed {} unused packages from cache", removed_count);
    }

    Ok(removed_count)
}

pub fn cleanup_all_unused(
    cache_dir: &Option<PathBuf>,
    cache_registry: &CacheRegistry,
    cache_stats: &CacheStatsLock,
) -> Result<usize> {
    let cache_dir = match cache_dir {
        Some(cache_dir) => cache_dir,
        None => return Ok(0),
    };
    let cache_root = canonical_cache_root(cache_dir)?;
    info!("Cleaning up all unused cached packages");
    let mut removed_count = 0;
    let mut packages_to_remove = Vec::new();

    {
        let registry = cache_registry.read().unwrap();
        for (cache_key, entry) in registry.iter() {
            if entry.used_by.is_empty() {
                packages_to_remove.push((
                    cache_key.clone(),
                    entry.cache_path.clone(),
                    entry.size_bytes,
                ));
            }
        }
    }

    for (cache_key, cache_path, size_bytes) in packages_to_remove {
        info!("Removing unused cached package: {}", cache_key);
        if cache_path.exists() {
            remove_cache_entry(&cache_root, &cache_path)?;
        }
        cache_registry.write().unwrap().remove(&cache_key);
        let mut stats = cache_stats.write().unwrap();
        stats.total_packages = stats.total_packages.saturating_sub(1);
        stats.total_size_bytes = stats.total_size_bytes.saturating_sub(size_bytes);
        removed_count += 1;
    }

    if removed_count > 0 {
        cache_stats.write().unwrap().last_cleanup = Some(chrono::Utc::now().to_rfc3339());
        info!("Removed {} unused packages from cache", removed_count);
    }

    Ok(removed_count)
}
