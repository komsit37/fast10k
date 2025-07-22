use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, error, info, warn};
use crate::models::DownloadRequest;

#[derive(Debug, Deserialize)]
struct CompanyTicker {
    pub cik_str: u64,
    pub ticker: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
struct CompanySubmissions {
    pub cik: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub sic: String,
    #[serde(rename = "sicDescription")]
    pub sic_description: String,
    #[serde(rename = "insiderTransactionForOwnerExists")]
    pub insider_transaction_for_owner_exists: u32,
    #[serde(rename = "insiderTransactionForIssuerExists")]
    pub insider_transaction_for_issuer_exists: u32,
    pub name: String,
    pub tickers: Vec<String>,
    pub exchanges: Vec<String>,
    pub ein: String,
    pub description: String,
    pub website: String,
    #[serde(rename = "investorWebsite")]
    pub investor_website: String,
    pub category: String,
    #[serde(rename = "fiscalYearEnd")]
    pub fiscal_year_end: String,
    #[serde(rename = "stateOfIncorporation")]
    pub state_of_incorporation: String,
    #[serde(rename = "stateOfIncorporationDescription")]
    pub state_of_incorporation_description: String,
    pub addresses: serde_json::Value,
    #[serde(rename = "phoneNumber")]
    pub phone_number: Option<String>,
    pub flags: Option<String>,
    #[serde(rename = "formerNames")]
    pub former_names: Vec<serde_json::Value>,
    pub filings: FilingsData,
}

#[derive(Debug, Deserialize)]
struct FilingsData {
    pub recent: RecentFilings,
    pub files: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct RecentFilings {
    #[serde(rename = "accessionNumber")]
    pub accession_number: Vec<String>,
    #[serde(rename = "filingDate")]
    pub filing_date: Vec<String>,
    #[serde(rename = "reportDate")]
    pub report_date: Vec<String>,
    #[serde(rename = "acceptanceDateTime")]
    pub acceptance_date_time: Vec<String>,
    pub act: Vec<String>,
    pub form: Vec<String>,
    #[serde(rename = "fileNumber")]
    pub file_number: Vec<String>,
    #[serde(rename = "filmNumber")]
    pub film_number: Vec<String>,
    pub items: Vec<String>,
    pub size: Vec<u64>,
    #[serde(rename = "isXBRL")]
    pub is_xbrl: Vec<u32>,
    #[serde(rename = "isInlineXBRL")]
    pub is_inline_xbrl: Vec<u32>,
    #[serde(rename = "primaryDocument")]
    pub primary_document: Vec<String>,
    #[serde(rename = "primaryDocDescription")]
    pub primary_doc_description: Vec<String>,
}

#[derive(Debug)]
struct FilingEntry {
    pub accession_number: String,
    pub filing_date: String,
    pub report_date: String,
    pub form: String,
    pub primary_document: String,
    pub primary_doc_description: String,
}

pub async fn download(request: &DownloadRequest, output_dir: &str) -> Result<usize> {
    info!("Starting EDGAR download for ticker: {}", request.ticker);
    
    let client = Client::builder()
        .user_agent("fast10k/0.1.0 (your.email@example.com)")
        .build()?;
    
    // Step 1: Find CIK for the ticker
    let cik = search_company_by_ticker(&client, &request.ticker).await?;
    info!("Found CIK {} for ticker {}", cik, request.ticker);
    
    // Step 2: Get company filings
    let filings = get_company_filings(&client, &cik).await?;
    info!("Found {} filings for CIK {}", filings.len(), cik);
    
    let company_dir = Path::new(output_dir).join("edgar").join(&request.ticker);
    fs::create_dir_all(&company_dir).await?;
    
    let mut download_count = 0;
    
    // Step 3: Download matching filings (limited by request.limit)
    for filing in filings {
        // Stop if we've reached the download limit
        if download_count >= request.limit {
            break;
        }
        // Filter by filing type if specified
        if let Some(ref filing_type) = request.filing_type {
            if !matches_filing_type(&filing.form, filing_type) {
                continue;
            }
        }
        
        // Filter by date range if specified
        if let Some(date_from) = request.date_from {
            let filing_date = chrono::NaiveDate::parse_from_str(&filing.filing_date, "%Y-%m-%d")?;
            if filing_date < date_from {
                continue;
            }
        }
        
        if let Some(date_to) = request.date_to {
            let filing_date = chrono::NaiveDate::parse_from_str(&filing.filing_date, "%Y-%m-%d")?;
            if filing_date > date_to {
                continue;
            }
        }
        
        let filename = format!("{}-{}-{}.txt", 
            filing.form.replace("/", "-"), 
            filing.filing_date, 
            filing.accession_number.replace("-", ""));
        let file_path = company_dir.join(filename);
        
        match download_filing(&client, &filing.accession_number, &file_path).await {
            Ok(_) => {
                info!("Downloaded filing: {}", file_path.display());
                download_count += 1;
            }
            Err(e) => {
                warn!("Failed to download filing {}: {}", filing.accession_number, e);
            }
        }
    }
    
    info!("Downloaded {} filings for ticker {}", download_count, request.ticker);
    Ok(download_count)
}

fn matches_filing_type(form: &str, filing_type: &crate::models::FilingType) -> bool {
    use crate::models::FilingType;
    match filing_type {
        FilingType::TenK => form.starts_with("10-K"),
        FilingType::TenQ => form.starts_with("10-Q"),
        FilingType::EightK => form.starts_with("8-K"),
        FilingType::Other(form_type) => form == form_type,
        _ => false,
    }
}

async fn search_company_by_ticker(client: &Client, ticker: &str) -> Result<String> {
    let url = "https://www.sec.gov/files/company_tickers.json";
    
    debug!("Fetching company tickers from: {}", url);
    let response = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch company tickers: HTTP {}", response.status()));
    }
    
    let tickers: HashMap<String, CompanyTicker> = response.json().await?;
    
    // Search for matching ticker (case-insensitive)
    let ticker_upper = ticker.to_uppercase();
    for company in tickers.values() {
        if company.ticker.to_uppercase() == ticker_upper {
            // Pad CIK to 10 digits with leading zeros
            let cik = format!("{:0>10}", company.cik_str);
            return Ok(cik);
        }
    }
    
    Err(anyhow!("Ticker {} not found in EDGAR database", ticker))
}

async fn get_company_filings(client: &Client, cik: &str) -> Result<Vec<FilingEntry>> {
    let url = format!("https://data.sec.gov/submissions/CIK{}.json", cik);
    
    debug!("Fetching company submissions from: {}", url);
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch company submissions: HTTP {}", response.status()));
    }
    
    let submissions: CompanySubmissions = response.json().await?;
    let recent = &submissions.filings.recent;
    
    let mut filings = Vec::new();
    
    // Combine all the parallel arrays into FilingEntry structs
    let len = recent.accession_number.len();
    for i in 0..len {
        filings.push(FilingEntry {
            accession_number: recent.accession_number[i].clone(),
            filing_date: recent.filing_date[i].clone(),
            report_date: recent.report_date.get(i).cloned().unwrap_or_default(),
            form: recent.form[i].clone(),
            primary_document: recent.primary_document.get(i).cloned().unwrap_or_default(),
            primary_doc_description: recent.primary_doc_description.get(i).cloned().unwrap_or_default(),
        });
    }
    
    info!("Retrieved {} recent filings for CIK {}", filings.len(), cik);
    Ok(filings)
}

async fn download_filing(client: &Client, accession_number: &str, output_path: &Path) -> Result<()> {
    // Format the accession number for the URL (remove dashes)
    let accession_clean = accession_number.replace("-", "");
    
    // Extract CIK from accession number (first 10 digits)
    if accession_clean.len() < 10 {
        return Err(anyhow!("Invalid accession number format: {}", accession_number));
    }
    
    let cik = &accession_clean[0..10];
    let cik_num = cik.parse::<u64>()
        .map_err(|_| anyhow!("Invalid CIK in accession number: {}", accession_number))?;
    
    // EDGAR filing URLs follow the pattern:
    // https://www.sec.gov/Archives/edgar/data/{CIK}/{accession_clean}/{primary_document}
    let base_url = format!(
        "https://www.sec.gov/Archives/edgar/data/{}/{}",
        cik_num, // Use numeric CIK without leading zeros for URL
        accession_clean
    );
    
    // Try different document name patterns with retry logic
    let document_urls = vec![
        format!("{}/{}.txt", base_url, accession_number),
        format!("{}/{}-index.html", base_url, accession_number),
        format!("{}/filing-details.html", base_url),
    ];
    
    for url in document_urls {
        for attempt in 1..=3 {
            debug!("Attempting to download from: {} (attempt {})", url, attempt);
            
            let response = match client
                .get(&url)
                .header("Accept", "text/html,text/plain,*/*")
                .header("User-Agent", "fast10k/0.1.0 (your.email@example.com)")
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    warn!("Request failed for {} (attempt {}): {}", url, attempt, e);
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_millis(1000 * attempt as u64)).await;
                        continue;
                    } else {
                        break;
                    }
                }
            };
            
            if response.status().is_success() {
                match response.text().await {
                    Ok(content) => {
                        if let Err(e) = fs::write(output_path, content).await {
                            error!("Failed to write file {}: {}", output_path.display(), e);
                            return Err(anyhow!("Failed to write downloaded content: {}", e));
                        }
                        info!("Successfully downloaded filing to: {}", output_path.display());
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Failed to read response content: {}", e);
                        if attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_millis(1000 * attempt as u64)).await;
                            continue;
                        }
                    }
                }
            } else if response.status().as_u16() == 429 {
                // Rate limited - wait longer before retry
                warn!("Rate limited, waiting before retry...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            } else {
                debug!("HTTP {} for URL: {}", response.status(), url);
                break; // Try next URL
            }
        }
    }
    
    Err(anyhow!("Failed to download filing {} from any attempted URL after retries", accession_number))
}