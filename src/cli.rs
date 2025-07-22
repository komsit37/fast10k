use clap::{Parser, Subcommand};
use chrono::NaiveDate;
use crate::models::{FilingType, Source, DocumentFormat};

#[derive(Parser)]
#[command(name = "fast10k")]
#[command(about = "Fast CLI tool for downloading, indexing, and searching SEC 10-K filings and financial documents")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download documents from specified source
    Download {
        /// Source to download from (edgar, edinet, tdnet)
        #[arg(short, long)]
        source: String,
        
        /// Company ticker symbol
        #[arg(short, long)]
        ticker: String,
        
        /// Filing type to download
        #[arg(short, long)]
        filing_type: Option<String>,
        
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from_date: Option<NaiveDate>,
        
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        to_date: Option<NaiveDate>,
        
        /// Output directory
        #[arg(short, long, default_value = "./downloads")]
        output: String,
        
        /// Maximum number of documents to download
        #[arg(short, long, default_value = "5")]
        limit: usize,
        
        /// Document format to download (txt, html, xbrl, ixbrl, complete)
        #[arg(long, default_value = "txt")]
        format: String,
    },
    
    /// Index downloaded documents into SQLite or Parquet
    Index {
        /// Directory containing downloaded documents
        #[arg(short, long, default_value = "./downloads")]
        input: String,
        
        /// Database file path
        #[arg(short, long, default_value = "./fast10k.db")]
        database: String,
    },
    
    /// Search indexed filings
    Search {
        /// Company ticker symbol
        #[arg(short, long)]
        ticker: Option<String>,
        
        /// Company name
        #[arg(short, long)]
        company: Option<String>,
        
        /// Filing type
        #[arg(short, long)]
        filing_type: Option<String>,
        
        /// Source
        #[arg(short, long)]
        source: Option<String>,
        
        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        from_date: Option<NaiveDate>,
        
        /// End date (YYYY-MM-DD)
        #[arg(long)]
        to_date: Option<NaiveDate>,
        
        /// Text query
        #[arg(short, long)]
        query: Option<String>,
        
        /// Database file path
        #[arg(short, long, default_value = "./fast10k.db")]
        database: String,
        
        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    
    /// Launch terminal UI to monitor downloads & search
    Tui {
        /// Database file path
        #[arg(short, long, default_value = "./fast10k.db")]
        database: String,
    },
}

impl Commands {
    pub fn parse_source(source: &str) -> Result<Source, anyhow::Error> {
        match source.to_lowercase().as_str() {
            "edgar" => Ok(Source::Edgar),
            "edinet" => Ok(Source::Edinet),
            "tdnet" => Ok(Source::Tdnet),
            other => Ok(Source::Other(other.to_string())),
        }
    }
    
    pub fn parse_filing_type(filing_type: &str) -> Result<FilingType, anyhow::Error> {
        match filing_type.to_lowercase().as_str() {
            "10-k" | "10k" => Ok(FilingType::TenK),
            "10-q" | "10q" => Ok(FilingType::TenQ),
            "8-k" | "8k" => Ok(FilingType::EightK),
            "transcript" => Ok(FilingType::Transcript),
            "press-release" | "press_release" => Ok(FilingType::PressRelease),
            other => Ok(FilingType::Other(other.to_string())),
        }
    }
    
    pub fn parse_document_format(format: &str) -> Result<DocumentFormat, anyhow::Error> {
        match format.to_lowercase().as_str() {
            "txt" | "text" => Ok(DocumentFormat::Txt),
            "html" | "htm" => Ok(DocumentFormat::Html),
            "xbrl" | "xml" => Ok(DocumentFormat::Xbrl),
            "ixbrl" | "inline-xbrl" | "inlinexbrl" => Ok(DocumentFormat::Ixbrl),
            "complete" | "all" => Ok(DocumentFormat::Complete),
            other => Err(anyhow::anyhow!("Unsupported document format: {}. Supported formats: txt, html, xbrl, ixbrl, complete", other)),
        }
    }
}