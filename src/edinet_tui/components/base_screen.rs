//! Base screen implementation providing common functionality

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, widgets::ListState, Frame};

use crate::edinet_tui::{
    traits::{Navigable, Paginated, ScreenAction, Scrollable},
    ui::Styles,
};

/// Generic screen state container
#[derive(Debug)]
pub struct ScreenState<T> {
    pub data: T,
    pub list_state: ListState,
    pub scroll_offset: usize,
    pub current_page: usize,
    pub items_per_page: usize,
    pub is_loading: bool,
    pub error_message: Option<String>,
    pub status_message: Option<String>,
}

impl<T> ScreenState<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            list_state: ListState::default(),
            scroll_offset: 0,
            current_page: 0,
            items_per_page: 20,
            is_loading: false,
            error_message: None,
            status_message: None,
        }
    }

    pub fn with_items_per_page(mut self, items_per_page: usize) -> Self {
        self.items_per_page = items_per_page;
        self
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
    }

    pub fn set_error(&mut self, error: Option<String>) {
        let has_error = error.is_some();
        self.error_message = error;
        if has_error {
            self.status_message = None;
        }
    }

    pub fn set_status(&mut self, status: Option<String>) {
        let has_status = status.is_some();
        self.status_message = status;
        if has_status {
            self.error_message = None;
        }
    }

    pub fn clear_messages(&mut self) {
        self.error_message = None;
        self.status_message = None;
    }
}

/// Base screen implementation with common functionality
pub struct BaseScreen<T> {
    pub state: ScreenState<T>,
    pub screen_type: crate::edinet_tui::app::Screen,
}

impl<T> BaseScreen<T> {
    pub fn new(data: T, screen_type: crate::edinet_tui::app::Screen) -> Self {
        Self {
            state: ScreenState::new(data),
            screen_type,
        }
    }

    pub fn with_pagination(mut self, items_per_page: usize) -> Self {
        self.state = self.state.with_items_per_page(items_per_page);
        self
    }
}

/// Implementation for list-based screens
impl<T> Navigable for BaseScreen<Vec<T>> {
    fn navigate_up(&mut self) {
        if self.state.data.is_empty() {
            return;
        }
        let selected = self.state.list_state.selected().unwrap_or(0);
        let new_selected = if selected == 0 {
            self.state.data.len() - 1
        } else {
            selected - 1
        };
        self.state.list_state.select(Some(new_selected));
    }

    fn navigate_down(&mut self) {
        if self.state.data.is_empty() {
            return;
        }
        let selected = self.state.list_state.selected().unwrap_or(0);
        let new_selected = (selected + 1) % self.state.data.len();
        self.state.list_state.select(Some(new_selected));
    }

    fn get_selected_index(&self) -> Option<usize> {
        self.state.list_state.selected()
    }

    fn set_selected_index(&mut self, index: Option<usize>) {
        self.state.list_state.select(index);
    }

    fn get_item_count(&self) -> usize {
        self.state.data.len()
    }
}

/// Implementation for paginated screens
impl<T> Paginated for BaseScreen<Vec<T>> {
    fn get_current_page(&self) -> usize {
        self.state.current_page
    }

    fn set_current_page(&mut self, page: usize) {
        let total_pages = self.get_total_pages();
        if page < total_pages {
            self.state.current_page = page;
            // Reset selection to first item on new page
            self.state.list_state.select(if self.state.data.is_empty() {
                None
            } else {
                Some(0)
            });
        }
    }

    fn get_total_pages(&self) -> usize {
        if self.state.data.is_empty() {
            1
        } else {
            (self.state.data.len() + self.state.items_per_page - 1) / self.state.items_per_page
        }
    }

    fn get_items_per_page(&self) -> usize {
        self.state.items_per_page
    }
}

/// Implementation for scrollable content
impl<T> Scrollable for BaseScreen<T> {
    fn scroll_up(&mut self, amount: usize) {
        self.state.scroll_offset = self.state.scroll_offset.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize) {
        let max_scroll = self.calculate_max_scroll();
        self.state.scroll_offset = std::cmp::min(self.state.scroll_offset + amount, max_scroll);
    }

    fn get_scroll_offset(&self) -> usize {
        self.state.scroll_offset
    }

    fn set_scroll_offset(&mut self, offset: usize) {
        let max_scroll = self.calculate_max_scroll();
        self.state.scroll_offset = std::cmp::min(offset, max_scroll);
    }

    fn calculate_max_scroll(&self) -> usize {
        // Default implementation - screens can override this
        0
    }
}

/// Helper methods for working with the current page's data
impl<T> BaseScreen<Vec<T>> {
    /// Get items for the current page
    pub fn get_current_page_items(&self) -> &[T] {
        let start_idx = self.state.current_page * self.state.items_per_page;
        let end_idx = std::cmp::min(start_idx + self.state.items_per_page, self.state.data.len());

        if start_idx < self.state.data.len() {
            &self.state.data[start_idx..end_idx]
        } else {
            &[]
        }
    }

    /// Get the currently selected item from the current page
    pub fn get_selected_item(&self) -> Option<&T> {
        if let Some(selected_idx) = self.state.list_state.selected() {
            let page_start = self.state.current_page * self.state.items_per_page;
            self.state.data.get(page_start + selected_idx)
        } else {
            None
        }
    }

    /// Set new data and reset pagination/selection
    pub fn set_data(&mut self, data: Vec<T>) {
        self.state.data = data;
        self.state.current_page = 0;
        self.state.list_state.select(if self.state.data.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    /// Add item to data
    pub fn add_item(&mut self, item: T) {
        self.state.data.push(item);
        // If this is the first item, select it
        if self.state.data.len() == 1 {
            self.state.list_state.select(Some(0));
        }
    }

    /// Clear all data
    pub fn clear_data(&mut self) {
        self.state.data.clear();
        self.state.current_page = 0;
        self.state.list_state.select(None);
    }
}

/// Trait for customizing base screen behavior
pub trait ScreenCustomization<T> {
    /// Custom draw implementation
    fn draw_content(&mut self, f: &mut Frame, area: Rect, data: &T);
    
    /// Custom key handling
    async fn handle_custom_key(&mut self, key: KeyEvent, data: &mut T) -> Result<Option<ScreenAction>>;
    
    /// Calculate content-specific max scroll
    fn calculate_content_max_scroll(&self, data: &T) -> usize {
        0
    }
}