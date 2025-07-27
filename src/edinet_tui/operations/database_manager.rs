//! Database manager for handling database operations

use anyhow::Result;
use chrono::NaiveDate;
use tokio::task::JoinHandle;

use crate::{
    config::Config,
    edinet,
    storage,
};

/// Database operation types
#[derive(Debug, Clone)]
pub enum DatabaseOperation {
    ShowStats,
    UpdateIndex,
    BuildIndex { from: NaiveDate, to: NaiveDate },
    ClearIndex,
    LoadStaticData { csv_path: String },
}

/// Database operation progress
#[derive(Debug, Clone)]
pub struct DatabaseProgress {
    pub operation: DatabaseOperation,
    pub status: DatabaseStatus,
    pub message: String,
    pub progress_percent: Option<f32>,
    pub started_at: chrono::DateTime<chrono::Local>,
    pub completed_at: Option<chrono::DateTime<chrono::Local>>,
    pub result: Option<String>,
}

/// Database operation status
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl DatabaseProgress {
    pub fn new(operation: DatabaseOperation) -> Self {
        Self {
            operation,
            status: DatabaseStatus::Queued,
            message: "Queued".to_string(),
            progress_percent: None,
            started_at: chrono::Local::now(),
            completed_at: None,
            result: None,
        }
    }

    pub fn set_in_progress(&mut self, message: String) {
        self.status = DatabaseStatus::InProgress;
        self.message = message;
    }

    pub fn set_completed(&mut self, message: String, result: Option<String>) {
        self.status = DatabaseStatus::Completed;
        self.message = message;
        self.result = result;
        self.completed_at = Some(chrono::Local::now());
        self.progress_percent = Some(100.0);
    }

    pub fn set_failed(&mut self, error: String) {
        self.status = DatabaseStatus::Failed;
        self.message = format!("Failed: {}", error);
        self.completed_at = Some(chrono::Local::now());
    }

    pub fn set_cancelled(&mut self) {
        self.status = DatabaseStatus::Cancelled;
        self.message = "Cancelled by user".to_string();
        self.completed_at = Some(chrono::Local::now());
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, DatabaseStatus::Queued | DatabaseStatus::InProgress)
    }
}

/// Database manager handles database operations
pub struct DatabaseManager {
    config: Config,
    current_operation: Option<DatabaseProgress>,
    operation_handle: Option<JoinHandle<Result<String>>>,
}

impl DatabaseManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            current_operation: None,
            operation_handle: None,
        }
    }

    /// Start a database operation
    pub async fn start_operation(&mut self, operation: DatabaseOperation) -> Result<()> {
        // Check if another operation is running
        if self.is_operation_in_progress() {
            return Err(anyhow::anyhow!("Another database operation is already in progress"));
        }

        let mut progress = DatabaseProgress::new(operation.clone());
        progress.set_in_progress("Starting operation...".to_string());
        
        self.current_operation = Some(progress);

        // Start the actual operation based on type
        let config = self.config.clone();
        let handle = match operation {
            DatabaseOperation::ShowStats => {
                tokio::spawn(async move {
                    Self::show_stats_operation(config).await
                })
            }
            DatabaseOperation::UpdateIndex => {
                tokio::spawn(async move {
                    Self::update_index_operation(config).await
                })
            }
            DatabaseOperation::BuildIndex { from, to } => {
                tokio::spawn(async move {
                    Self::build_index_operation(config, from, to).await
                })
            }
            DatabaseOperation::ClearIndex => {
                tokio::spawn(async move {
                    Self::clear_index_operation(config).await
                })
            }
            DatabaseOperation::LoadStaticData { csv_path } => {
                tokio::spawn(async move {
                    Self::load_static_data_operation(config, csv_path).await
                })
            }
        };

        self.operation_handle = Some(handle);
        Ok(())
    }

    /// Cancel current operation
    pub fn cancel_operation(&mut self) {
        if let Some(handle) = self.operation_handle.take() {
            handle.abort();
        }

        if let Some(progress) = &mut self.current_operation {
            progress.set_cancelled();
        }
    }

    /// Check and update operation progress
    pub async fn update_progress(&mut self) -> Result<()> {
        if let Some(handle) = &mut self.operation_handle {
            if handle.is_finished() {
                let result = match handle.await {
                    Ok(operation_result) => operation_result,
                    Err(e) => {
                        if let Some(progress) = &mut self.current_operation {
                            progress.set_failed(format!("Operation task failed: {}", e));
                        }
                        self.operation_handle = None;
                        return Ok(());
                    }
                };

                // Update progress based on result
                if let Some(progress) = &mut self.current_operation {
                    match result {
                        Ok(message) => {
                            progress.set_completed("Operation completed successfully".to_string(), Some(message));
                        }
                        Err(e) => {
                            progress.set_failed(e.to_string());
                        }
                    }
                }

                self.operation_handle = None;
            }
        }

        Ok(())
    }

    /// Check if operation is in progress
    pub fn is_operation_in_progress(&self) -> bool {
        self.current_operation.as_ref()
            .map_or(false, |p| p.is_active())
    }

    /// Get current operation progress
    pub fn get_operation_progress(&self) -> Option<&DatabaseProgress> {
        self.current_operation.as_ref()
    }

    /// Get operation status message
    pub fn get_operation_status(&self) -> Option<String> {
        self.current_operation.as_ref()
            .map(|p| p.message.clone())
    }

    /// Clear completed operation from history
    pub fn clear_completed_operation(&mut self) {
        if let Some(progress) = &self.current_operation {
            if !progress.is_active() {
                self.current_operation = None;
            }
        }
    }

    // Operation implementations

    async fn show_stats_operation(config: Config) -> Result<String> {
        let db_path = config.database_path_str();
        
        // Get document count for EDINET source
        let doc_count = storage::count_documents_by_source(&crate::models::Source::Edinet, db_path).await
            .map_err(|e| anyhow::anyhow!("Failed to count documents: {}", e))?;

        // Get date range
        let date_range = storage::get_date_range_for_source(&crate::models::Source::Edinet, db_path).await
            .map_err(|e| anyhow::anyhow!("Failed to get date range: {}", e))?;

        Ok(format!(
            "Database Statistics:\n\
            Documents: {}\n\
            Date Range: {} to {}\n\
            Database Path: {}",
            doc_count, date_range.0, date_range.1, db_path
        ))
    }

    async fn update_index_operation(config: Config) -> Result<String> {
        // This would use the edinet indexer - simplified for now
        // let mut indexer = edinet::indexer::EdinetIndexer::new(config)?;
        // let result = indexer.update_index().await?;
        
        // Placeholder implementation
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        Ok("Index update completed (placeholder implementation)".to_string())
    }

    async fn build_index_operation(config: Config, from: NaiveDate, to: NaiveDate) -> Result<String> {
        // This would use the edinet indexer - simplified for now
        // let mut indexer = edinet::indexer::EdinetIndexer::new(config)?;
        // let result = indexer.build_index(from, to).await?;
        
        // Placeholder implementation
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        Ok(format!(
            "Index build completed for {} to {} (placeholder implementation)",
            from, to
        ))
    }

    async fn clear_index_operation(config: Config) -> Result<String> {
        // This would clear the documents table - simplified for now
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        Ok("Index cleared successfully (placeholder implementation)".to_string())
    }

    async fn load_static_data_operation(config: Config, csv_path: String) -> Result<String> {
        let count = storage::load_edinet_static_data(config.database_path_str(), &csv_path).await
            .map_err(|e| anyhow::anyhow!("Failed to load static data: {}", e))?;

        Ok(format!("Loaded {} static entries from {}", count, csv_path))
    }

    /// Quick database health check
    pub async fn health_check(&self) -> Result<DatabaseHealthStatus> {
        let db_path = self.config.database_path_str();
        
        // Check if database file exists
        if !std::path::Path::new(db_path).exists() {
            return Ok(DatabaseHealthStatus {
                status: "Not Found".to_string(),
                documents_count: 0,
                static_entries_count: 0,
                last_updated: None,
                issues: vec!["Database file does not exist".to_string()],
            });
        }

        let mut issues = Vec::new();

        // Check document count
        let documents_count = storage::count_documents_by_source(&crate::models::Source::Edinet, db_path).await.unwrap_or_else(|e| {
            issues.push(format!("Cannot count documents: {}", e));
            0
        }) as usize;

        // Check static entries count - simplified for now
        let static_entries_count = 0; // Would implement proper count function

        // Determine overall status
        let status = if issues.is_empty() {
            if documents_count > 0 && static_entries_count > 0 {
                "Healthy".to_string()
            } else if static_entries_count > 0 {
                "Ready".to_string()
            } else {
                "Empty".to_string()
            }
        } else {
            "Issues".to_string()
        };

        Ok(DatabaseHealthStatus {
            status,
            documents_count,
            static_entries_count,
            last_updated: None, // Could implement this by tracking in metadata table
            issues,
        })
    }
}

/// Database health status
#[derive(Debug, Clone)]
pub struct DatabaseHealthStatus {
    pub status: String,
    pub documents_count: usize,
    pub static_entries_count: usize,
    pub last_updated: Option<chrono::DateTime<chrono::Local>>,
    pub issues: Vec<String>,
}

impl DatabaseHealthStatus {
    pub fn is_healthy(&self) -> bool {
        self.issues.is_empty() && self.status == "Healthy"
    }

    pub fn summary(&self) -> String {
        format!(
            "Status: {} | Documents: {} | Static: {} | Issues: {}",
            self.status,
            self.documents_count,
            self.static_entries_count,
            self.issues.len()
        )
    }
}