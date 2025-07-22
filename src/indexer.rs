use anyhow::Result;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tracing::{info, warn, error};
use uuid::Uuid;
use crate::models::{Document, FilingType, Source};
use crate::storage;

pub async fn index_documents(input_dir: &str, database_path: &str) -> Result<usize> {
    info!("Starting indexing from directory: {}", input_dir);
    
    let input_path = Path::new(input_dir);
    if !input_path.exists() {
        anyhow::bail!("Input directory does not exist: {}", input_dir);
    }
    
    let mut indexed_count = 0;
    
    // Walk through the directory structure
    for entry in walkdir::WalkDir::new(input_path) {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            match process_file(path).await {
                Ok(Some(document)) => {
                    if let Err(e) = storage::insert_document(&document, database_path).await {
                        error!("Failed to insert document {}: {}", document.id, e);
                    } else {
                        indexed_count += 1;
                        info!("Indexed document: {} ({})", document.id, path.display());
                    }
                }
                Ok(None) => {
                    // File was processed but not indexed (e.g., not a document file)
                }
                Err(e) => {
                    warn!("Failed to process file {}: {}", path.display(), e);
                }
            }
        }
    }
    
    info!("Indexing completed. Total documents indexed: {}", indexed_count);
    Ok(indexed_count)
}

async fn process_file(file_path: &Path) -> Result<Option<Document>> {
    // Extract information from the file path and filename
    let path_components = extract_path_info(file_path)?;
    
    // Skip non-document files
    if !is_document_file(file_path) {
        return Ok(None);
    }
    
    // Extract content if needed
    let mut metadata = HashMap::new();
    metadata.insert("file_size".to_string(), 
                   std::fs::metadata(file_path)?.len().to_string());
    metadata.insert("file_extension".to_string(), 
                   file_path.extension()
                           .unwrap_or_default()
                           .to_string_lossy()
                           .to_string());
    
    // Try to extract text content for full-text search
    if let Ok(content) = extract_text_content(file_path).await {
        if !content.trim().is_empty() {
            metadata.insert("content_preview".to_string(), 
                           content.chars().take(500).collect::<String>());
        }
    }
    
    let document = Document {
        id: Uuid::new_v4().to_string(),
        ticker: path_components.ticker,
        company_name: path_components.company_name,
        filing_type: path_components.filing_type,
        source: path_components.source,
        date: path_components.date,
        content_path: file_path.to_path_buf(),
        metadata,
    };
    
    Ok(Some(document))
}

struct PathInfo {
    source: Source,
    ticker: String,
    company_name: String,
    filing_type: FilingType,
    date: chrono::NaiveDate,
}

fn extract_path_info(file_path: &Path) -> Result<PathInfo> {
    let path_str = file_path.to_string_lossy();
    let components: Vec<&str> = path_str.split('/').collect();
    
    // Try to extract source from path (e.g., downloads/edgar/AAPL/...)
    let source = if path_str.contains("/edgar/") {
        Source::Edgar
    } else if path_str.contains("/edinet/") {
        Source::Edinet
    } else if path_str.contains("/tdnet/") {
        Source::Tdnet
    } else {
        Source::Other("unknown".to_string())
    };
    
    // Extract ticker from path structure
    let ticker = components.iter()
        .rev()
        .nth(1) // Second to last component should be ticker
        .unwrap_or(&"UNKNOWN")
        .to_string();
    
    // For now, use ticker as company name (could be enhanced with a lookup table)
    let company_name = format!("{} Corp", ticker);
    
    // Try to determine filing type from filename
    let filename = file_path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    
    let filing_type = if filename.contains("10-k") || filename.contains("10k") {
        FilingType::TenK
    } else if filename.contains("10-q") || filename.contains("10q") {
        FilingType::TenQ
    } else if filename.contains("8-k") || filename.contains("8k") {
        FilingType::EightK
    } else if filename.contains("earnings") || filename.contains("transcript") {
        FilingType::Transcript
    } else {
        FilingType::Other("unknown".to_string())
    };
    
    // Use file modification time as date (could be enhanced to parse from content)
    let metadata = std::fs::metadata(file_path)?;
    let modified_time = metadata.modified()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified_time.into();
    let date = datetime.date_naive();
    
    Ok(PathInfo {
        source,
        ticker,
        company_name,
        filing_type,
        date,
    })
}

fn is_document_file(file_path: &Path) -> bool {
    if let Some(extension) = file_path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), "pdf" | "txt" | "html" | "htm" | "xml" | "xbrl")
    } else {
        false
    }
}

async fn extract_text_content(file_path: &Path) -> Result<String> {
    let extension = file_path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    
    match extension.as_str() {
        "txt" => {
            Ok(std::fs::read_to_string(file_path)?)
        }
        "pdf" => {
            // TODO: Implement PDF text extraction using lopdf
            warn!("PDF text extraction not yet implemented");
            Ok(String::new())
        }
        "html" | "htm" => {
            // TODO: Implement HTML text extraction
            warn!("HTML text extraction not yet implemented");
            Ok(String::new())
        }
        "xml" | "xbrl" => {
            // TODO: Implement XML/XBRL parsing using quick-xml
            warn!("XML/XBRL parsing not yet implemented");
            Ok(String::new())
        }
        _ => Ok(String::new())
    }
}