//! EDINET Terminal User Interface (TUI)
//! 
//! This module provides a comprehensive TUI for managing EDINET documents,
//! including database management, searching, and viewing capabilities.

pub mod app;
pub mod ui;
pub mod events;
pub mod screens;

pub use app::App;
pub use events::AppEvent;

// Re-export screen modules for easy access
pub use screens::{
    main_menu::MainMenuScreen,
    database::DatabaseScreen,
    search::SearchScreen,
    results::ResultsScreen,
    viewer::ViewerScreen,
    help::HelpScreen,
};