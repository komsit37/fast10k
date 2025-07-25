//! EDINET document downloading functionality

use crate::edinet::{EdinetDocument, EdinetApi, EdinetError, EdinetErrorResponse};
use crate::models::DownloadRequest;
use crate::storage;
use crate::config::Config;
use anyhow::Result;
use reqwest::Client;
use std::path::Path;
use tracing::{debug, info, warn};

/// Download documents from EDINET using the provided request
pub async fn download_documents(request: &DownloadRequest, output_dir: &str) -> Result<usize> {
    let config = Config::from_env()?;
    download_documents_with_config(request, output_dir, &config).await
}

/// Download documents with custom configuration
pub async fn download_documents_with_config(
    request: &DownloadRequest,
    output_dir: &str,
    config: &Config,
) -> Result<usize> {
    info!("Starting EDINET download for ticker: {}", request.ticker);

    let client = Client::builder()
        .user_agent(&config.http.user_agent)
        .timeout(config.http_timeout())
        .build()?;

    // Create output directory structure
    let company_dir = Path::new(output_dir).join("edinet").join(&request.ticker);
    std::fs::create_dir_all(&company_dir)?;

    // Step 1: Search for company by ticker to get EDINET code
    let edinet_code = search_edinet_company(&request.ticker, config).await?;
    info!("Found EDINET code: {} for ticker: {}", edinet_code, request.ticker);

    // Step 2: Get list of available documents from local database
    let documents = get_edinet_documents_from_db(&edinet_code, request, config).await?;
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

        match download_edinet_document(&client, document, &output_path, config).await {
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
        tokio::time::sleep(config.edinet_download_delay()).await;
    }

    info!("Downloaded {} EDINET documents", downloaded_count);
    Ok(downloaded_count)
}

/// Search for EDINET company code by ticker symbol
async fn search_edinet_company(ticker: &str, config: &Config) -> Result<String, EdinetError> {
    debug!("Searching for company with ticker: {}", ticker);

    // Find EDINET code from static database only
    match storage::get_edinet_code_by_securities_code(config.database_path_str(), ticker).await {
        Ok(Some(edinet_code)) => {
            info!(
                "Found EDINET code {} for ticker {} in static database",
                edinet_code, ticker
            );
            Ok(edinet_code)
        }
        Ok(None) => Err(EdinetError::CompanyNotFound(ticker.to_string())),
        Err(e) => Err(EdinetError::Config(e.to_string())),
    }
}

/// Get EDINET documents from local database
async fn get_edinet_documents_from_db(
    _edinet_code: &str,
    request: &DownloadRequest,
    config: &Config,
) -> Result<Vec<EdinetDocument>, EdinetError> {
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
    let documents = storage::search_documents(
        &search_query,
        config.database_path_str(),
        request.limit,
    )
    .await
    .map_err(|e| EdinetError::Config(e.to_string()))?;
    
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

/// Download a single EDINET document
async fn download_edinet_document(
    client: &Client,
    document: &EdinetDocument,
    output_path: &Path,
    config: &Config,
) -> Result<(), EdinetError> {
    let api_key = config.edinet_api_key.as_ref().ok_or(EdinetError::MissingApiKey)?;

    let url = format!(
        "{}{}/{}",
        EdinetApi::BASE_URL,
        EdinetApi::DOCUMENT_DOWNLOAD_ENDPOINT,
        document.doc_id.as_deref().unwrap_or("unknown")
    );

    debug!("Downloading document from: {}", url);

    let response = client
        .get(&url)
        .query(&[("type", "1")]) // type=1 for ZIP format
        .header("Ocp-Apim-Subscription-Key", api_key)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        let response_text = response.text().await?;
        if let Ok(error_response) = serde_json::from_str::<EdinetErrorResponse>(&response_text) {
            return Err(EdinetError::ApiError {
                status_code: error_response.status_code,
                message: error_response.message,
            });
        } else {
            return Err(EdinetError::ApiError {
                status_code: status.as_u16(),
                message: response_text,
            });
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