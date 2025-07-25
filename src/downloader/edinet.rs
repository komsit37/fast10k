use crate::models::DownloadRequest;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info, warn};

// EDINET API endpoints
const EDINET_BASE_URL: &str = "https://api.edinet-fsa.go.jp";

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
    info!(
        "Found EDINET code: {} for ticker: {}",
        edinet_code, request.ticker
    );

    // Step 2: Get list of available documents from local database
    let documents = get_edinet_documents_from_db(&client, &edinet_code, request).await?;
    info!("Found {} documents for company", documents.len());

    let mut downloaded_count = 0;

    // Step 3: Download each document
    for (index, document) in documents.iter().enumerate() {
        let file_name = format!(
            "{}-{}.zip",
            document.doc_id.as_deref().unwrap_or("unknown"),
            document.submit_date.as_deref().unwrap_or("unknown")
        );
        let output_path = company_dir.join(file_name);

        // Log document details before downloading
        info!(
            "Downloading document {}/{}: {} - {} ({})",
            index + 1,
            documents.len(),
            document.doc_id.as_deref().unwrap_or("unknown"),
            document
                .doc_description
                .as_deref()
                .unwrap_or("Unknown document type"),
            document.submit_date.as_deref().unwrap_or("unknown date")
        );

        match download_edinet_document(&client, &document, &output_path).await {
            Ok(()) => {
                downloaded_count += 1;
                info!("✓ Successfully downloaded: {}", output_path.display());
            }
            Err(e) => {
                warn!(
                    "✗ Failed to download document {}: {}",
                    document.doc_id.as_deref().unwrap_or("unknown"),
                    e
                );
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

async fn search_edinet_company(_client: &Client, ticker: &str) -> Result<String> {
    debug!("Searching for company with ticker: {}", ticker);

    // Find EDINET code from static database only
    match crate::storage::get_edinet_code_by_securities_code("./fast10k.db", ticker).await {
        Ok(Some(edinet_code)) => {
            info!(
                "Found EDINET code {} for ticker {} in static database",
                edinet_code, ticker
            );
            Ok(edinet_code)
        }
        Ok(None) => {
            Err(anyhow!("Company with ticker {} not found in static database. Make sure the static data is loaded with 'edinet load-static'", ticker))
        }
        Err(e) => {
            Err(anyhow!("Failed to query static database for ticker {}: {}", ticker, e))
        }
    }
}

async fn get_edinet_documents_from_db(
    _client: &Client,
    _edinet_code: &str,
    request: &DownloadRequest,
) -> Result<Vec<EdinetDocument>> {
    // Query local database instead of scanning API
    let search_query = crate::models::SearchQuery {
        ticker: Some(request.ticker.clone()),
        company_name: None,
        filing_type: request.filing_type.clone(),
        source: Some(crate::models::Source::Edinet),
        date_from: request.date_from,
        date_to: request.date_to,
        text_query: None,
    };

    info!("Querying documents database for documents...");
    let documents =
        crate::storage::search_documents(&search_query, "./fast10k.db", request.limit).await?;
    info!("Found {} documents in documents database", documents.len());

    // Convert Document to EdinetDocument for downloading
    let mut edinet_documents = Vec::new();
    for doc in documents {
        // Extract document ID from metadata if available, otherwise use the document ID
        let doc_id = doc
            .metadata
            .get("doc_id")
            .or_else(|| doc.metadata.get("document_id"))
            .unwrap_or(&doc.id)
            .clone();

        let edinet_doc = EdinetDocument {
            seq_number: 0, // Not used for download
            doc_id: Some(doc_id),
            edinet_code: doc.metadata.get("edinet_code").cloned(),
            sec_code: Some(doc.ticker.clone()),
            jcn: doc.metadata.get("jcn").cloned(),
            filer_name: Some(doc.company_name.clone()),
            fund_code: None,
            ordinance_code: doc.metadata.get("ordinance_code").cloned(),
            form_code: doc.metadata.get("form_code").cloned(),
            doc_type_code: doc.metadata.get("doc_type_code").cloned(),
            period_start: doc.metadata.get("period_start").cloned(),
            period_end: doc.metadata.get("period_end").cloned(),
            submit_date: Some(doc.date.format("%Y-%m-%d").to_string()),
            doc_description: doc
                .metadata
                .get("doc_description")
                .or_else(|| doc.metadata.get("description"))
                .cloned(),
            issuer_edinet_code: doc.metadata.get("issuer_edinet_code").cloned(),
            subject_edinet_code: doc.metadata.get("subject_edinet_code").cloned(),
            subsidiary_edinet_code: doc.metadata.get("subsidiary_edinet_code").cloned(),
            current_report_reason: doc.metadata.get("current_report_reason").cloned(),
            parent_doc_id: doc.metadata.get("parent_doc_id").cloned(),
            ope_date_time: doc.metadata.get("ope_date_time").cloned(),
            withdrawal_status: doc.metadata.get("withdrawal_status").cloned(),
            doc_info_edit_status: doc.metadata.get("doc_info_edit_status").cloned(),
            disclosure_request_status: doc.metadata.get("disclosure_request_status").cloned(),
            xbrl_flag: doc.metadata.get("xbrl_flag").cloned(),
            pdf_flag: doc.metadata.get("pdf_flag").cloned(),
            attach_doc_flag: doc.metadata.get("attach_doc_flag").cloned(),
            english_flag: doc.metadata.get("english_flag").cloned(),
            csv_flag: doc.metadata.get("csv_flag").cloned(),
            legal_status: doc.metadata.get("legal_status").cloned(),
        };

        edinet_documents.push(edinet_doc);
    }

    Ok(edinet_documents)
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

    let mut request_builder = client.get(&url).query(&[("type", &"1".to_string())]); // type=1 for ZIP format

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
        assert_eq!(
            parsed.results[0].filer_name.as_deref().unwrap(),
            "Test Company Ltd."
        );
    }

    #[tokio::test]
    async fn test_download_creates_directory_structure() {
        use crate::models::{DocumentFormat, Source};
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

