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
        format: crate::models::DocumentFormat::Complete, // Default format for indexed files
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
    
    // Extract company name and filing details from actual file content if available
    let (company_name, filing_type, date) = if matches!(source, Source::Edgar) {
        extract_edgar_info(file_path, &ticker)?
    } else {
        // Fallback for non-EDGAR files
        let company_name = format!("{} Corp", ticker);
        let filing_type = determine_filing_type_from_filename(file_path);
        let date = get_file_date(file_path)?;
        (company_name, filing_type, date)
    };
    
    Ok(PathInfo {
        source,
        ticker,
        company_name,
        filing_type,
        date,
    })
}

fn extract_edgar_info(file_path: &Path, ticker: &str) -> Result<(String, FilingType, chrono::NaiveDate)> {
    // Try to extract info from EDGAR filename pattern: FORM-YYYY-MM-DD-ACCESSION.ext
    let filename = file_path.file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    
    let parts: Vec<&str> = filename.split('-').collect();
    
    // Extract filing type from first part of filename
    let filing_type = if parts.len() > 0 {
        match parts[0].to_uppercase().as_str() {
            "10-K" | "10K" | "10" => FilingType::TenK,
            "10-Q" | "10Q" => FilingType::TenQ,
            "8-K" | "8K" | "8" => FilingType::EightK,
            "4" => FilingType::Other("Form 4".to_string()),
            "144" => FilingType::Other("Form 144".to_string()),
            "DEF 14A" | "DEF14A" | "DEFA14A" => FilingType::Other("Proxy Statement".to_string()),
            "S-3ASR" | "S3ASR" => FilingType::Other("Registration Statement".to_string()),
            "424B2" => FilingType::Other("Prospectus".to_string()),
            "25-NSE" => FilingType::Other("Notification".to_string()),
            "FWP" => FilingType::Other("Free Writing Prospectus".to_string()),
            "SD" => FilingType::Other("Specialized Disclosure".to_string()),
            "PX14A6G" => FilingType::Other("Proxy Statement".to_string()),
            "3" => FilingType::Other("Form 3".to_string()),
            "11-K" => FilingType::Other("Form 11-K".to_string()),
            "SAMPLE" => FilingType::Other("Sample Document".to_string()),
            other => FilingType::Other(other.to_string()),
        }
    } else {
        determine_filing_type_from_filename(file_path)
    };
    
    // Extract date from filename pattern
    let date = if parts.len() >= 4 {
        // Try to parse YYYY-MM-DD from parts[1], parts[2], parts[3]
        if let (Ok(year), Ok(month), Ok(day)) = (
            parts[1].parse::<i32>(),
            parts[2].parse::<u32>(),
            parts[3].parse::<u32>()
        ) {
            chrono::NaiveDate::from_ymd_opt(year, month, day)
                .unwrap_or_else(|| get_file_date(file_path).unwrap_or_else(|_| chrono::Utc::now().date_naive()))
        } else {
            get_file_date(file_path)?
        }
    } else {
        get_file_date(file_path)?
    };
    
    // Try to extract company name from file content if it's a text file
    let company_name = if let Ok(content) = std::fs::read_to_string(file_path) {
        extract_company_name_from_content(&content, ticker)
    } else {
        get_company_name_from_ticker(ticker)
    };
    
    Ok((company_name, filing_type, date))
}

fn determine_filing_type_from_filename(file_path: &Path) -> FilingType {
    let filename = file_path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    
    if filename.contains("10-k") || filename.contains("10k") {
        FilingType::TenK
    } else if filename.contains("10-q") || filename.contains("10q") {
        FilingType::TenQ
    } else if filename.contains("8-k") || filename.contains("8k") {
        FilingType::EightK
    } else if filename.contains("earnings") || filename.contains("transcript") {
        FilingType::Transcript
    } else {
        FilingType::Other("unknown".to_string())
    }
}

fn get_file_date(file_path: &Path) -> Result<chrono::NaiveDate> {
    let metadata = std::fs::metadata(file_path)?;
    let modified_time = metadata.modified()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified_time.into();
    Ok(datetime.date_naive())
}

fn extract_company_name_from_content(content: &str, ticker: &str) -> String {
    // Look for company name in EDGAR filing headers
    let lines = content.lines().take(50); // Check first 50 lines
    
    for line in lines {
        if line.contains("COMPANY CONFORMED NAME:") {
            if let Some(name_part) = line.split("COMPANY CONFORMED NAME:").nth(1) {
                let name = name_part.trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }
        }
        // Alternative patterns
        if line.contains("CONFORMED NAME:") {
            if let Some(name_part) = line.split("CONFORMED NAME:").nth(1) {
                let name = name_part.trim().to_string();
                if !name.is_empty() {
                    return name;
                }
            }
        }
    }
    
    // Fallback to ticker-based name
    get_company_name_from_ticker(ticker)
}

fn get_company_name_from_ticker(ticker: &str) -> String {
    // Simple mapping for common tickers - could be expanded
    match ticker.to_uppercase().as_str() {
        "AAPL" => "Apple Inc.".to_string(),
        "MSFT" => "Microsoft Corporation".to_string(),
        "GOOGL" | "GOOG" => "Alphabet Inc.".to_string(),
        "TSLA" => "Tesla, Inc.".to_string(),
        "AMZN" => "Amazon.com, Inc.".to_string(),
        "META" => "Meta Platforms, Inc.".to_string(),
        "NVDA" => "NVIDIA Corporation".to_string(),
        "AMD" => "Advanced Micro Devices, Inc.".to_string(),
        _ => format!("{} Corp.", ticker)
    }
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