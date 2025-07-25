//! EDINET-specific error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EdinetError {
    #[error("EDINET API key not configured. Set EDINET_API_KEY environment variable")]
    MissingApiKey,
    
    #[error("Company with ticker '{0}' not found in static database. Run 'edinet load-static' first")]
    CompanyNotFound(String),
    
    #[error("Failed to parse EDINET response for date {date}: {source}")]
    ApiResponseError {
        date: String,
        #[source]
        source: serde_json::Error,
    },
    
    #[error("EDINET API error (status {status_code}): {message}")]
    ApiError {
        status_code: u16,
        message: String,
    },
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid date format: {0}")]
    InvalidDate(#[from] chrono::ParseError),
    
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<anyhow::Error> for EdinetError {
    fn from(err: anyhow::Error) -> Self {
        EdinetError::Config(err.to_string())
    }
}