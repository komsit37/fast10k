//! EDINET (Japan Financial Services Agency) module
//! 
//! This module provides functionality for working with EDINET, Japan's electronic 
//! disclosure system for financial documents. It includes document indexing, 
//! searching, and downloading capabilities.

pub mod types;
pub mod indexer;
pub mod downloader;
pub mod errors;
pub mod reader;

pub use types::*;
pub use errors::EdinetError;

// Re-export commonly used functions
pub use indexer::{
    build_edinet_index,
    build_edinet_index_by_date,
    update_edinet_index,
    get_edinet_index_stats,
};

pub use downloader::download_documents;
pub use reader::{read_edinet_zip, DocumentSection};