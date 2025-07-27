//! Core traits for the EDINET TUI architecture
//!
//! This module defines the foundational traits that enable code reuse and
//! consistent behavior across all TUI screens.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{layout::Rect, Frame};

use crate::edinet_tui::app::Screen as ScreenType;

/// Actions that can be returned from screen event handling
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenAction {
    /// Navigate to a different screen
    NavigateTo(ScreenType),
    /// Go back to previous screen
    NavigateBack,
    /// Quit the application
    Quit,
    /// Set status message
    SetStatus(String),
    /// Set error message
    SetError(String),
    /// Clear messages
    ClearMessages,
    /// No action taken
    None,
}

/// Core trait for all TUI screens
pub trait Screen {
    /// Draw the screen content
    fn draw(&mut self, f: &mut Frame, area: Rect);
    
    /// Handle keyboard input and return an optional action
    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<ScreenAction>;
    
    /// Get the screen type identifier
    fn screen_type(&self) -> ScreenType;
    
    /// Whether this screen supports navigation back
    fn can_navigate_back(&self) -> bool {
        true
    }
    
    /// Called when screen becomes active
    fn on_enter(&mut self) {}
    
    /// Called when screen becomes inactive
    fn on_exit(&mut self) {}
    
    /// Called to refresh screen data
    async fn refresh(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Trait for screens with navigable lists
pub trait Navigable {
    /// Move selection up
    fn navigate_up(&mut self);
    
    /// Move selection down
    fn navigate_down(&mut self);
    
    /// Get currently selected index
    fn get_selected_index(&self) -> Option<usize>;
    
    /// Set selected index
    fn set_selected_index(&mut self, index: Option<usize>);
    
    /// Get total number of items
    fn get_item_count(&self) -> usize;
    
    /// Navigate to first item
    fn navigate_to_first(&mut self) {
        if self.get_item_count() > 0 {
            self.set_selected_index(Some(0));
        }
    }
    
    /// Navigate to last item
    fn navigate_to_last(&mut self) {
        let count = self.get_item_count();
        if count > 0 {
            self.set_selected_index(Some(count - 1));
        }
    }
}

/// Trait for screens with scrollable content
pub trait Scrollable {
    /// Scroll up by given amount
    fn scroll_up(&mut self, amount: usize);
    
    /// Scroll down by given amount
    fn scroll_down(&mut self, amount: usize);
    
    /// Get current scroll offset
    fn get_scroll_offset(&self) -> usize;
    
    /// Set scroll offset with bounds checking
    fn set_scroll_offset(&mut self, offset: usize);
    
    /// Calculate maximum scroll offset
    fn calculate_max_scroll(&self) -> usize;
    
    /// Scroll to top
    fn scroll_to_top(&mut self) {
        self.set_scroll_offset(0);
    }
    
    /// Scroll to bottom
    fn scroll_to_bottom(&mut self) {
        let max_scroll = self.calculate_max_scroll();
        self.set_scroll_offset(max_scroll);
    }
    
    /// Page up (scroll up by page size)
    fn page_up(&mut self) {
        let page_size = self.get_page_size();
        let current = self.get_scroll_offset();
        self.set_scroll_offset(current.saturating_sub(page_size));
    }
    
    /// Page down (scroll down by page size)
    fn page_down(&mut self) {
        let page_size = self.get_page_size();
        let current = self.get_scroll_offset();
        let max_scroll = self.calculate_max_scroll();
        self.set_scroll_offset(std::cmp::min(current + page_size, max_scroll));
    }
    
    /// Get page size for scrolling
    fn get_page_size(&self) -> usize {
        20 // Default page size
    }
}

/// Trait for screens with paginated content
pub trait Paginated {
    /// Get current page number (0-based)
    fn get_current_page(&self) -> usize;
    
    /// Set current page
    fn set_current_page(&mut self, page: usize);
    
    /// Get total number of pages
    fn get_total_pages(&self) -> usize;
    
    /// Get items per page
    fn get_items_per_page(&self) -> usize;
    
    /// Go to next page
    fn next_page(&mut self) {
        let current = self.get_current_page();
        let total = self.get_total_pages();
        if current + 1 < total {
            self.set_current_page(current + 1);
        }
    }
    
    /// Go to previous page
    fn previous_page(&mut self) {
        let current = self.get_current_page();
        if current > 0 {
            self.set_current_page(current - 1);
        }
    }
    
    /// Go to first page
    fn go_to_first_page(&mut self) {
        self.set_current_page(0);
    }
    
    /// Go to last page
    fn go_to_last_page(&mut self) {
        let total = self.get_total_pages();
        if total > 0 {
            self.set_current_page(total - 1);
        }
    }
}

/// Trait for screens that can handle async operations
#[async_trait::async_trait]
pub trait AsyncOperationHandler {
    /// Start an async operation
    async fn start_operation(&mut self, operation: AsyncOperation) -> Result<()>;
    
    /// Cancel current operation
    fn cancel_operation(&mut self);
    
    /// Check if operation is in progress
    fn is_operation_in_progress(&self) -> bool;
    
    /// Get operation status message
    fn get_operation_status(&self) -> Option<String>;
}

/// Types of async operations
#[derive(Debug, Clone)]
pub enum AsyncOperation {
    Download { document_id: String, ticker: String },
    Search { query: crate::models::SearchQuery },
    LoadContent { document_id: String },
    DatabaseUpdate,
    DatabaseBuild { from: chrono::NaiveDate, to: chrono::NaiveDate },
}

/// Trait for form handling
pub trait FormHandler {
    /// Get current field index
    fn get_current_field(&self) -> usize;
    
    /// Set current field
    fn set_current_field(&mut self, field: usize);
    
    /// Get total number of fields
    fn get_field_count(&self) -> usize;
    
    /// Move to next field
    fn next_field(&mut self) {
        let current = self.get_current_field();
        let total = self.get_field_count();
        self.set_current_field((current + 1) % total);
    }
    
    /// Move to previous field
    fn previous_field(&mut self) {
        let current = self.get_current_field();
        let total = self.get_field_count();
        self.set_current_field(if current == 0 { total - 1 } else { current - 1 });
    }
    
    /// Handle character input for current field
    fn handle_char_input(&mut self, c: char);
    
    /// Handle backspace for current field
    fn handle_backspace(&mut self);
    
    /// Handle delete for current field
    fn handle_delete(&mut self);
    
    /// Validate form data
    fn validate(&self) -> Result<(), String>;
    
    /// Submit form
    async fn submit(&mut self) -> Result<ScreenAction>;
}