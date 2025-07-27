//! Content loader for handling document content reading and caching

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::{
    config::Config,
    edinet::reader::{read_edinet_zip, DocumentSection},
    models::Document,
};

/// Content cache entry
#[derive(Debug, Clone)]
pub struct ContentCache {
    pub document_id: String,
    pub sections: Vec<DocumentSection>,
    pub loaded_at: chrono::DateTime<chrono::Local>,
    pub file_path: PathBuf,
}

impl ContentCache {
    pub fn new(document_id: String, sections: Vec<DocumentSection>, file_path: PathBuf) -> Self {
        Self {
            document_id,
            sections,
            loaded_at: chrono::Local::now(),
            file_path,
        }
    }

    /// Check if cache entry is still valid (file hasn't changed)
    pub fn is_valid(&self) -> bool {
        // Check if file still exists and hasn't been modified
        if let Ok(metadata) = std::fs::metadata(&self.file_path) {
            if let Ok(modified) = metadata.modified() {
                let modified_time = chrono::DateTime::<chrono::Local>::from(modified);
                return modified_time <= self.loaded_at;
            }
        }
        false
    }

    /// Get cache age in seconds
    pub fn age_seconds(&self) -> i64 {
        let now = chrono::Local::now();
        now.signed_duration_since(self.loaded_at).num_seconds()
    }
}

/// Content loader manages document content loading and caching
pub struct ContentLoader {
    config: Config,
    cache: HashMap<String, ContentCache>,
    max_cache_size: usize,
    max_cache_age_seconds: i64,
}

impl ContentLoader {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            cache: HashMap::new(),
            max_cache_size: 50, // Keep up to 50 documents in cache
            max_cache_age_seconds: 3600, // 1 hour cache timeout
        }
    }

    pub fn with_cache_settings(mut self, max_size: usize, max_age_seconds: i64) -> Self {
        self.max_cache_size = max_size;
        self.max_cache_age_seconds = max_age_seconds;
        self
    }

    /// Load document content with caching
    pub async fn load_document_content(&mut self, document: &Document) -> Result<Vec<DocumentSection>> {
        let document_id = self.get_document_id(document);

        // Check cache first
        if let Some(cached) = self.cache.get(&document_id) {
            if cached.is_valid() && cached.age_seconds() < self.max_cache_age_seconds {
                return Ok(cached.sections.clone());
            } else {
                // Remove invalid/expired cache entry
                self.cache.remove(&document_id);
            }
        }

        // Load from file
        let sections = self.load_from_file(document).await?;

        // Update cache if we found content
        if !sections.is_empty() {
            self.update_cache(document, sections.clone()).await;
        }

        Ok(sections)
    }

    /// Load content directly from file without caching
    async fn load_from_file(&self, document: &Document) -> Result<Vec<DocumentSection>> {
        let document_id = self.get_document_id(document);
        let download_dir = PathBuf::from(self.config.download_dir_str());
        let edinet_dir = download_dir.join("edinet").join(&document.ticker);

        // Look for the specific ZIP file matching this document's ID
        if let Ok(entries) = std::fs::read_dir(&edinet_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        // Only load files that exactly match the document ID
                        if filename.contains(&document_id) {
                            return read_edinet_zip(
                                path.to_str().unwrap(),
                                usize::MAX, // No limit on sections
                                usize::MAX, // No limit on content length
                            );
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Document content not found locally. Download the document first."))
    }

    /// Update cache with new content
    async fn update_cache(&mut self, document: &Document, sections: Vec<DocumentSection>) {
        let document_id = self.get_document_id(document);
        
        // Find the actual file path for cache validation
        let download_dir = PathBuf::from(self.config.download_dir_str());
        let edinet_dir = download_dir.join("edinet").join(&document.ticker);
        
        if let Ok(entries) = std::fs::read_dir(&edinet_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.contains(&document_id) {
                            let cache_entry = ContentCache::new(document_id.clone(), sections, path);
                            self.cache.insert(document_id, cache_entry);
                            
                            // Clean up cache if needed
                            self.cleanup_cache().await;
                            return;
                        }
                    }
                }
            }
        }
    }

    /// Get cached content if available and valid
    pub fn get_cached_content(&self, document: &Document) -> Option<&Vec<DocumentSection>> {
        let document_id = self.get_document_id(document);
        
        if let Some(cached) = self.cache.get(&document_id) {
            if cached.is_valid() && cached.age_seconds() < self.max_cache_age_seconds {
                return Some(&cached.sections);
            }
        }
        None
    }

    /// Check if document content is cached
    pub fn is_cached(&self, document: &Document) -> bool {
        self.get_cached_content(document).is_some()
    }

    /// Preload content for multiple documents
    pub async fn preload_documents(&mut self, documents: &[Document]) -> Result<usize> {
        let mut loaded_count = 0;
        
        for document in documents {
            if !self.is_cached(document) {
                match self.load_document_content(document).await {
                    Ok(_) => loaded_count += 1,
                    Err(_) => continue, // Skip documents that can't be loaded
                }
            }
        }
        
        Ok(loaded_count)
    }

    /// Clear all cached content
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&mut self) {
        let now = chrono::Local::now();
        
        // Remove expired entries
        self.cache.retain(|_, cache| {
            cache.is_valid() && 
            now.signed_duration_since(cache.loaded_at).num_seconds() < self.max_cache_age_seconds
        });

        // If still over limit, remove oldest entries
        if self.cache.len() > self.max_cache_size {
            let mut entries: Vec<_> = self.cache.iter().map(|(k, v)| (k.clone(), v.loaded_at)).collect();
            entries.sort_by_key(|(_, loaded_at)| *loaded_at);
            
            let to_remove = self.cache.len() - self.max_cache_size;
            for i in 0..to_remove {
                if let Some((key, _)) = entries.get(i) {
                    self.cache.remove(key);
                }
            }
        }
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> ContentCacheStats {
        let mut stats = ContentCacheStats::default();
        
        for cache in self.cache.values() {
            stats.total_entries += 1;
            stats.total_sections += cache.sections.len();
            
            if cache.is_valid() {
                stats.valid_entries += 1;
            } else {
                stats.invalid_entries += 1;
            }
            
            if cache.age_seconds() > self.max_cache_age_seconds {
                stats.expired_entries += 1;
            }
        }
        
        stats
    }

    /// Check if a document is available locally (downloaded)
    pub fn is_document_available(&self, document: &Document) -> bool {
        let document_id = self.get_document_id(document);
        let download_dir = PathBuf::from(self.config.download_dir_str());
        let edinet_dir = download_dir.join("edinet").join(&document.ticker);

        if !edinet_dir.exists() {
            return false;
        }

        if let Ok(entries) = std::fs::read_dir(&edinet_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        if filename.contains(&document_id) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Generate document ID for cache keys
    fn get_document_id(&self, document: &Document) -> String {
        document.metadata.get("doc_id")
            .or_else(|| document.metadata.get("document_id"))
            .unwrap_or(&document.id)
            .clone()
    }
}

/// Content cache statistics
#[derive(Debug, Default)]
pub struct ContentCacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub invalid_entries: usize,
    pub expired_entries: usize,
    pub total_sections: usize,
}

impl ContentCacheStats {
    pub fn hit_rate(&self) -> f32 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.valid_entries as f32 / self.total_entries as f32 * 100.0
        }
    }

    pub fn average_sections_per_document(&self) -> f32 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.total_sections as f32 / self.total_entries as f32
        }
    }
}