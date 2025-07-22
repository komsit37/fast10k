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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilingType {
    TenK,
    TenQ,
    EightK,
    Transcript,
    PressRelease,
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
}