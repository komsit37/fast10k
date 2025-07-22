use anyhow::Result;
use crate::models::{DownloadRequest, Source};

pub mod edgar;
pub mod edinet;
pub mod tdnet;

pub async fn download_documents(request: &DownloadRequest, output_dir: &str) -> Result<usize> {
    // Create output directory if it doesn't exist
    std::fs::create_dir_all(output_dir)?;
    
    match &request.source {
        Source::Edgar => edgar::download(request, output_dir).await,
        Source::Edinet => edinet::download(request, output_dir).await,
        Source::Tdnet => tdnet::download(request, output_dir).await,
        Source::Other(name) => {
            anyhow::bail!("Unsupported source: {}", name)
        }
    }
}