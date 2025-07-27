//! Screen modules for the EDINET TUI

pub mod main_menu;
pub mod main_menu_refactored;
pub mod database;
pub mod search;
pub mod results;
pub mod viewer;
pub mod help;

// Re-export all screens
pub use main_menu::MainMenuScreen;
pub use main_menu_refactored::MainMenuScreenRefactored;
pub use database::DatabaseScreen;
pub use search::SearchScreen;
pub use results::ResultsScreen;
pub use viewer::ViewerScreen;
pub use help::HelpScreen;