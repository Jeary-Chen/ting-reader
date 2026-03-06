use crate::core::error::{Result, TingError};
use crate::db::manager::DatabaseManager;
use crate::db::models::PluginRecord;
use crate::db::repository::base::Repository;
use async_trait::async_trait;
use rusqlite::OptionalExtension;
use std::sync::Arc;

/// Repository for PluginRecord entities
pub struct PluginRepository {
    db: Arc<DatabaseManager>,
}

impl PluginRepository {
    /// Create a new PluginRepository
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self { db }
    }
    
    /// Find plugins by type
    pub async fn find_by_type(&self, plugin_type: &str) -> Result<Vec<PluginRecord>> {
        let plugin_type = plugin_type.to_string();
        self.db.execute(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, version, plugin_type, description, author, enabled, config, \
                 created_at, updated_at FROM plugin_registry WHERE plugin_type = ? ORDER BY name"
            ).map_err(TingError::DatabaseError)?;
            
            let plugins = stmt.query_map([&plugin_type], |row| {
                Ok(PluginRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    plugin_type: row.get(3)?,
                    description: row.get(4)?,
                    author: row.get(5)?,
                    enabled: row.get(6)?,
                    config: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            }).map_err(TingError::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(TingError::DatabaseError)?;
            
            Ok(plugins)
        }).await
    }
    
    /// Find enabled plugins
    pub async fn find_enabled(&self) -> Result<Vec<PluginRecord>> {
        self.db.execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, version, plugin_type, description, author, enabled, config, \
                 created_at, updated_at FROM plugin_registry WHERE enabled = 1 ORDER BY name"
            ).map_err(TingError::DatabaseError)?;
            
            let plugins = stmt.query_map([], |row| {
                Ok(PluginRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    plugin_type: row.get(3)?,
                    description: row.get(4)?,
                    author: row.get(5)?,
                    enabled: row.get(6)?,
                    config: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            }).map_err(TingError::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(TingError::DatabaseError)?;
            
            Ok(plugins)
        }).await
    }
}

#[async_trait]
impl Repository<PluginRecord> for PluginRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<PluginRecord>> {
        let id = id.to_string();
        self.db.execute(move |conn| {
            conn.query_row(
                "SELECT id, name, version, plugin_type, description, author, enabled, config, \
                 created_at, updated_at FROM plugin_registry WHERE id = ?",
                [&id],
                |row| {
                    Ok(PluginRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        version: row.get(2)?,
                        plugin_type: row.get(3)?,
                        description: row.get(4)?,
                        author: row.get(5)?,
                        enabled: row.get(6)?,
                        config: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                }
            ).optional()
            .map_err(TingError::DatabaseError)
        }).await
    }
    
    async fn find_all(&self) -> Result<Vec<PluginRecord>> {
        self.db.execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, version, plugin_type, description, author, enabled, config, \
                 created_at, updated_at FROM plugin_registry ORDER BY name"
            ).map_err(TingError::DatabaseError)?;
            
            let plugins = stmt.query_map([], |row| {
                Ok(PluginRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    plugin_type: row.get(3)?,
                    description: row.get(4)?,
                    author: row.get(5)?,
                    enabled: row.get(6)?,
                    config: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            }).map_err(TingError::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(TingError::DatabaseError)?;
            
            Ok(plugins)
        }).await
    }
    
    async fn create(&self, plugin: &PluginRecord) -> Result<()> {
        let plugin = plugin.clone();
        self.db.execute(move |conn| {
            conn.execute(
                "INSERT INTO plugin_registry (id, name, version, plugin_type, description, \
                 author, enabled, config) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    &plugin.id,
                    &plugin.name,
                    &plugin.version,
                    &plugin.plugin_type,
                    &plugin.description,
                    &plugin.author,
                    plugin.enabled,
                    &plugin.config,
                ],
            ).map_err(TingError::DatabaseError)?;
            Ok(())
        }).await
    }
    
    async fn update(&self, plugin: &PluginRecord) -> Result<()> {
        let plugin = plugin.clone();
        self.db.execute(move |conn| {
            conn.execute(
                "UPDATE plugin_registry SET name = ?, version = ?, plugin_type = ?, \
                 description = ?, author = ?, enabled = ?, config = ?, updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now') \
                 WHERE id = ?",
                rusqlite::params![
                    &plugin.name,
                    &plugin.version,
                    &plugin.plugin_type,
                    &plugin.description,
                    &plugin.author,
                    plugin.enabled,
                    &plugin.config,
                    &plugin.id,
                ],
            ).map_err(TingError::DatabaseError)?;
            Ok(())
        }).await
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db.execute(move |conn| {
            conn.execute("DELETE FROM plugin_registry WHERE id = ?", [&id])
                .map_err(TingError::DatabaseError)?;
            Ok(())
        }).await
    }
}
