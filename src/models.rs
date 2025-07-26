use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub ticker: String,
    pub company_name: String,
    pub filing_type: FilingType,
    pub source: Source,
    pub date: NaiveDate,
    pub content_path: PathBuf,
    pub metadata: HashMap<String, String>,
    pub format: DocumentFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilingType {
    TenK,
    TenQ,
    EightK,
    Transcript,
    PressRelease,
    // EDINET-specific filing types
    AnnualSecuritiesReport,         // 有価証券報告書
    QuarterlySecuritiesReport,      // 四半期報告書  
    SemiAnnualSecuritiesReport,     // 半期報告書
    ExtraordinaryReport,            // 臨時報告書
    Other(String),
}

impl FilingType {
    pub fn as_str(&self) -> &str {
        match self {
            FilingType::TenK => "10-K",
            FilingType::TenQ => "10-Q",
            FilingType::EightK => "8-K",
            FilingType::Transcript => "Transcript",
            FilingType::PressRelease => "Press Release",
            FilingType::AnnualSecuritiesReport => "Annual Securities Report",
            FilingType::QuarterlySecuritiesReport => "Quarterly Securities Report",
            FilingType::SemiAnnualSecuritiesReport => "Semi-Annual Securities Report",
            FilingType::ExtraordinaryReport => "Extraordinary Report",
            FilingType::Other(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Source {
    Edgar,
    Edinet,
    Tdnet,
    Other(String),
}

impl Source {
    pub fn as_str(&self) -> &str {
        match self {
            Source::Edgar => "EDGAR",
            Source::Edinet => "EDINET",
            Source::Tdnet => "TDNet",
            Source::Other(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentFormat {
    Txt,
    Html,
    Xbrl,
    Ixbrl,
    Complete,
    Other(String),
}

impl DocumentFormat {
    pub fn as_str(&self) -> &str {
        match self {
            DocumentFormat::Txt => "txt",
            DocumentFormat::Html => "html",
            DocumentFormat::Xbrl => "xbrl",
            DocumentFormat::Ixbrl => "ixbrl",
            DocumentFormat::Complete => "complete",
            DocumentFormat::Other(s) => s,
        }
    }
    
    pub fn file_extension(&self) -> &str {
        match self {
            DocumentFormat::Txt => "txt",
            DocumentFormat::Html => "htm",
            DocumentFormat::Xbrl => "xml",
            DocumentFormat::Ixbrl => "htm",
            DocumentFormat::Complete => "zip",
            DocumentFormat::Other(_) => "zip", // Default to zip for mixed formats
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub ticker: Option<String>,
    pub company_name: Option<String>,
    pub filing_type: Option<FilingType>,
    pub source: Option<Source>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub text_query: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DownloadRequest {
    pub source: Source,
    pub ticker: String,
    pub filing_type: Option<FilingType>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub limit: usize,
    pub format: DocumentFormat,
}