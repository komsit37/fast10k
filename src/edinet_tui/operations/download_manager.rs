//! Download manager for handling document downloads

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::task::JoinHandle;

use crate::{
    config::Config,
    models::{Document, DownloadRequest, DocumentFormat, Source},
    downloader,
};

/// Download progress tracking
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub document_id: String,
    pub ticker: String,
    pub status: DownloadStatus,
    pub message: String,
    pub progress_percent: Option<f32>,
    pub started_at: chrono::DateTime<chrono::Local>,
    pub completed_at: Option<chrono::DateTime<chrono::Local>>,
}

/// Download status states
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadProgress {
    pub fn new(document_id: String, ticker: String) -> Self {
        Self {
            document_id,
            ticker,
            status: DownloadStatus::Queued,
            message: "Queued for download".to_string(),
            progress_percent: None,
            started_at: chrono::Local::now(),
            completed_at: None,
        }
    }

    pub fn set_in_progress(&mut self, message: String) {
        self.status = DownloadStatus::InProgress;
        self.message = message;
    }

    pub fn set_completed(&mut self, message: String) {
        self.status = DownloadStatus::Completed;
        self.message = message;
        self.completed_at = Some(chrono::Local::now());
        self.progress_percent = Some(100.0);
    }

    pub fn set_failed(&mut self, error: String) {
        self.status = DownloadStatus::Failed;
        self.message = format!("Failed: {}", error);
        self.completed_at = Some(chrono::Local::now());
    }

    pub fn set_cancelled(&mut self) {
        self.status = DownloadStatus::Cancelled;
        self.message = "Cancelled by user".to_string();
        self.completed_at = Some(chrono::Local::now());
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, DownloadStatus::Queued | DownloadStatus::InProgress)
    }

    pub fn is_completed(&self) -> bool {
        !self.is_active()
    }
}

/// Download manager handles multiple concurrent downloads
pub struct DownloadManager {
    config: Config,
    active_downloads: HashMap<String, DownloadProgress>,
    download_handles: HashMap<String, JoinHandle<Result<usize>>>,
    max_concurrent_downloads: usize,
}

impl DownloadManager {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            active_downloads: HashMap::new(),
            download_handles: HashMap::new(),
            max_concurrent_downloads: 3, // Reasonable default
        }
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_downloads = max;
        self
    }

    /// Start downloading a document
    pub async fn download_document(&mut self, document: &Document) -> Result<String> {
        let document_id = self.get_document_id(document);
        
        // Check if already downloading or completed recently
        if let Some(progress) = self.active_downloads.get(&document_id) {
            if progress.is_active() {
                return Ok(document_id);
            }
        }

        // Check concurrent download limit
        let active_count = self.active_downloads.values()
            .filter(|p| p.is_active())
            .count();
        
        if active_count >= self.max_concurrent_downloads {
            return Err(anyhow::anyhow!("Maximum concurrent downloads ({}) reached", self.max_concurrent_downloads));
        }

        // Create progress tracker
        let mut progress = DownloadProgress::new(document_id.clone(), document.ticker.clone());
        progress.set_in_progress(format!("Starting download for {}", document.ticker));
        
        self.active_downloads.insert(document_id.clone(), progress);

        // Create download request
        let download_request = DownloadRequest {
            source: Source::Edinet,
            ticker: document.ticker.clone(),
            filing_type: Some(document.filing_type.clone()),
            date_from: Some(document.date),
            date_to: Some(document.date),
            limit: 1,
            format: DocumentFormat::Complete,
        };

        // Start async download
        let download_dir = self.config.download_dir_str().to_string();
        let doc_id = document_id.clone();
        
        let handle = tokio::spawn(async move {
            downloader::download_documents(&download_request, &download_dir).await
        });

        self.download_handles.insert(document_id.clone(), handle);

        Ok(document_id)
    }

    /// Cancel a download
    pub fn cancel_download(&mut self, document_id: &str) {
        if let Some(handle) = self.download_handles.remove(document_id) {
            handle.abort();
        }

        if let Some(progress) = self.active_downloads.get_mut(document_id) {
            progress.set_cancelled();
        }
    }

    /// Cancel all active downloads
    pub fn cancel_all_downloads(&mut self) {
        let active_ids: Vec<String> = self.active_downloads.keys()
            .filter(|id| self.active_downloads.get(*id).map_or(false, |p| p.is_active()))
            .cloned()
            .collect();

        for id in active_ids {
            self.cancel_download(&id);
        }
    }

    /// Check and update download progress
    pub async fn update_progress(&mut self) -> Result<()> {
        let mut completed_downloads = Vec::new();

        // Check all active downloads
        for (document_id, handle) in &mut self.download_handles {
            if handle.is_finished() {
                let result = match handle.await {
                    Ok(download_result) => download_result,
                    Err(e) => {
                        // Handle was cancelled or panicked
                        if let Some(progress) = self.active_downloads.get_mut(document_id) {
                            progress.set_failed(format!("Download task failed: {}", e));
                        }
                        completed_downloads.push(document_id.clone());
                        continue;
                    }
                };

                // Update progress based on result
                if let Some(progress) = self.active_downloads.get_mut(document_id) {
                    match result {
                        Ok(count) => {
                            progress.set_completed(format!("Downloaded {} document(s)", count));
                        }
                        Err(e) => {
                            progress.set_failed(e.to_string());
                        }
                    }
                }

                completed_downloads.push(document_id.clone());
            }
        }

        // Clean up completed downloads
        for document_id in completed_downloads {
            self.download_handles.remove(&document_id);
        }

        Ok(())
    }

    /// Get download progress for a document
    pub fn get_download_progress(&self, document_id: &str) -> Option<&DownloadProgress> {
        self.active_downloads.get(document_id)
    }

    /// Get all active downloads
    pub fn get_active_downloads(&self) -> Vec<&DownloadProgress> {
        self.active_downloads.values()
            .filter(|p| p.is_active())
            .collect()
    }

    /// Get all downloads (active and completed)
    pub fn get_all_downloads(&self) -> Vec<&DownloadProgress> {
        self.active_downloads.values().collect()
    }

    /// Check if a document is currently being downloaded
    pub fn is_downloading(&self, document_id: &str) -> bool {
        self.active_downloads.get(document_id)
            .map_or(false, |p| p.is_active())
    }

    /// Check if any downloads are active
    pub fn has_active_downloads(&self) -> bool {
        self.active_downloads.values().any(|p| p.is_active())
    }

    /// Get download statistics
    pub fn get_stats(&self) -> DownloadStats {
        let mut stats = DownloadStats::default();
        
        for progress in self.active_downloads.values() {
            match progress.status {
                DownloadStatus::Queued => stats.queued += 1,
                DownloadStatus::InProgress => stats.in_progress += 1,
                DownloadStatus::Completed => stats.completed += 1,
                DownloadStatus::Failed => stats.failed += 1,
                DownloadStatus::Cancelled => stats.cancelled += 1,
            }
        }

        stats.total = stats.queued + stats.in_progress + stats.completed + stats.failed + stats.cancelled;
        stats
    }

    /// Clear completed downloads from history
    pub fn clear_completed(&mut self) {
        self.active_downloads.retain(|_, progress| progress.is_active());
    }

    /// Check if a document is already downloaded locally
    pub fn is_document_downloaded(&self, document: &Document) -> bool {
        let download_dir = PathBuf::from(self.config.download_dir_str());
        let edinet_dir = download_dir.join("edinet").join(&document.ticker);

        if !edinet_dir.exists() {
            return false;
        }

        // Look for ZIP files that match this document
        let doc_id = self.get_document_id(document);
        
        if let Ok(entries) = std::fs::read_dir(&edinet_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.contains(&doc_id) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Generate a unique document ID for tracking
    fn get_document_id(&self, document: &Document) -> String {
        // Use document metadata if available, otherwise generate from document fields
        document.metadata.get("doc_id")
            .or_else(|| document.metadata.get("document_id"))
            .unwrap_or(&document.id)
            .clone()
    }
}

/// Download statistics
#[derive(Debug, Default)]
pub struct DownloadStats {
    pub total: usize,
    pub queued: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl DownloadStats {
    pub fn success_rate(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.completed as f32 / self.total as f32 * 100.0
        }
    }
}