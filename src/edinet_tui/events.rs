//! Event handling for the EDINET TUI

use crossterm::event::KeyEvent;
use crate::models::Document;

/// Application events that can be triggered from various screens
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Quit the application
    Quit,
    /// Navigate to a specific screen
    NavigateToScreen(super::app::Screen),
    /// Show status message
    ShowStatus(String),
    /// Show error message
    ShowError(String),
    /// Clear messages
    ClearMessages,
    
    // Database events
    /// Database operation completed
    DatabaseOperationComplete(String),
    /// Database operation failed
    DatabaseOperationFailed(String),
    
    // Search events
    /// Search completed with results
    SearchComplete(Vec<Document>),
    /// Search failed
    SearchFailed(String),
    
    // Document events
    /// Document selected for viewing
    DocumentSelected(Document),
    /// Document download started
    DocumentDownloadStarted(String),
    /// Document download completed
    DocumentDownloadComplete(String),
    /// Document download failed
    DocumentDownloadFailed(String),
}

/// Trait for screens that can handle events
pub trait EventHandler {
    /// Handle a key event and optionally return an app event
    async fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<Option<AppEvent>>;
}