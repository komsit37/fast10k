//! EDINET document indexing functionality

use crate::edinet::{EdinetDocument, EdinetIndexResponse, EdinetApi, EdinetError};
use crate::models::{Document, FilingType, Source, DocumentFormat};
use crate::storage;
use crate::config::Config;
use anyhow::Result;
use chrono::{NaiveDate, Utc, Duration as ChronoDuration, Weekday, Datelike};
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Build EDINET index for the specified number of days back from today
pub async fn build_edinet_index(database_path: &str, days_back: i64) -> Result<usize> {
    let end_date = Utc::now();
    let start_date = end_date - ChronoDuration::days(days_back);

    build_edinet_index_by_date(
        database_path,
        start_date.date_naive(),
        end_date.date_naive(),
    ).await
}

/// Build EDINET index for documents between the specified dates (inclusive)
pub async fn build_edinet_index_by_date(
    database_path: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<usize> {
    let config = Config::from_env()?;
    build_edinet_index_by_date_with_config(database_path, start_date, end_date, &config).await
}

/// Build EDINET index with custom configuration
pub async fn build_edinet_index_by_date_with_config(
    database_path: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
    config: &Config,
) -> Result<usize> {
    println!("🚀 Starting EDINET index build from {} to {}", start_date, end_date);

    // Check for API key
    if config.edinet_api_key.is_none() {
        return Err(EdinetError::MissingApiKey.into());
    }

    println!("✅ EDINET API key found, proceeding with indexing");

    let start_time = Instant::now();
    info!("Indexing EDINET documents from {} to {}", start_date, end_date);

    let client = Client::builder()
        .user_agent(&config.http.user_agent)
        .timeout(config.http_timeout())
        .build()?;

    let mut total_indexed = 0;
    let total_days = (end_date - start_date).num_days() + 1;
    let weekdays: Vec<NaiveDate> = (0..total_days)
        .map(|i| start_date + ChronoDuration::days(i))
        .filter(|date| !matches!(date.weekday(), Weekday::Sat | Weekday::Sun))
        .collect();

    info!("Will process {} weekdays out of {} total days (skipping weekends)", weekdays.len(), total_days);

    for (index, date) in weekdays.iter().enumerate() {
        let date_str = date.format("%Y-%m-%d").to_string();
        
        match get_edinet_documents_for_date(&client, &date_str, config).await {
            Ok(documents) => {
                if !documents.is_empty() {
                    info!("Processing {} EDINET documents for {}", documents.len(), date_str);
                    
                    let indexed_count = index_documents(&documents, database_path).await?;
                    total_indexed += indexed_count;
                    
                    let progress = ((index + 1) as f64 / weekdays.len() as f64 * 100.0) as u32;
                    println!("🗓️  Processing date {} ({}/{} weekdays, {}% complete) - ✅ Indexed {} documents (total: {})", 
                        date_str, index + 1, weekdays.len(), progress, indexed_count, total_indexed);
                } else {
                    debug!("No documents found for {}", date_str);
                }
            }
            Err(e) => {
                warn!("Failed to get documents for {}: {}", date_str, e);
                continue;
            }
        }

        // Rate limiting
        tokio::time::sleep(config.edinet_api_delay()).await;
    }

    let elapsed = start_time.elapsed();
    info!("🎉 EDINET indexing complete!");
    info!("📈 Total documents indexed: {}", total_indexed);
    info!("⏱️  Total time: {} minutes {} seconds", elapsed.as_secs() / 60, elapsed.as_secs() % 60);
    info!("📅 Processed {} weekdays from {} to {}", weekdays.len(), start_date, end_date);

    println!("🎉 EDINET indexing complete!");
    println!("📈 Total documents indexed: {}", total_indexed);
    println!("⏱️  Total time: {} minutes {} seconds", elapsed.as_secs() / 60, elapsed.as_secs() % 60);
    println!("📅 Processed {} weekdays from {} to {}", weekdays.len(), start_date, end_date);

    Ok(total_indexed)
}

/// Update EDINET index from the last indexed date to today
pub async fn update_edinet_index(database_path: &str, days_back: i64) -> Result<usize> {
    info!("Updating EDINET index with documents from last {} days", days_back);
    build_edinet_index(database_path, days_back).await
}

/// Get EDINET documents for a specific date
async fn get_edinet_documents_for_date(
    client: &Client,
    date: &str,
    config: &Config,
) -> Result<Vec<EdinetDocument>, EdinetError> {
    let api_key = config.edinet_api_key.as_ref().ok_or(EdinetError::MissingApiKey)?;
    
    let url = format!("{}{}", EdinetApi::BASE_URL, EdinetApi::DOCUMENTS_ENDPOINT);
    
    debug!("Fetching EDINET documents for date: {}", date);
    
    let response = client
        .get(&url)
        .query(&[("date", date), ("type", "2")]) // type=2 for corporate reports
        .header("Ocp-Apim-Subscription-Key", api_key)
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    if !status.is_success() {
        return Err(EdinetError::ApiError {
            status_code: status.as_u16(),
            message: response_text,
        });
    }

    let edinet_response: EdinetIndexResponse = serde_json::from_str(&response_text)
        .map_err(|e| EdinetError::ApiResponseError {
            date: date.to_string(),
            source: e,
        })?;

    Ok(edinet_response.results)
}

/// Index EDINET documents into the database
async fn index_documents(documents: &[EdinetDocument], database_path: &str) -> Result<usize> {
    let mut indexed_count = 0;

    for doc in documents {
        // Skip documents without required fields
        if doc.doc_id.is_none() || doc.filer_name.is_none() {
            continue;
        }

        let filing_type = map_edinet_form_to_filing_type(doc.form_code.as_deref());
        let format = determine_document_format(doc);

        // Create metadata HashMap
        let mut metadata = HashMap::new();
        
        // Store all EDINET-specific fields in metadata
        if let Some(ref edinet_code) = doc.edinet_code {
            metadata.insert("edinet_code".to_string(), edinet_code.clone());
        }
        if let Some(ref form_code) = doc.form_code {
            metadata.insert("form_code".to_string(), form_code.clone());
        }
        if let Some(ref doc_type_code) = doc.doc_type_code {
            metadata.insert("doc_type_code".to_string(), doc_type_code.clone());
        }
        if let Some(ref period_start) = doc.period_start {
            metadata.insert("period_start".to_string(), period_start.clone());
        }
        if let Some(ref period_end) = doc.period_end {
            metadata.insert("period_end".to_string(), period_end.clone());
        }
        if let Some(ref doc_description) = doc.doc_description {
            metadata.insert("doc_description".to_string(), doc_description.clone());
        }
        if let Some(ref xbrl_flag) = doc.xbrl_flag {
            metadata.insert("xbrl_flag".to_string(), xbrl_flag.clone());
        }
        if let Some(ref pdf_flag) = doc.pdf_flag {
            metadata.insert("pdf_flag".to_string(), pdf_flag.clone());
        }

        let document = Document {
            id: doc.doc_id.as_ref().unwrap().clone(),
            ticker: extract_ticker_from_sec_code(doc.sec_code.as_deref()),
            company_name: doc.filer_name.as_ref().unwrap().clone(),
            filing_type,
            source: Source::Edinet,
            date: parse_submit_date(doc.submit_date.as_deref())?,
            content_path: PathBuf::from(""), // Will be set when document is downloaded
            metadata,
            format,
        };

        // Insert document into database
        if let Err(e) = storage::insert_document(&document, database_path).await {
            warn!("Failed to insert document {}: {}", document.id, e);
            continue;
        }

        indexed_count += 1;
    }

    Ok(indexed_count)
}

/// Map EDINET form code to our FilingType enum
fn map_edinet_form_to_filing_type(form_code: Option<&str>) -> FilingType {
    match form_code {
        Some(code) if code.starts_with("030") => FilingType::TenK, // Annual securities report
        Some(code) if code.starts_with("043") => FilingType::TenQ, // Quarterly securities report
        Some(code) if code.starts_with("120") => FilingType::EightK, // Extraordinary report
        Some(code) => FilingType::Other(format!("EDINET Form {}", code)),
        None => FilingType::Other("Unknown EDINET Form".to_string()),
    }
}

/// Determine document format based on available flags
fn determine_document_format(doc: &EdinetDocument) -> DocumentFormat {
    let has_xbrl = doc.xbrl_flag.as_deref() == Some("1");
    let has_pdf = doc.pdf_flag.as_deref() == Some("1");

    match (has_xbrl, has_pdf) {
        (true, true) => DocumentFormat::Complete,
        (true, false) => DocumentFormat::Xbrl,
        (false, true) => DocumentFormat::Html, // PDF in EDINET is often HTML-based
        (false, false) => DocumentFormat::Txt,
    }
}

/// Extract ticker symbol from securities code
fn extract_ticker_from_sec_code(sec_code: Option<&str>) -> String {
    sec_code
        .map(|code| code.chars().take(4).collect())
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

/// Parse EDINET submit date string to NaiveDate
fn parse_submit_date(submit_date: Option<&str>) -> Result<NaiveDate> {
    match submit_date {
        Some(date_str) => {
            // EDINET date format is typically "YYYY-MM-DD HH:MM:SS"
            let date_part = date_str.split_whitespace().next().unwrap_or(date_str);
            NaiveDate::parse_from_str(date_part, "%Y-%m-%d")
                .map_err(|e| anyhow::anyhow!("Failed to parse date '{}': {}", date_str, e))
        }
        None => Ok(Utc::now().date_naive()),
    }
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
        }
    }
    
    Ok(())
}