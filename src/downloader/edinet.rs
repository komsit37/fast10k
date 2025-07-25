//! EDINET downloader interface
//! 
//! This module provides the interface for the downloader system to access
//! EDINET functionality. The actual implementation is in the `edinet` module.

use crate::models::DownloadRequest;
use crate::edinet;
use anyhow::Result;

/// Download EDINET documents (delegated to edinet module)
pub async fn download(request: &DownloadRequest, output_dir: &str) -> Result<usize> {
    edinet::downloader::download_documents(request, output_dir).await
}