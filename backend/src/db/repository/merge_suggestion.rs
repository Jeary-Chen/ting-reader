use crate::core::error::{Result, TingError};
use crate::db::manager::DatabaseManager;
use crate::db::models::MergeSuggestion;
use crate::db::repository::base::Repository;
use async_trait::async_trait;
use rusqlite::OptionalExtension;
use std::sync::Arc;

/// Repository for MergeSuggestion entities
pub struct MergeSuggestionRepository {
    db: Arc<DatabaseManager>,
}

impl MergeSuggestionRepository {
    /// Create a new MergeSuggestionRepository
    pub fn new(db: Arc<DatabaseManager>) -> Self {
        Self { db }
    }
    
    /// Find suggestions by status
    pub async fn find_by_status(&self, status: &str) -> Result<Vec<MergeSuggestion>> {
        let status = status.to_string();
        self.db.execute(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, book_a_id, book_b_id, score, reason, status, created_at \
                 FROM merge_suggestions WHERE status = ? ORDER BY score DESC"
            ).map_err(TingError::DatabaseError)?;
            
            let suggestions = stmt.query_map([&status], |row| {
                Ok(MergeSuggestion {
                    id: row.get(0)?,
                    book_a_id: row.get(1)?,
                    book_b_id: row.get(2)?,
                    score: row.get(3)?,
                    reason: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                })
            }).map_err(TingError::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(TingError::DatabaseError)?;
            
            Ok(suggestions)
        }).await
    }
    
    /// Check if suggestion exists
    pub async fn exists(&self, book_a_id: &str, book_b_id: &str) -> Result<bool> {
        let book_a_id = book_a_id.to_string();
        let book_b_id = book_b_id.to_string();
        self.db.execute(move |conn| {
            // Check both directions (a,b) or (b,a)
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM merge_suggestions WHERE \
                 (book_a_id = ? AND book_b_id = ?) OR (book_a_id = ? AND book_b_id = ?)",
                rusqlite::params![&book_a_id, &book_b_id, &book_b_id, &book_a_id],
                |row| row.get(0)
            ).map_err(TingError::DatabaseError)?;
            Ok(count > 0)
        }).await
    }
}

#[async_trait]
impl Repository<MergeSuggestion> for MergeSuggestionRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<MergeSuggestion>> {
        let id = id.to_string();
        self.db.execute(move |conn| {
            conn.query_row(
                "SELECT id, book_a_id, book_b_id, score, reason, status, created_at \
                 FROM merge_suggestions WHERE id = ?",
                [&id],
                |row| {
                    Ok(MergeSuggestion {
                        id: row.get(0)?,
                        book_a_id: row.get(1)?,
                        book_b_id: row.get(2)?,
                        score: row.get(3)?,
                        reason: row.get(4)?,
                        status: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                }
            ).optional()
            .map_err(TingError::DatabaseError)
        }).await
    }
    
    async fn find_all(&self) -> Result<Vec<MergeSuggestion>> {
        self.db.execute(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, book_a_id, book_b_id, score, reason, status, created_at \
                 FROM merge_suggestions ORDER BY created_at DESC"
            ).map_err(TingError::DatabaseError)?;
            
            let suggestions = stmt.query_map([], |row| {
                Ok(MergeSuggestion {
                    id: row.get(0)?,
                    book_a_id: row.get(1)?,
                    book_b_id: row.get(2)?,
                    score: row.get(3)?,
                    reason: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                })
            }).map_err(TingError::DatabaseError)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(TingError::DatabaseError)?;
            
            Ok(suggestions)
        }).await
    }
    
    async fn create(&self, suggestion: &MergeSuggestion) -> Result<()> {
        let suggestion = suggestion.clone();
        self.db.execute(move |conn| {
            conn.execute(
                "INSERT INTO merge_suggestions (id, book_a_id, book_b_id, score, reason, status, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    &suggestion.id,
                    &suggestion.book_a_id,
                    &suggestion.book_b_id,
                    suggestion.score,
                    &suggestion.reason,
                    &suggestion.status,
                    &suggestion.created_at,
                ],
            ).map_err(TingError::DatabaseError)?;
            Ok(())
        }).await
    }
    
    async fn update(&self, suggestion: &MergeSuggestion) -> Result<()> {
        let suggestion = suggestion.clone();
        self.db.execute(move |conn| {
            conn.execute(
                "UPDATE merge_suggestions SET book_a_id = ?, book_b_id = ?, score = ?, \
                 reason = ?, status = ? WHERE id = ?",
                rusqlite::params![
                    &suggestion.book_a_id,
                    &suggestion.book_b_id,
                    suggestion.score,
                    &suggestion.reason,
                    &suggestion.status,
                    &suggestion.id,
                ],
            ).map_err(TingError::DatabaseError)?;
            Ok(())
        }).await
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let id = id.to_string();
        self.db.execute(move |conn| {
            conn.execute("DELETE FROM merge_suggestions WHERE id = ?", [&id])
                .map_err(TingError::DatabaseError)?;
            Ok(())
        }).await
    }
}
