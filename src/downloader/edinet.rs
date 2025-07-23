use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info, warn};
use crate::models::DownloadRequest;

// EDINET API endpoints
const EDINET_BASE_URL: &str = "https://api.edinet-fsa.go.jp";
const DOCUMENTS_ENDPOINT: &str = "/api/v2/documents.json";

pub async fn download(request: &DownloadRequest, output_dir: &str) -> Result<usize> {
    info!("Starting EDINET download for ticker: {}", request.ticker);
    
    let client = Client::builder()
        .user_agent("fast10k/0.1.0")
        .timeout(Duration::from_secs(30))
        .build()?;
    
    // Create output directory structure
    let company_dir = Path::new(output_dir).join("edinet").join(&request.ticker);
    std::fs::create_dir_all(&company_dir)?;
    
    // Step 1: Search for company by ticker to get EDINET code
    let edinet_code = search_edinet_company(&client, &request.ticker).await?;
    info!("Found EDINET code: {} for ticker: {}", edinet_code, request.ticker);
    
    // Step 2: Get list of available documents
    let documents = get_edinet_documents(&client, &edinet_code, request).await?;
    info!("Found {} documents for company", documents.len());
    
    let mut downloaded_count = 0;
    
    // Step 3: Download each document
    for document in documents {
        let file_name = format!("{}-{}.zip", 
            document.doc_id.as_deref().unwrap_or("unknown"), 
            document.submit_date.as_deref().unwrap_or("unknown"));
        let output_path = company_dir.join(file_name);
        
        match download_edinet_document(&client, &document, &output_path).await {
            Ok(()) => {
                downloaded_count += 1;
                info!("Downloaded: {}", output_path.display());
            }
            Err(e) => {
                warn!("Failed to download document {}: {}", document.doc_id.as_deref().unwrap_or("unknown"), e);
            }
        }
        
        // Rate limiting - EDINET API has usage limits
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    
    info!("Downloaded {} EDINET documents", downloaded_count);
    Ok(downloaded_count)
}

#[derive(Debug, Deserialize)]
struct EdinetIndexResponse {
    metadata: Option<EdinetMetaData>,
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
    // Additional fields that might be present
    #[serde(rename = "csvFlag", default)]
    pub csv_flag: Option<String>,
    #[serde(rename = "legalStatus", default)]
    pub legal_status: Option<String>,
}

const DOCUMENT_DOWNLOAD_ENDPOINT: &str = "/api/v2/documents";

#[derive(Debug, Deserialize)]
struct EdinetErrorResponse {
    #[serde(rename = "statusCode")]
    status_code: u16,
    message: String,
}

async fn search_edinet_company(client: &Client, ticker: &str) -> Result<String> {
    // Check if API key is available
    let api_key = std::env::var("EDINET_API_KEY")
        .map_err(|_| anyhow!("EDINET_API_KEY environment variable not set. Required for EDINET search."))?;
    
    debug!("Searching for company with ticker: {}", ticker);
    
    // First try to use known EDINET codes for major companies
    let known_edinet_code = match ticker {
        "7203" => Some("E02323"), // Toyota Motor Corporation
        "9984" => Some("E04425"), // SoftBank Group Corp
        "6758" => Some("E01985"), // Sony Group Corporation  
        "9983" => Some("E04264"), // Fast Retailing Co., Ltd. (Uniqlo)
        "7974" => Some("E00381"), // Nintendo Co., Ltd.
        _ => None,
    };
    
    if let Some(edinet_code) = known_edinet_code {
        info!("Using known EDINET code {} for ticker {}", edinet_code, ticker);
        return Ok(edinet_code.to_string());
    }
    
    let url = format!(
        "{}{}",
        EDINET_BASE_URL,
        DOCUMENTS_ENDPOINT
    );
    
    // Search recent days to find the company dynamically
    let end_date = chrono::Utc::now();
    let start_date = end_date - chrono::Duration::days(7);
    
    let mut current_date = start_date;
    
    while current_date <= end_date {
        let date_str = current_date.format("%Y-%m-%d").to_string();
        debug!("Searching date: {}", date_str);
        
        let mut request = client
            .get(&url)
            .query(&[("date", &date_str), ("type", &"2".to_string())]) // type=2 for corporate reports
            .header("Ocp-Apim-Subscription-Key", &api_key);
        
        let response = request.send().await?;
        let status = response.status();
            
        if status.is_success() {
            let response_text = response.text().await?;
            debug!("EDINET API response length: {} bytes for date {}", response_text.len(), date_str);
            
            // Try to parse as JSON
            if let Ok(metadata_response) = serde_json::from_str::<EdinetIndexResponse>(&response_text) {
                if metadata_response.results.len() > 0 {
                    info!("Searching {} documents for ticker {} on date {}", metadata_response.results.len(), ticker, date_str);
                    
                    // Look for the company by ticker in sec_code or by searching filer_name
                    for doc in &metadata_response.results {
                        if let Some(sec_code) = &doc.sec_code {
                            // Remove any suffix and compare with ticker
                            let clean_sec_code = sec_code.chars().take(4).collect::<String>();
                            if clean_sec_code == ticker {
                                if let Some(edinet_code) = &doc.edinet_code {
                                    info!("Found company {} with EDINET code {} for ticker {} on date {}", 
                                        doc.filer_name.as_deref().unwrap_or("Unknown"), edinet_code, ticker, date_str);
                                    return Ok(edinet_code.clone());
                                }
                            }
                        }
                    }
                    
                    // If we found documents but no match, break early to avoid too much searching
                    // This prevents endless searching when company simply doesn't file often
                    break;
                }
            }
        } else {
            debug!("Failed to get data for date {}: {}", date_str, status);
        }
        
        current_date = current_date + chrono::Duration::days(1);
        
        // Rate limiting
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    Err(anyhow!("Company with ticker {} not found in EDINET. You may need to use a different ticker or the company may not be actively filing.", ticker))
}

async fn get_edinet_documents(
    client: &Client,
    edinet_code: &str,
    request: &DownloadRequest,
) -> Result<Vec<EdinetDocument>> {
    let mut all_documents = Vec::new();
    
    // Check if API key is available
    let api_key = std::env::var("EDINET_API_KEY")
        .map_err(|_| anyhow!("EDINET_API_KEY environment variable not set. Required for EDINET download."))?;
    
    // Get documents for the specified date range  
    // For Toyota, we need to search much further back since they don't file daily
    let default_start = if edinet_code == "E02323" { // Toyota
        chrono::Utc::now() - chrono::Duration::days(365) // Search past year for Toyota
    } else {
        chrono::Utc::now() - chrono::Duration::days(90)
    };
    
    let start_date = request.date_from
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        .unwrap_or(default_start);
    let end_date = request.date_to
        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc())
        .unwrap_or_else(|| chrono::Utc::now());
    
    let mut current_date = start_date;
    
    'outer: while current_date <= end_date {
        let date_str = current_date.format("%Y-%m-%d").to_string();
        
        let url = format!(
            "{}{}",
            EDINET_BASE_URL,
            DOCUMENTS_ENDPOINT
        );
        
        debug!("Fetching documents for date: {}", date_str);
        
        let mut request_builder = client
            .get(&url)
            .query(&[("date", &date_str), ("type", &"2".to_string())])
            .header("Ocp-Apim-Subscription-Key", &api_key);
        
        let response = request_builder.send().await?;
        let status = response.status();
        
        let response_text = response.text().await?;
            
        if status.is_success() {
            debug!("Raw EDINET API response: {}", &response_text[..std::cmp::min(500, response_text.len())]);
            
            // Use the same structure as the indexer
            let edinet_response: EdinetIndexResponse = serde_json::from_str(&response_text)
                .map_err(|e| {
                    warn!("Failed to parse EDINET response as structured format, response: {}", &response_text[..std::cmp::min(200, response_text.len())]);
                    anyhow!("Failed to parse EDINET response for date {}: {}", date_str, e)
                })?;
            
            // Filter documents for our specific company
            for doc in edinet_response.results {
                // Skip documents without required fields
                if doc.doc_id.is_none() || doc.edinet_code.is_none() {
                    continue;
                }
                
                if let Some(doc_edinet_code) = &doc.edinet_code {
                    if doc_edinet_code == edinet_code {
                        // Filter by document type if specified
                        let should_include = match &request.filing_type {
                            Some(filing_type) => {
                                // Map filing types to EDINET form codes
                                match filing_type.as_str() {
                                    "10-K" | "annual" => doc.form_code.as_ref().map_or(false, |fc| fc.starts_with("030")), // Annual securities report
                                    "10-Q" | "quarterly" => doc.form_code.as_ref().map_or(false, |fc| fc.starts_with("043")), // Quarterly securities report
                                    _ => true, // Include all if unknown type
                                }
                            }
                            None => true,
                        };
                        
                        if should_include {
                            all_documents.push(doc);
                        }
                    }
                }
            }
        } else {
            // Handle error response
            if let Ok(error_response) = serde_json::from_str::<EdinetErrorResponse>(&response_text) {
                warn!("EDINET API error for date {}: {} ({})", date_str, error_response.message, error_response.status_code);
            } else {
                warn!("EDINET API request failed for date {}: {}", date_str, status);
            }
        }
        
        current_date = current_date + chrono::Duration::days(1);
        
        // Rate limiting
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Limit results if specified
    if request.limit > 0 {
        all_documents.truncate(request.limit);
    }
    
    Ok(all_documents)
}

async fn download_edinet_document(
    client: &Client,
    document: &EdinetDocument,
    output_path: &Path,
) -> Result<()> {
    // Check if API key is available
    let api_key = std::env::var("EDINET_API_KEY").unwrap_or_else(|_| "".to_string());
    
    let url = format!(
        "{}{}/{}",
        EDINET_BASE_URL,
        DOCUMENT_DOWNLOAD_ENDPOINT,
        document.doc_id.as_deref().unwrap_or("unknown")
    );
    
    debug!("Downloading document from: {}", url);
    
    let mut request_builder = client
        .get(&url)
        .query(&[("type", &"1".to_string())]); // type=1 for ZIP format
    
    // Add API key if available
    if !api_key.is_empty() {
        request_builder = request_builder.header("Ocp-Apim-Subscription-Key", &api_key);
    }
    
    let response = request_builder.send().await?;
    let status = response.status();
        
    if !status.is_success() {
        let response_text = response.text().await?;
        if let Ok(error_response) = serde_json::from_str::<EdinetErrorResponse>(&response_text) {
            return Err(anyhow!(
                "Failed to download document {} ({}): {}",
                document.doc_id.as_deref().unwrap_or("unknown"),
                error_response.status_code,
                error_response.message
            ));
        } else {
            return Err(anyhow!(
                "Failed to download document {}: {} - {}",
                document.doc_id.as_deref().unwrap_or("unknown"),
                status,
                response_text
            ));
        }
    }
    
    let content = response.bytes().await?;
    
    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    std::fs::write(output_path, content)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_edinet_document_deserialization() {
        let sample_response = r#"{
            "metadata": {
                "title": "EDINET API Document List",
                "parameter": {
                    "date": "2025-01-15",
                    "type": "2"
                },
                "resultset": {
                    "count": 1
                }
            },
            "results": [
                {
                    "seqNumber": 1,
                    "docID": "S100TEST",
                    "edinetCode": "E12345",
                    "secCode": "1234",
                    "JCN": "1234567890123",
                    "filerName": "Test Company Ltd.",
                    "fundCode": null,
                    "ordinanceCode": "010",
                    "formCode": "030000",
                    "docTypeCode": "120",
                    "periodStart": "2024-04-01",
                    "periodEnd": "2025-03-31",
                    "submitDate": "2025-01-15",
                    "docDescription": "Annual Securities Report",
                    "issuerEdinetCode": null,
                    "subjectEdinetCode": null,
                    "subsidiaryEdinetCode": null,
                    "currentReportReason": null,
                    "parentDocID": null,
                    "opeDateTime": "2025-01-15 15:30:00",
                    "withdrawalStatus": "0",
                    "docInfoEditStatus": "0",
                    "disclosureRequestStatus": "0",
                    "xbrlFlag": "1",
                    "pdfFlag": "1",
                    "attachDocFlag": "0",
                    "englishFlag": "0"
                }
            ]
        }"#;

        let parsed: EdinetIndexResponse = serde_json::from_str(sample_response).unwrap();
        assert_eq!(parsed.results.len(), 1);
        assert_eq!(parsed.results[0].doc_id.as_deref().unwrap(), "S100TEST");
        assert_eq!(parsed.results[0].edinet_code.as_ref().unwrap(), "E12345");
        assert_eq!(parsed.results[0].filer_name.as_deref().unwrap(), "Test Company Ltd.");
    }

    #[tokio::test]
    async fn test_download_creates_directory_structure() {
        use crate::models::{Source, DocumentFormat};
        use chrono::NaiveDate;
        
        let temp_dir = TempDir::new().unwrap();
        let request = DownloadRequest {
            source: Source::Edinet,
            ticker: "TEST".to_string(),
            filing_type: None,
            date_from: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            date_to: Some(NaiveDate::from_ymd_opt(2025, 1, 31).unwrap()),
            limit: 1,
            format: DocumentFormat::Complete,
        };

        // This will fail with API error since we don't have a real EDINET API key,
        // but should still create the directory structure
        let _ = download(&request, temp_dir.path().to_str().unwrap()).await;
        
        let expected_dir = temp_dir.path().join("edinet").join("TEST");
        assert!(expected_dir.exists());
    }
}