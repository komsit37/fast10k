//! EDINET indexer interface
//! 
//! This module provides the interface for the main application to access
//! EDINET indexing functionality. The actual implementation is in the `edinet` module.

use crate::edinet;
use anyhow::Result;
use chrono::NaiveDate;

/// Build EDINET index for the specified number of days back from today
pub async fn build_edinet_index(database_path: &str, days_back: i64) -> Result<usize> {
    edinet::indexer::build_edinet_index(database_path, days_back).await
}

/// Build EDINET index for documents between the specified dates (inclusive)
pub async fn build_edinet_index_by_date(
    database_path: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<usize> {
    edinet::indexer::build_edinet_index_by_date(database_path, start_date, end_date).await
}

/// Update EDINET index from the last indexed date to today
pub async fn update_edinet_index(database_path: &str, days_back: i64) -> Result<usize> {
    edinet::indexer::update_edinet_index(database_path, days_back).await
}

/// Get statistics about the EDINET index
pub async fn get_edinet_index_stats(database_path: &str) -> Result<()> {
    edinet::indexer::get_edinet_index_stats(database_path).await
}