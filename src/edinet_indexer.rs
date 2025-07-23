use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use std::collections::HashMap;
use tracing::{debug, info, warn, error};
use uuid::Uuid;
use chrono::{NaiveDate, Utc, Duration as ChronoDuration, Datelike};
use crate::models::{Document, FilingType, Source, DocumentFormat};
use crate::storage;

// Define EDINET structures for indexing (similar to downloader but focused on indexing needs)
#[derive(Debug, Deserialize, Clone)]
pub struct EdinetDocument {
    #[serde(rename = "seqNumber")]
    pub seq_number: i32,
    #[serde(rename = "docID")]
    pub doc_id: Option<String>,
    #[serde(rename = "edinetCode")]
    pub edinet_code: Option<String>,
    #[serde(rename = "secCode")]
    pub sec_code: Option<String>,
    #[serde(rename = "JCN")]
    pub jcn: Option<String>,
    #[serde(rename = "filerName")]
    pub filer_name: Option<String>,
    #[serde(rename = "fundCode")]
    pub fund_code: Option<String>,
    #[serde(rename = "ordinanceCode")]
    pub ordinance_code: Option<String>,
    #[serde(rename = "formCode")]
    pub form_code: Option<String>,
    #[serde(rename = "docTypeCode")]
    pub doc_type_code: Option<String>,
    #[serde(rename = "periodStart")]
    pub period_start: Option<String>,
    #[serde(rename = "periodEnd")]
    pub period_end: Option<String>,
    #[serde(rename = "submitDateTime")]
    pub submit_date: Option<String>,
    #[serde(rename = "docDescription")]
    pub doc_description: Option<String>,
    #[serde(rename = "issuerEdinetCode")]
    pub issuer_edinet_code: Option<String>,
    #[serde(rename = "subjectEdinetCode")]
    pub subject_edinet_code: Option<String>,
    #[serde(rename = "subsidiaryEdinetCode")]
    pub subsidiary_edinet_code: Option<String>,
    #[serde(rename = "currentReportReason")]
    pub current_report_reason: Option<String>,
    #[serde(rename = "parentDocID")]
    pub parent_doc_id: Option<String>,
    #[serde(rename = "opeDateTime")]
    pub ope_date_time: Option<String>,
    #[serde(rename = "withdrawalStatus")]
    pub withdrawal_status: Option<String>,
    #[serde(rename = "docInfoEditStatus")]
    pub doc_info_edit_status: Option<String>,
    #[serde(rename = "disclosureStatus")]
    pub disclosure_request_status: Option<String>,
    #[serde(rename = "xbrlFlag")]
    pub xbrl_flag: Option<String>,
    #[serde(rename = "pdfFlag")]
    pub pdf_flag: Option<String>,
    #[serde(rename = "attachDocFlag")]
    pub attach_doc_flag: Option<String>,
    #[serde(rename = "englishDocFlag")]
    pub english_flag: Option<String>,
    #[serde(rename = "csvFlag", default)]
    pub csv_flag: Option<String>,
    #[serde(rename = "legalStatus", default)]
    pub legal_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EdinetIndexResponse {
    metadata: EdinetMetaData,
    results: Vec<EdinetDocument>,
}

#[derive(Debug, Deserialize)]
struct EdinetMetaData {
    title: String,
    parameter: EdinetParameter,
    resultset: EdinetResultSet,
}

#[derive(Debug, Deserialize)]
struct EdinetParameter {
    date: String,
    #[serde(rename = "type")]
    doc_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EdinetResultSet {
    count: i32,
}

// EDINET API endpoints
const EDINET_BASE_URL: &str = "https://api.edinet-fsa.go.jp";
const DOCUMENTS_ENDPOINT: &str = "/api/v2/documents.json";

/// Build comprehensive EDINET index using date range
pub async fn build_edinet_index_by_date(database_path: &str, start_date: NaiveDate, end_date: NaiveDate) -> Result<usize> {
    info!("Building EDINET index from {} to {}", start_date, end_date);
    
    // Check if API key is available
    let api_key = std::env::var("EDINET_API_KEY").map_err(|_| 
        anyhow!("EDINET_API_KEY environment variable not set. Required for EDINET indexing."))?;
    
    let client = Client::builder()
        .user_agent("fast10k/0.1.0")
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let mut total_indexed = 0;
    
    // Convert NaiveDate to DateTime<Utc> for processing
    let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap().and_utc();
    
    info!("Indexing EDINET documents from {} to {}", 
        start_datetime.format("%Y-%m-%d"), 
        end_datetime.format("%Y-%m-%d"));
    
    // Calculate total days for progress tracking
    let total_days = (end_datetime - start_datetime).num_days() + 1;
    
    // Index documents day by day (but with sampling for efficiency)
    let mut current_date = start_datetime;
    let mut processed_days = 0;
    
    while current_date <= end_datetime {
        let date_str = current_date.format("%Y-%m-%d").to_string();
        
        // Skip weekends for efficiency (less filings on weekends)
        let weekday = current_date.weekday();
        if matches!(weekday, chrono::Weekday::Sat | chrono::Weekday::Sun) {
            current_date = current_date + ChronoDuration::days(1);
            continue;
        }
        
        info!("Processing EDINET documents for date: {} ({}/{})", 
            date_str, processed_days + 1, total_days);
        
        match index_documents_for_date(&client, &api_key, &date_str, database_path).await {
            Ok(count) => {
                total_indexed += count;
                info!("Indexed {} documents for {}", count, date_str);
            }
            Err(e) => {
                warn!("Failed to index documents for {}: {}", date_str, e);
            }
        }
        
        processed_days += 1;
        current_date = current_date + ChronoDuration::days(1);
        
        // Rate limiting - be respectful to EDINET API
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Progress reporting
        if processed_days % 10 == 0 {
            info!("Progress: {}/{} days processed, {} documents indexed", 
                processed_days, total_days, total_indexed);
        }
    }
    
    info!("EDINET indexing complete. Total documents indexed: {}", total_indexed);
    Ok(total_indexed)
}

/// Build comprehensive EDINET index going back specified number of days (legacy function)
pub async fn build_edinet_index(database_path: &str, days_back: i64) -> Result<usize> {
    let end_date = Utc::now().date_naive();
    let start_date = (Utc::now() - ChronoDuration::days(days_back)).date_naive();
    build_edinet_index_by_date(database_path, start_date, end_date).await
}

/// Index documents for a specific date
async fn index_documents_for_date(
    client: &Client, 
    api_key: &str, 
    date_str: &str,
    database_path: &str
) -> Result<usize> {
    let url = format!("{}{}", EDINET_BASE_URL, DOCUMENTS_ENDPOINT);
    
    let response = client
        .get(&url)
        .query(&[("date", date_str), ("type", "2")]) // type=2 for corporate reports
        .header("Ocp-Apim-Subscription-Key", api_key)
        .send()
        .await?;
    
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("EDINET API request failed for {}: {}", date_str, status));
    }
    
    let response_text = response.text().await?;
    debug!("EDINET API response for {}: {} bytes", date_str, response_text.len());
    
    let edinet_response: EdinetIndexResponse = serde_json::from_str(&response_text)
        .map_err(|e| anyhow!("Failed to parse EDINET response for {}: {}", date_str, e))?;
    
    let documents = edinet_response.results;
    info!("Processing {} EDINET documents for {}", documents.len(), date_str);
    
    let mut indexed_count = 0;
    
    for edinet_doc in documents {
        match convert_edinet_to_document(&edinet_doc, date_str) {
            Ok(document) => {
                match storage::insert_document(&document, database_path).await {
                    Ok(_) => {
                        indexed_count += 1;
                        debug!("Indexed EDINET document: {} - {}", 
                            document.ticker, document.company_name);
                    }
                    Err(e) => {
                        warn!("Failed to insert EDINET document {}: {}", document.id, e);
                    }
                }
            }
            Err(e) => {
                debug!("Skipped EDINET document: {}", e);
            }
        }
    }
    
    Ok(indexed_count)
}

/// Convert EDINET document to our standard Document format
fn convert_edinet_to_document(edinet_doc: &EdinetDocument, date_str: &str) -> Result<Document> {
    // Skip documents with null required fields
    if edinet_doc.doc_id.is_none() || edinet_doc.filer_name.is_none() {
        return Err(anyhow!("Document has null required fields"));
    }
    
    // Extract ticker from secCode if available
    let ticker = edinet_doc.sec_code
        .as_ref()
        .map(|code| code.chars().take(4).collect::<String>())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    
    // Skip documents without proper identification
    if ticker == "UNKNOWN" && edinet_doc.edinet_code.is_none() {
        return Err(anyhow!("Document lacks proper identification"));
    }
    
    // Use EDINET code as ticker if sec_code is not available
    let final_ticker = if ticker == "UNKNOWN" {
        edinet_doc.edinet_code
            .as_ref()
            .map(|code| code.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string())
    } else {
        ticker
    };
    
    // Map EDINET form codes to our FilingType enum
    let filing_type = map_edinet_form_code(&edinet_doc.form_code);
    
    // Parse the date
    let document_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .unwrap_or_else(|_| Utc::now().date_naive());
    
    // Build metadata
    let mut metadata = HashMap::new();
    metadata.insert("edinet_code".to_string(), 
        edinet_doc.edinet_code.clone().unwrap_or_default());
    metadata.insert("doc_id".to_string(), edinet_doc.doc_id.clone().unwrap_or_default());
    metadata.insert("jcn".to_string(), 
        edinet_doc.jcn.clone().unwrap_or_default());
    metadata.insert("form_code".to_string(), 
        edinet_doc.form_code.clone().unwrap_or_default());
    metadata.insert("doc_type_code".to_string(), 
        edinet_doc.doc_type_code.clone().unwrap_or_default());
    metadata.insert("submit_date".to_string(), edinet_doc.submit_date.clone().unwrap_or_default());
    metadata.insert("doc_description".to_string(), edinet_doc.doc_description.clone().unwrap_or_default());
    metadata.insert("ordinance_code".to_string(), 
        edinet_doc.ordinance_code.clone().unwrap_or_default());
    
    // Add flags
    if let Some(xbrl_flag) = &edinet_doc.xbrl_flag {
        metadata.insert("xbrl_flag".to_string(), xbrl_flag.clone());
    }
    if let Some(pdf_flag) = &edinet_doc.pdf_flag {
        metadata.insert("pdf_flag".to_string(), pdf_flag.clone());
    }
    if let Some(english_flag) = &edinet_doc.english_flag {
        metadata.insert("english_flag".to_string(), english_flag.clone());
    }
    
    // Add period information if available
    if let Some(period_start) = &edinet_doc.period_start {
        metadata.insert("period_start".to_string(), period_start.clone());
    }
    if let Some(period_end) = &edinet_doc.period_end {
        metadata.insert("period_end".to_string(), period_end.clone());
    }
    
    // Determine document format based on flags
    let format = determine_document_format(edinet_doc);
    
    let document = Document {
        id: Uuid::new_v4().to_string(),
        ticker: final_ticker,
        company_name: edinet_doc.filer_name.clone().unwrap_or_else(|| "Unknown Company".to_string()),
        filing_type,
        source: Source::Edinet,
        date: document_date,
        content_path: std::path::PathBuf::from(format!("edinet/{}", edinet_doc.doc_id.clone().unwrap_or_else(|| "unknown".to_string()))),
        metadata,
        format,
    };
    
    Ok(document)
}

/// Determine document format based on EDINET flags
fn determine_document_format(edinet_doc: &EdinetDocument) -> DocumentFormat {
    let mut formats = Vec::new();
    
    // Check for XBRL availability
    if let Some(xbrl_flag) = &edinet_doc.xbrl_flag {
        if xbrl_flag == "1" {
            formats.push("xbrl");
        }
    }
    
    // Check for PDF availability 
    if let Some(pdf_flag) = &edinet_doc.pdf_flag {
        if pdf_flag == "1" {
            formats.push("pdf");
        }
    }
    
    // Return appropriate format based on what's available
    if formats.is_empty() {
        DocumentFormat::Complete
    } else if formats.len() == 1 {
        match formats[0] {
            "xbrl" => DocumentFormat::Xbrl,
            "pdf" => DocumentFormat::Html,
            _ => DocumentFormat::Complete,
        }
    } else {
        // Multiple formats available - use Other to store comma-separated list
        DocumentFormat::Other(formats.join(","))
    }
}

/// Map EDINET form codes to our FilingType enum
fn map_edinet_form_code(form_code: &Option<String>) -> FilingType {
    match form_code.as_deref() {
        Some("030000") => FilingType::TenK,      // Annual securities report (equivalent to 10-K)
        Some("043000") => FilingType::TenQ,      // Quarterly securities report (equivalent to 10-Q) 
        Some("010000") => FilingType::Other("Internal Control Report".to_string()),
        Some("070000") => FilingType::Other("Extraordinary Report".to_string()),
        Some("042000") => FilingType::Other("Confirmation Letter".to_string()),
        Some("060000") => FilingType::Other("Reference Document".to_string()),
        Some("995000") => FilingType::Other("Extraordinary Report (Securities)".to_string()),
        Some("07A000") => FilingType::Other("Securities Report (Investment Trust)".to_string()),
        Some("10A000") => FilingType::Other("Semi-Annual Report (Investment Trust)".to_string()),
        Some("04A000") | Some("04A001") => FilingType::Other("Securities Registration Statement".to_string()),
        Some(other) => FilingType::Other(format!("EDINET Form {}", other)),
        None => FilingType::Other("Unknown EDINET Form".to_string()),
    }
}

/// Update EDINET index with recent documents (last N days)
pub async fn update_edinet_index(database_path: &str, days_back: i64) -> Result<usize> {
    info!("Updating EDINET index with documents from last {} days", days_back);
    build_edinet_index(database_path, days_back).await
}

/// Get statistics about the EDINET index
pub async fn get_edinet_index_stats(database_path: &str) -> Result<()> {
    println!("EDINET Index Statistics:");
    info!("EDINET Index Statistics:");
    
    // Get total document count for EDINET source
    match storage::count_documents_by_source(&Source::Edinet, database_path).await {
        Ok(count) => {
            println!("Total EDINET documents: {}", count);
            info!("Total EDINET documents: {}", count);
        },
        Err(e) => {
            println!("Failed to get document count: {}", e);
            warn!("Failed to get document count: {}", e);
        },
    }
    
    // Get date range
    match storage::get_date_range_for_source(&Source::Edinet, database_path).await {
        Ok((start, end)) => {
            println!("Date range: {} to {}", start, end);
            info!("Date range: {} to {}", start, end);
        },
        Err(e) => {
            println!("Failed to get date range: {}", e);
            warn!("Failed to get date range: {}", e);
        },
    }
    
    // Get top companies by document count
    match storage::get_top_companies_for_source(&Source::Edinet, database_path, 10).await {
        Ok(companies) => {
            println!("Top 10 companies by document count:");
            info!("Top 10 companies by document count:");
            for (company, count) in companies {
                println!("  {}: {} documents", company, count);
                info!("  {}: {} documents", company, count);
            }
        }
        Err(e) => {
            println!("Failed to get top companies: {}", e);
            warn!("Failed to get top companies: {}", e);
        },
    }
    
    Ok(())
}