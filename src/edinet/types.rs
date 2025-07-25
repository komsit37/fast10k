//! Shared EDINET types and data structures

use serde::Deserialize;

/// EDINET API response containing metadata and document results
#[derive(Debug, Deserialize)]
pub struct EdinetIndexResponse {
    /// Optional metadata about the response
    pub metadata: Option<EdinetMetaData>,
    /// List of documents in the response
    pub results: Vec<EdinetDocument>,
}

/// Metadata information for EDINET API responses
#[derive(Debug, Deserialize)]
pub struct EdinetMetaData {
    /// Response title
    pub title: String,
    /// Request parameters
    pub parameter: EdinetParameter,
    /// Result set information
    pub resultset: EdinetResultSet,
}

/// Parameters used in the EDINET API request
#[derive(Debug, Deserialize)]
pub struct EdinetParameter {
    /// Date parameter
    pub date: String,
    /// Document type parameter
    #[serde(rename = "type")]
    pub doc_type: Option<String>,
}

/// Information about the result set
#[derive(Debug, Deserialize)]
pub struct EdinetResultSet {
    /// Number of results
    pub count: i32,
}

/// Individual EDINET document metadata
#[derive(Debug, Deserialize, Clone)]
pub struct EdinetDocument {
    /// Sequence number in the response
    #[serde(rename = "seqNumber")]
    pub seq_number: i32,
    
    /// Document ID - required for downloading
    #[serde(rename = "docID")]
    pub doc_id: Option<String>,
    
    /// EDINET code of the company
    #[serde(rename = "edinetCode")]
    pub edinet_code: Option<String>,
    
    /// Securities code (ticker symbol)
    #[serde(rename = "secCode")]
    pub sec_code: Option<String>,
    
    /// Japanese Corporate Number
    #[serde(rename = "JCN")]
    pub jcn: Option<String>,
    
    /// Company name (filer name)
    #[serde(rename = "filerName")]
    pub filer_name: Option<String>,
    
    /// Fund code
    #[serde(rename = "fundCode")]
    pub fund_code: Option<String>,
    
    /// Ordinance code
    #[serde(rename = "ordinanceCode")]
    pub ordinance_code: Option<String>,
    
    /// Form code (document type identifier)
    #[serde(rename = "formCode")]
    pub form_code: Option<String>,
    
    /// Document type code
    #[serde(rename = "docTypeCode")]
    pub doc_type_code: Option<String>,
    
    /// Reporting period start date
    #[serde(rename = "periodStart")]
    pub period_start: Option<String>,
    
    /// Reporting period end date
    #[serde(rename = "periodEnd")]
    pub period_end: Option<String>,
    
    /// Document submission date and time
    #[serde(rename = "submitDateTime")]
    pub submit_date: Option<String>,
    
    /// Document description
    #[serde(rename = "docDescription")]
    pub doc_description: Option<String>,
    
    /// Issuer EDINET code
    #[serde(rename = "issuerEdinetCode")]
    pub issuer_edinet_code: Option<String>,
    
    /// Subject EDINET code
    #[serde(rename = "subjectEdinetCode")]
    pub subject_edinet_code: Option<String>,
    
    /// Subsidiary EDINET code
    #[serde(rename = "subsidiaryEdinetCode")]
    pub subsidiary_edinet_code: Option<String>,
    
    /// Current report reason
    #[serde(rename = "currentReportReason")]
    pub current_report_reason: Option<String>,
    
    /// Parent document ID
    #[serde(rename = "parentDocID")]
    pub parent_doc_id: Option<String>,
    
    /// Operation date and time
    #[serde(rename = "opeDateTime")]
    pub ope_date_time: Option<String>,
    
    /// Withdrawal status
    #[serde(rename = "withdrawalStatus")]
    pub withdrawal_status: Option<String>,
    
    /// Document info edit status
    #[serde(rename = "docInfoEditStatus")]
    pub doc_info_edit_status: Option<String>,
    
    /// Disclosure request status
    #[serde(rename = "disclosureStatus")]
    pub disclosure_request_status: Option<String>,
    
    /// XBRL flag
    #[serde(rename = "xbrlFlag")]
    pub xbrl_flag: Option<String>,
    
    /// PDF flag
    #[serde(rename = "pdfFlag")]
    pub pdf_flag: Option<String>,
    
    /// Attached document flag
    #[serde(rename = "attachDocFlag")]
    pub attach_doc_flag: Option<String>,
    
    /// English document flag
    #[serde(rename = "englishDocFlag")]
    pub english_flag: Option<String>,
    
    /// CSV flag
    #[serde(rename = "csvFlag", default)]
    pub csv_flag: Option<String>,
    
    /// Legal status
    #[serde(rename = "legalStatus", default)]
    pub legal_status: Option<String>,
}

/// EDINET API error response structure
#[derive(Debug, Deserialize)]
pub struct EdinetErrorResponse {
    /// HTTP status code
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    /// Error message
    pub message: String,
}

/// EDINET API endpoints and constants
pub struct EdinetApi;

impl EdinetApi {
    /// Base URL for EDINET API
    pub const BASE_URL: &'static str = "https://api.edinet-fsa.go.jp";
    /// Documents listing endpoint
    pub const DOCUMENTS_ENDPOINT: &'static str = "/api/v2/documents.json";
    /// Document download endpoint (without document ID)
    pub const DOCUMENT_DOWNLOAD_ENDPOINT: &'static str = "/api/v2/documents";
}