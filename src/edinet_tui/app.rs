//! Main TUI application state and logic

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};

use super::screens::*;
use crate::config::Config;
use crate::models::{FilingType, SearchQuery, Source};
use crate::storage;

/// Application screens
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    Database,
    Search,
    Results,
    Viewer,
    Help,
}

/// Main TUI application state
pub struct App {
    /// Current active screen
    pub current_screen: Screen,
    /// Previous screen for navigation
    pub previous_screen: Option<Screen>,
    /// Application configuration
    pub config: Config,

    // Screen states
    pub main_menu: MainMenuScreen,
    pub database: DatabaseScreen,
    pub search: SearchScreen,
    pub results: ResultsScreen,
    pub viewer: ViewerScreen,
    pub help: HelpScreen,

    // Global application state
    pub should_quit: bool,
    pub show_help_popup: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
}

impl App {
    /// Create a new TUI application
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            current_screen: Screen::MainMenu,
            previous_screen: None,
            config: config.clone(),

            main_menu: MainMenuScreen::new(),
            database: DatabaseScreen::new(config.clone()),
            search: SearchScreen::new(),
            results: ResultsScreen::new(),
            viewer: ViewerScreen::new(),
            help: HelpScreen::new(),

            should_quit: false,
            show_help_popup: false,
            status_message: None,
            error_message: None,
        })
    }

    /// Run the main application loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Initial database check
        self.check_database_status().await;

        loop {
            // Draw the UI
            terminal.draw(|f| self.draw(f))?;

            // Handle events
            if let Ok(event) = crossterm::event::read() {
                if let crossterm::event::Event::Key(key) = event {
                    self.handle_key_event(key).await?;
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle keyboard input events
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Global shortcuts
        match key.code {
            KeyCode::F(1) | KeyCode::Char('?') => {
                self.show_help_popup = !self.show_help_popup;
                return Ok(());
            }
            KeyCode::Esc => {
                if self.show_help_popup {
                    self.show_help_popup = false;
                    return Ok(());
                }
                // ESC handling is now delegated to individual screen handlers
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
                return Ok(());
            }
            _ => {}
        }

        // Screen-specific event handling
        if !self.show_help_popup {
            match self.current_screen {
                Screen::MainMenu => self.handle_main_menu_event(key).await?,
                Screen::Database => self.handle_database_event(key).await?,
                Screen::Search => self.handle_search_event(key).await?,
                Screen::Results => self.handle_results_event(key).await?,
                Screen::Viewer => self.handle_viewer_event(key).await?,
                Screen::Help => self.handle_help_event(key).await?,
            }
        }

        Ok(())
    }

    /// Draw the UI
    pub fn draw(&mut self, f: &mut Frame) {
        let size = f.size();

        // Main layout: status bar at bottom, content area above
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(size);

        // Draw current screen content
        match self.current_screen {
            Screen::MainMenu => self.main_menu.draw(f, chunks[0]),
            Screen::Database => self.database.draw(f, chunks[0]),
            Screen::Search => self.search.draw(f, chunks[0]),
            Screen::Results => self.results.draw(f, chunks[0]),
            Screen::Viewer => self.viewer.draw(f, chunks[0]),
            Screen::Help => self.help.draw(f, chunks[0]),
        }

        // Draw status bar
        self.draw_status_bar(f, chunks[1]);

        // Draw help popup if active
        if self.show_help_popup {
            self.draw_help_popup(f, size);
        }
    }

    /// Draw status bar with current screen info and shortcuts
    fn draw_status_bar(&self, f: &mut Frame, area: Rect) {
        let status_text = if let Some(ref msg) = self.status_message {
            format!("Status: {}", msg)
        } else if let Some(ref err) = self.error_message {
            format!("Error: {}", err)
        } else {
            format!(
                "EDINET TUI - {} | ESC: Back | Q: Quit | F1/?:Help",
                match self.current_screen {
                    Screen::MainMenu => "Main Menu",
                    Screen::Database => "Database Management",
                    Screen::Search => "Search Documents",
                    Screen::Results => "Search Results",
                    Screen::Viewer => "Document Viewer",
                    Screen::Help => "Help",
                }
            )
        };

        let style = if self.error_message.is_some() {
            Style::default().fg(Color::Red)
        } else if self.status_message.is_some() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Gray)
        };

        let status_bar = Paragraph::new(status_text)
            .style(style)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(status_bar, area);
    }

    /// Draw help popup with context-sensitive shortcuts
    fn draw_help_popup(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(80, 70, area);

        f.render_widget(Clear, popup_area);

        let help_content = self.get_context_help();
        let help_popup = Paragraph::new(help_content)
            .block(
                Block::default()
                    .title("Help - Context Shortcuts")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(help_popup, popup_area);
    }

    /// Get context-sensitive help content
    fn get_context_help(&self) -> String {
        let global_help = "Global Shortcuts:\n\
            ESC - Go back\n\
            Q - Quit application\n\
            F1 / ? - Toggle this help\n\n";

        let screen_help = match self.current_screen {
            Screen::MainMenu => {
                "Main Menu:\n\
                ↑/↓ - Navigate menu\n\
                Enter - Select option\n\
                1 - Search Documents\n\
                2 - Database Management\n\
                3 - Help\n\
                q - Quit"
            }
            Screen::Database => {
                "Database Management:\n\
                ↑/↓ - Navigate options\n\
                Enter - Execute action\n\
                s - Show statistics\n\
                u - Update index\n\
                b - Build index (date range)\n\
                c - Clear/rebuild index"
            }
            Screen::Search => {
                "Search Documents:\n\
                Tab - Next field\n\
                Shift+Tab - Previous field\n\
                Enter - Execute search\n\
                Type in text fields\n\
                ↑/↓ - Navigate dropdowns\n\
                Space - Toggle selections"
            }
            Screen::Results => {
                "Search Results:\n\
                ↑/↓ - Navigate documents\n\
                Enter - View document\n\
                d - Download document\n\
                r - Refresh search\n\
                / - New search\n\
                Page Up/Down - Navigate pages"
            }
            Screen::Viewer => {
                "Document Viewer:\n\
                ↑/↓ - Scroll content\n\
                Page Up/Down - Page scroll\n\
                Home/End - Top/Bottom\n\
                d - Download document\n\
                s - Save content to file\n\
                Enter - Open in external viewer"
            }
            Screen::Help => {
                "Help Screen:\n\
                ↑/↓ - Scroll help content\n\
                Tab - Switch help sections"
            }
        };

        format!("{}{}", global_help, screen_help)
    }

    /// Navigate to a specific screen
    pub fn navigate_to_screen(&mut self, screen: Screen) {
        self.previous_screen = Some(self.current_screen.clone());
        self.current_screen = screen;
        self.clear_messages();
    }

    /// Set status message
    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
        self.error_message = None;
    }

    /// Set error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.status_message = None;
    }

    /// Clear status and error messages
    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }

    /// Check database status on startup
    async fn check_database_status(&mut self) {
        // This will be implemented to check if database exists and has data
        // For now, just set a status message
        self.set_status("Ready - Database connection established".to_string());
    }

    // Event handlers for each screen
    async fn handle_main_menu_event(&mut self, key: KeyEvent) -> Result<()> {
        // Extract the required data before borrowing self
        match key.code {
            KeyCode::Up => {
                let selected = self.main_menu.menu_state.selected().unwrap_or(0);
                let new_selected = if selected == 0 {
                    self.main_menu.menu_options.len() - 1
                } else {
                    selected - 1
                };
                self.main_menu.menu_state.select(Some(new_selected));
            }
            KeyCode::Down => {
                let selected = self.main_menu.menu_state.selected().unwrap_or(0);
                let new_selected = (selected + 1) % self.main_menu.menu_options.len();
                self.main_menu.menu_state.select(Some(new_selected));
            }
            KeyCode::Enter => {
                if let Some(selected) = self.main_menu.menu_state.selected() {
                    if let Some(option) = self.main_menu.menu_options.get(selected) {
                        self.navigate_to_screen(option.screen.clone());
                    }
                }
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char(c) => {
                // Handle shortcut keys
                for option in &self.main_menu.menu_options {
                    if option.shortcut == c {
                        self.navigate_to_screen(option.screen.clone());
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_database_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                let selected = self.database.operation_state.selected().unwrap_or(0);
                let new_selected = if selected == 0 {
                    self.database.operations.len() - 1
                } else {
                    selected - 1
                };
                self.database.operation_state.select(Some(new_selected));
            }
            KeyCode::Down => {
                let selected = self.database.operation_state.selected().unwrap_or(0);
                let new_selected = (selected + 1) % self.database.operations.len();
                self.database.operation_state.select(Some(new_selected));
            }
            KeyCode::Enter => {
                if let Some(selected) = self.database.operation_state.selected() {
                    if selected == 0 {
                        // Show Stats
                        self.set_status("Database statistics - feature coming soon".to_string());
                    } else if selected == 1 {
                        // Update Index
                        self.set_status("Index update - feature coming soon".to_string());
                    } else if selected == 2 {
                        // Build Index
                        self.set_status("Index build - feature coming soon".to_string());
                    } else if selected == 3 {
                        // Clear Index
                        self.set_status("Index clear - feature coming soon".to_string());
                    }
                }
            }
            KeyCode::Esc => {
                // Database screen: ESC goes back to Main Menu
                self.navigate_to_screen(Screen::MainMenu);
            }
            KeyCode::Char('s') => {
                self.set_status("Database statistics - feature coming soon".to_string());
            }
            KeyCode::Char('u') => {
                self.set_status("Index update - feature coming soon".to_string());
            }
            KeyCode::Char('b') => {
                self.set_status("Index build - feature coming soon".to_string());
            }
            KeyCode::Char('c') => {
                self.set_status("Index clear - feature coming soon".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_search_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                self.search.current_field =
                    (self.search.current_field + 1) % self.search.fields.len();
                self.search.update_field_focus();
                self.set_status(format!(
                    "Focus: {}",
                    self.search.fields[self.search.current_field].as_str()
                ));
            }
            KeyCode::BackTab => {
                self.search.current_field = if self.search.current_field == 0 {
                    self.search.fields.len() - 1
                } else {
                    self.search.current_field - 1
                };
                self.search.update_field_focus();
                self.set_status(format!(
                    "Focus: {}",
                    self.search.fields[self.search.current_field].as_str()
                ));
            }
            KeyCode::Up => {
                if self.search.current_field > 0 {
                    self.search.current_field -= 1;
                    self.search.update_field_focus();
                }
            }
            KeyCode::Down => {
                if self.search.current_field < self.search.fields.len() - 1 {
                    self.search.current_field += 1;
                    self.search.update_field_focus();
                }
            }
            KeyCode::Enter => {
                // Execute search
                self.execute_search().await?;
            }
            KeyCode::Esc => {
                // Search screen: ESC goes back to Main Menu
                self.navigate_to_screen(Screen::MainMenu);
            }
            KeyCode::Char(c) => {
                self.search.handle_char_input(c);
            }
            KeyCode::Backspace => {
                self.search.handle_backspace();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_results_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle download cancellation
        if self.results.is_downloading {
            if let KeyCode::Esc = key.code {
                self.results.is_downloading = false;
                self.results.download_status = None;
                self.set_status("Download cancelled".to_string());
                return Ok(());
            }
            // Ignore all other keys during download
            return Ok(());
        }
        
        match key.code {
            KeyCode::Up => {
                self.results.navigate_up();
                self.set_status(
                    "Navigate results with ↑/↓, Enter to view, d to download".to_string(),
                );
            }
            KeyCode::Down => {
                self.results.navigate_down();
                self.set_status(
                    "Navigate results with ↑/↓, Enter to view, d to download".to_string(),
                );
            }
            KeyCode::PageUp => {
                self.results.previous_page();
                self.set_status("Previous page".to_string());
            }
            KeyCode::PageDown => {
                self.results.next_page();
                self.set_status("Next page".to_string());
            }
            KeyCode::Enter | KeyCode::Char('v') => {
                if let Some(document) = self.results.get_selected_document() {
                    self.viewer.set_document(document.clone());
                    // Check download status after setting document
                    self.viewer.is_downloaded = self.viewer.is_document_downloaded(self);
                    self.navigate_to_screen(Screen::Viewer);
                } else {
                    self.set_error("No document selected".to_string());
                }
            }
            KeyCode::Esc => {
                // Results screen: ESC goes back to Search
                self.navigate_to_screen(Screen::Search);
            }
            KeyCode::Char('d') => {
                // Download selected document
                if let Some(document) = self.results.get_selected_document().cloned() {
                    self.results.is_downloading = true;
                    self.results.download_status = Some(format!("Downloading {}...", document.ticker));
                    self.set_status(format!("Starting download for {}", document.ticker));
                    
                    let download_request = crate::models::DownloadRequest {
                        source: crate::models::Source::Edinet,
                        ticker: document.ticker.clone(),
                        filing_type: Some(document.filing_type.clone()),
                        date_from: Some(document.date),
                        date_to: Some(document.date),
                        limit: 1,
                        format: crate::models::DocumentFormat::Complete,
                    };
                    
                    match crate::downloader::download_documents(&download_request, self.config.download_dir_str()).await {
                        Ok(count) => {
                            self.set_status(format!(
                                "Successfully downloaded {} document(s) to {}",
                                count,
                                self.config.download_dir_str()
                            ));
                        }
                        Err(e) => {
                            self.set_error(format!("Download failed: {}", e));
                        }
                    }
                    
                    self.results.is_downloading = false;
                    self.results.download_status = None;
                } else {
                    self.set_error("No document selected".to_string());
                }
            }
            KeyCode::Char('/') => {
                self.navigate_to_screen(Screen::Search);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_viewer_event(&mut self, key: KeyEvent) -> Result<()> {
        // Handle download cancellation
        if self.viewer.is_downloading {
            if let KeyCode::Esc = key.code {
                self.viewer.is_downloading = false;
                self.viewer.download_status = None;
                self.set_status("Download cancelled".to_string());
                return Ok(());
            }
            // Ignore all other keys during download
            return Ok(());
        }

        match key.code {
            KeyCode::Tab => {
                // Switch between modes
                self.viewer.mode = match self.viewer.mode {
                    super::screens::viewer::ViewerMode::Info => super::screens::viewer::ViewerMode::Content,
                    super::screens::viewer::ViewerMode::Content => super::screens::viewer::ViewerMode::Download,
                    super::screens::viewer::ViewerMode::Download => super::screens::viewer::ViewerMode::Info,
                };
                self.viewer.scroll_offset = 0;
            }
            KeyCode::Up => {
                match self.viewer.mode {
                    super::screens::viewer::ViewerMode::Info | super::screens::viewer::ViewerMode::Download => {
                        if self.viewer.scroll_offset > 0 {
                            self.viewer.scroll_offset -= 1;
                        }
                    }
                    super::screens::viewer::ViewerMode::Content => {
                        if self.viewer.content_sections.is_some() && self.viewer.current_section > 0 {
                            self.viewer.current_section -= 1;
                            self.viewer.scroll_offset = 0;
                        }
                    }
                }
            }
            KeyCode::Down => {
                match self.viewer.mode {
                    super::screens::viewer::ViewerMode::Info | super::screens::viewer::ViewerMode::Download => {
                        self.viewer.scroll_offset += 1;
                    }
                    super::screens::viewer::ViewerMode::Content => {
                        if let Some(ref sections) = self.viewer.content_sections {
                            if self.viewer.current_section < sections.len() - 1 {
                                self.viewer.current_section += 1;
                                self.viewer.scroll_offset = 0;
                            }
                        }
                    }
                }
            }
            KeyCode::PageUp => {
                match self.viewer.mode {
                    super::screens::viewer::ViewerMode::Info | super::screens::viewer::ViewerMode::Download => {
                        self.viewer.scroll_offset = self.viewer.scroll_offset.saturating_sub(10);
                    }
                    super::screens::viewer::ViewerMode::Content => {
                        self.viewer.scroll_offset = self.viewer.scroll_offset.saturating_sub(10);
                    }
                }
            }
            KeyCode::PageDown => {
                match self.viewer.mode {
                    super::screens::viewer::ViewerMode::Info | super::screens::viewer::ViewerMode::Download => {
                        self.viewer.scroll_offset += 10;
                    }
                    super::screens::viewer::ViewerMode::Content => {
                        self.viewer.scroll_offset += 10;
                    }
                }
            }
            KeyCode::Home => {
                self.viewer.scroll_offset = 0;
                if self.viewer.mode == super::screens::viewer::ViewerMode::Content {
                    self.viewer.current_section = 0;
                }
            }
            KeyCode::End => {
                if self.viewer.mode == super::screens::viewer::ViewerMode::Content {
                    if let Some(ref sections) = self.viewer.content_sections {
                        self.viewer.current_section = sections.len().saturating_sub(1);
                    }
                }
                self.viewer.scroll_offset = 0;
            }
            KeyCode::Enter => {
                match self.viewer.mode {
                    super::screens::viewer::ViewerMode::Content => {
                        // Load content if not already loaded
                        self.load_viewer_content().await?;
                    }
                    super::screens::viewer::ViewerMode::Download => {
                        // Download document
                        self.download_viewer_document().await?;
                    }
                    super::screens::viewer::ViewerMode::Info => {
                        // Switch to content view
                        self.viewer.mode = super::screens::viewer::ViewerMode::Content;
                        self.load_viewer_content().await?;
                    }
                }
            }
            KeyCode::Char('d') => {
                // Download document
                self.download_viewer_document().await?;
            }
            KeyCode::Char('r') => {
                // Reload/refresh content
                if self.viewer.mode == super::screens::viewer::ViewerMode::Content {
                    self.viewer.content_sections = None;
                    self.load_viewer_content().await?;
                }
            }
            KeyCode::Char('s') => {
                // Save content to file (placeholder)
                self.set_status("Save functionality not implemented yet".to_string());
            }
            KeyCode::Esc => {
                // Viewer screen: ESC goes back to Results
                self.navigate_to_screen(Screen::Results);
            }
            _ => {}
        }
        Ok(())
    }

    /// Load document content for viewer
    async fn load_viewer_content(&mut self) -> Result<()> {
        if self.viewer.content_sections.is_some() {
            return Ok(()); // Already loaded
        }

        let document = match &self.viewer.current_document {
            Some(doc) => doc.clone(),
            None => return Ok(()),
        };

        self.viewer.is_loading = true;
        self.set_status("Loading document content...".to_string());

        // Construct expected download path
        let download_dir = std::path::PathBuf::from(self.config.download_dir_str());
        let edinet_dir = download_dir.join("edinet").join(&document.ticker);

        // Look for ZIP files in the directory
        if let Ok(entries) = std::fs::read_dir(&edinet_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("zip") {
                    match crate::edinet::reader::read_edinet_zip(path.to_str().unwrap(), 20, 1000) {
                        Ok(sections) => {
                            self.viewer.content_sections = Some(sections);
                            self.viewer.current_section = 0;
                            self.viewer.is_loading = false;
                            self.set_status("Document content loaded".to_string());
                            return Ok(());
                        }
                        Err(e) => {
                            self.set_error(format!("Failed to read document: {}", e));
                            self.viewer.is_loading = false;
                            return Ok(());
                        }
                    }
                }
            }
        }

        // If no downloaded file found, suggest downloading
        self.set_error("Document not found locally. Use 'd' to download first.".to_string());
        self.viewer.is_loading = false;
        Ok(())
    }

    /// Download document from viewer
    async fn download_viewer_document(&mut self) -> Result<()> {
        let document = match &self.viewer.current_document {
            Some(doc) => doc.clone(),
            None => return Ok(()),
        };

        self.viewer.is_downloading = true;
        self.viewer.download_status = Some(format!("Downloading {}...", document.ticker));
        
        self.set_status(format!("Starting download for {}", document.ticker));

        let download_request = crate::models::DownloadRequest {
            source: crate::models::Source::Edinet,
            ticker: document.ticker.clone(),
            filing_type: Some(document.filing_type.clone()),
            date_from: Some(document.date),
            date_to: Some(document.date),
            limit: 1,
            format: crate::models::DocumentFormat::Complete,
        };

        match crate::downloader::download_documents(&download_request, self.config.download_dir_str()).await {
            Ok(count) => {
                self.set_status(format!("Successfully downloaded {} document(s)", count));
                // Clear content sections to force reload
                self.viewer.content_sections = None;
                // Update download status
                self.viewer.is_downloaded = self.viewer.is_document_downloaded(self);
            }
            Err(e) => {
                self.set_error(format!("Download failed: {}", e));
            }
        }

        self.viewer.is_downloading = false;
        self.viewer.download_status = None;
        Ok(())
    }

    async fn handle_help_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                if self.help.current_section > 0 {
                    self.help.current_section -= 1;
                    self.help
                        .section_state
                        .select(Some(self.help.current_section));
                    self.help.scroll_offset = 0;
                }
            }
            KeyCode::Down => {
                if self.help.current_section < self.help.sections.len() - 1 {
                    self.help.current_section += 1;
                    self.help
                        .section_state
                        .select(Some(self.help.current_section));
                    self.help.scroll_offset = 0;
                }
            }
            KeyCode::PageUp => {
                self.help.scroll_offset = self.help.scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.help.scroll_offset += 10;
            }
            KeyCode::Home => {
                self.help.scroll_offset = 0;
            }
            KeyCode::Esc => {
                // Help screen: ESC goes back to Main Menu
                self.navigate_to_screen(Screen::MainMenu);
            }
            _ => {}
        }
        Ok(())
    }

    /// Execute search with current form values
    async fn execute_search(&mut self) -> Result<()> {
        use chrono::NaiveDate;

        // Validate date inputs
        if !self.search.date_from_input.is_empty() {
            if NaiveDate::parse_from_str(&self.search.date_from_input.value, "%Y-%m-%d").is_err() {
                self.set_error("Invalid 'Date From' format. Please use YYYY-MM-DD".to_string());
                return Ok(());
            }
        }

        if !self.search.date_to_input.is_empty() {
            if NaiveDate::parse_from_str(&self.search.date_to_input.value, "%Y-%m-%d").is_err() {
                self.set_error("Invalid 'Date To' format. Please use YYYY-MM-DD".to_string());
                return Ok(());
            }
        }

        // Build search query
        let search_query = SearchQuery {
            ticker: if self.search.ticker_input.is_empty() {
                None
            } else {
                Some(self.search.ticker_input.value.clone())
            },
            company_name: if self.search.company_input.is_empty() {
                None
            } else {
                Some(self.search.company_input.value.clone())
            },
            filing_type: self.search.filing_type_list.selected().cloned(),
            source: Some(Source::Edinet),
            date_from: if self.search.date_from_input.is_empty() {
                None
            } else {
                NaiveDate::parse_from_str(&self.search.date_from_input.value, "%Y-%m-%d").ok()
            },
            date_to: if self.search.date_to_input.is_empty() {
                None
            } else {
                NaiveDate::parse_from_str(&self.search.date_to_input.value, "%Y-%m-%d").ok()
            },
            text_query: if self.search.text_query_input.is_empty() {
                None
            } else {
                Some(self.search.text_query_input.value.clone())
            },
        };

        // Check if search has any criteria
        if search_query.ticker.is_none()
            && search_query.company_name.is_none()
            && search_query.filing_type.is_none()
            && search_query.date_from.is_none()
            && search_query.date_to.is_none()
            && search_query.text_query.is_none()
        {
            self.set_error("Please enter at least one search criteria".to_string());
            return Ok(());
        }

        self.set_status("Searching documents...".to_string());


        match storage::search_documents(&search_query, self.config.database_path_str(), 100).await {
            Ok(documents) => {
                self.set_status(format!("Found {} documents", documents.len()));

                // Store results in the results screen
                self.results.set_documents(documents);
                self.search.last_query = Some(search_query);

                // Navigate to results screen
                self.navigate_to_screen(Screen::Results);
            }
            Err(e) => {
                self.set_error(format!("Search failed: {}", e));
            }
        }

        Ok(())
    }
}

/// Helper function to center a rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

