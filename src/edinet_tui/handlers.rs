//! Common event handlers for the EDINET TUI
//!
//! This module provides reusable event handling logic that can be composed
//! across different screens to eliminate code duplication.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::traits::{Navigable, Scrollable, Paginated, FormHandler, ScreenAction};
use crate::edinet_tui::app::Screen;

/// Common keyboard event handling utilities
pub struct CommonKeyHandler;

impl CommonKeyHandler {
    /// Handle navigation keys for list-based screens
    pub fn handle_navigation_keys<T: Navigable>(
        navigable: &mut T,
        key: KeyEvent,
    ) -> Option<ScreenAction> {
        match key.code {
            KeyCode::Up => {
                navigable.navigate_up();
                Some(ScreenAction::None)
            }
            KeyCode::Down => {
                navigable.navigate_down();
                Some(ScreenAction::None)
            }
            KeyCode::Home => {
                navigable.navigate_to_first();
                Some(ScreenAction::SetStatus("First item".to_string()))
            }
            KeyCode::End => {
                navigable.navigate_to_last();
                Some(ScreenAction::SetStatus("Last item".to_string()))
            }
            _ => None,
        }
    }

    /// Handle scrolling keys for content-based screens
    pub fn handle_scroll_keys<T: Scrollable>(
        scrollable: &mut T,
        key: KeyEvent,
    ) -> Option<ScreenAction> {
        match key.code {
            KeyCode::Up => {
                scrollable.scroll_up(1);
                Some(ScreenAction::None)
            }
            KeyCode::Down => {
                scrollable.scroll_down(1);
                Some(ScreenAction::None)
            }
            KeyCode::PageUp => {
                scrollable.page_up();
                Some(ScreenAction::SetStatus("Page up".to_string()))
            }
            KeyCode::PageDown => {
                scrollable.page_down();
                Some(ScreenAction::SetStatus("Page down".to_string()))
            }
            KeyCode::Home => {
                scrollable.scroll_to_top();
                Some(ScreenAction::SetStatus("Top of content".to_string()))
            }
            KeyCode::End => {
                scrollable.scroll_to_bottom();
                Some(ScreenAction::SetStatus("Bottom of content".to_string()))
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                scrollable.page_up();
                Some(ScreenAction::SetStatus("Scroll up one page".to_string()))
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                scrollable.page_down();
                Some(ScreenAction::SetStatus("Scroll down one page".to_string()))
            }
            _ => None,
        }
    }

    /// Handle pagination keys
    pub fn handle_pagination_keys<T: Paginated>(
        paginated: &mut T,
        key: KeyEvent,
    ) -> Option<ScreenAction> {
        match key.code {
            KeyCode::Left | KeyCode::PageUp => {
                paginated.previous_page();
                Some(ScreenAction::SetStatus("Previous page".to_string()))
            }
            KeyCode::Right | KeyCode::PageDown => {
                paginated.next_page();
                Some(ScreenAction::SetStatus("Next page".to_string()))
            }
            KeyCode::Home => {
                paginated.go_to_first_page();
                Some(ScreenAction::SetStatus("First page".to_string()))
            }
            KeyCode::End => {
                paginated.go_to_last_page();
                Some(ScreenAction::SetStatus("Last page".to_string()))
            }
            _ => None,
        }
    }

    /// Handle form navigation and input
    pub fn handle_form_keys<T: FormHandler>(
        form: &mut T,
        key: KeyEvent,
    ) -> Option<ScreenAction> {
        match key.code {
            KeyCode::Tab => {
                form.next_field();
                Some(ScreenAction::SetStatus(format!(
                    "Field {}/{}",
                    form.get_current_field() + 1,
                    form.get_field_count()
                )))
            }
            KeyCode::BackTab => {
                form.previous_field();
                Some(ScreenAction::SetStatus(format!(
                    "Field {}/{}",
                    form.get_current_field() + 1,
                    form.get_field_count()
                )))
            }
            KeyCode::Char(c) => {
                form.handle_char_input(c);
                Some(ScreenAction::None)
            }
            KeyCode::Backspace => {
                form.handle_backspace();
                Some(ScreenAction::None)
            }
            KeyCode::Delete => {
                form.handle_delete();
                Some(ScreenAction::None)
            }
            _ => None,
        }
    }

    /// Handle global application keys
    pub fn handle_global_keys(key: KeyEvent) -> Option<ScreenAction> {
        match key.code {
            KeyCode::Char('q') => Some(ScreenAction::Quit),
            KeyCode::F(1) | KeyCode::Char('?') => {
                // Help will be handled by the app
                None
            }
            KeyCode::Esc => Some(ScreenAction::NavigateBack),
            _ => None,
        }
    }

    /// Handle vim-like movement keys
    pub fn handle_vim_keys<T: Scrollable>(
        scrollable: &mut T,
        key: KeyEvent,
        pending_g: &mut bool,
    ) -> Option<ScreenAction> {
        match key.code {
            KeyCode::Char('j') => {
                scrollable.scroll_down(1);
                Some(ScreenAction::None)
            }
            KeyCode::Char('k') => {
                scrollable.scroll_up(1);
                Some(ScreenAction::None)
            }
            KeyCode::Char('g') => {
                if *pending_g {
                    scrollable.scroll_to_top();
                    *pending_g = false;
                    Some(ScreenAction::SetStatus("Top of content".to_string()))
                } else {
                    *pending_g = true;
                    Some(ScreenAction::SetStatus("Press 'g' again to go to top".to_string()))
                }
            }
            KeyCode::Char('G') => {
                scrollable.scroll_to_bottom();
                *pending_g = false;
                Some(ScreenAction::SetStatus("Bottom of content".to_string()))
            }
            _ => {
                if *pending_g {
                    *pending_g = false;
                    Some(ScreenAction::SetStatus("Command cancelled".to_string()))
                } else {
                    None
                }
            }
        }
    }
}

/// Specialized handler for menu-style screens
pub struct MenuHandler;

impl MenuHandler {
    /// Handle menu selection with Enter key
    pub fn handle_menu_selection<T: Navigable>(
        navigable: &T,
        key: KeyEvent,
        menu_actions: &[ScreenAction],
    ) -> Option<ScreenAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(selected) = navigable.get_selected_index() {
                    menu_actions.get(selected).cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Handle menu shortcuts (character keys)
    pub fn handle_menu_shortcuts(
        key: KeyEvent,
        shortcuts: &[(char, ScreenAction)],
    ) -> Option<ScreenAction> {
        if let KeyCode::Char(c) = key.code {
            for (shortcut_char, action) in shortcuts {
                if *shortcut_char == c || shortcut_char.to_ascii_uppercase() == c.to_ascii_uppercase() {
                    return Some(action.clone());
                }
            }
        }
        None
    }
}

/// Event handler composition utility
pub struct EventHandlerChain {
    handlers: Vec<Box<dyn Fn(KeyEvent) -> Option<ScreenAction>>>,
}

impl EventHandlerChain {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Add a handler to the chain
    pub fn add_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(KeyEvent) -> Option<ScreenAction> + 'static,
    {
        self.handlers.push(Box::new(handler));
        self
    }

    /// Process key event through all handlers
    pub fn handle(&self, key: KeyEvent) -> Option<ScreenAction> {
        for handler in &self.handlers {
            if let Some(action) = handler(key) {
                return Some(action);
            }
        }
        None
    }
}

impl Default for EventHandlerChain {
    fn default() -> Self {
        Self::new()
    }
}

// Note: These helper functions are commented out due to lifetime complexity
// In practice, screens should call the handlers directly:
//
// if let Some(action) = CommonKeyHandler::handle_navigation_keys(&mut self.navigable, key) {
//     return Ok(action);
// }