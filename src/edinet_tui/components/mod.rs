//! Reusable UI components for the EDINET TUI
//!
//! This module provides composable UI components that implement common patterns
//! and can be reused across different screens.

pub mod list_view;
pub mod document_table;
pub mod status_display;
pub mod form_field;
pub mod base_screen;

pub use list_view::ListView;
pub use document_table::DocumentTable;
pub use status_display::StatusDisplay;
pub use form_field::{FormField, FormFieldType};
pub use base_screen::BaseScreen;