//! Main TUI application state and logic

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};

use crate::config::Config;
use super::screens::*;

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
                // ESC goes back to previous screen or main menu
                if let Some(prev) = self.previous_screen.clone() {
                    self.navigate_to_screen(prev);
                } else if self.current_screen != Screen::MainMenu {
                    self.navigate_to_screen(Screen::MainMenu);
                } else {
                    self.should_quit = true;
                }
                return Ok(());
            }
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
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
            format!("EDINET TUI - {} | ESC: Back | Ctrl+Q: Quit | F1/?:Help", 
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
            .block(Block::default()
                .title("Help - Context Shortcuts")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Yellow)))
            .style(Style::default().fg(Color::White));

        f.render_widget(help_popup, popup_area);
    }

    /// Get context-sensitive help content
    fn get_context_help(&self) -> String {
        let global_help = "Global Shortcuts:\n\
            ESC - Go back / Main menu\n\
            Ctrl+Q - Quit application\n\
            F1 / ? - Toggle this help\n\n";

        let screen_help = match self.current_screen {
            Screen::MainMenu => "Main Menu:\n\
                ↑/↓ - Navigate menu\n\
                Enter - Select option\n\
                1 - Database Management\n\
                2 - Search Documents\n\
                3 - Help\n\
                q - Quit",
            Screen::Database => "Database Management:\n\
                ↑/↓ - Navigate options\n\
                Enter - Execute action\n\
                s - Show statistics\n\
                u - Update index\n\
                b - Build index (date range)\n\
                c - Clear/rebuild index",
            Screen::Search => "Search Documents:\n\
                Tab - Next field\n\
                Shift+Tab - Previous field\n\
                Enter - Execute search\n\
                Type in text fields\n\
                ↑/↓ - Navigate dropdowns\n\
                Space - Toggle selections",
            Screen::Results => "Search Results:\n\
                ↑/↓ - Navigate documents\n\
                Enter - View document\n\
                d - Download document\n\
                r - Refresh search\n\
                / - New search\n\
                Page Up/Down - Navigate pages",
            Screen::Viewer => "Document Viewer:\n\
                ↑/↓ - Scroll content\n\
                Page Up/Down - Page scroll\n\
                Home/End - Top/Bottom\n\
                d - Download document\n\
                s - Save content to file\n\
                Enter - Open in external viewer",
            Screen::Help => "Help Screen:\n\
                ↑/↓ - Scroll help content\n\
                Tab - Switch help sections",
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
                    if selected == 0 { // Show Stats
                        self.set_status("Database statistics - feature coming soon".to_string());
                    } else if selected == 1 { // Update Index
                        self.set_status("Index update - feature coming soon".to_string());
                    } else if selected == 2 { // Build Index
                        self.set_status("Index build - feature coming soon".to_string());
                    } else if selected == 3 { // Clear Index
                        self.set_status("Index clear - feature coming soon".to_string());
                    }
                }
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
                self.search.current_field = (self.search.current_field + 1) % self.search.fields.len();
                self.search.update_field_focus();
                self.set_status(format!("Focus: {}", self.search.fields[self.search.current_field].as_str()));
            }
            KeyCode::BackTab => {
                self.search.current_field = if self.search.current_field == 0 {
                    self.search.fields.len() - 1
                } else {
                    self.search.current_field - 1
                };
                self.search.update_field_focus();
                self.set_status(format!("Focus: {}", self.search.fields[self.search.current_field].as_str()));
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
                self.set_status("Search functionality - coming soon".to_string());
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
        match key.code {
            KeyCode::Up => {
                self.results.navigate_up();
                self.set_status("Navigate results with ↑/↓, Enter to view, d to download".to_string());
            }
            KeyCode::Down => {
                self.results.navigate_down();
                self.set_status("Navigate results with ↑/↓, Enter to view, d to download".to_string());
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
                self.set_status("Document viewer - coming soon".to_string());
            }
            KeyCode::Char('d') => {
                self.set_status("Document download - coming soon".to_string());
            }
            KeyCode::Char('/') => {
                self.navigate_to_screen(Screen::Search);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_viewer_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                self.viewer.mode = match self.viewer.mode {
                    super::screens::viewer::ViewerMode::Info => super::screens::viewer::ViewerMode::Content,
                    super::screens::viewer::ViewerMode::Content => super::screens::viewer::ViewerMode::Download,
                    super::screens::viewer::ViewerMode::Download => super::screens::viewer::ViewerMode::Info,
                };
                self.set_status(format!("Switched to {:?} mode", self.viewer.mode));
            }
            KeyCode::Up => {
                if self.viewer.scroll_offset > 0 {
                    self.viewer.scroll_offset -= 1;
                }
            }
            KeyCode::Down => {
                self.viewer.scroll_offset += 1;
            }
            KeyCode::PageUp => {
                self.viewer.scroll_offset = self.viewer.scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.viewer.scroll_offset += 10;
            }
            KeyCode::Home => {
                self.viewer.scroll_offset = 0;
            }
            KeyCode::Char('d') => {
                self.set_status("Document download - coming soon".to_string());
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_help_event(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                if self.help.current_section > 0 {
                    self.help.current_section -= 1;
                    self.help.section_state.select(Some(self.help.current_section));
                    self.help.scroll_offset = 0;
                }
            }
            KeyCode::Down => {
                if self.help.current_section < self.help.sections.len() - 1 {
                    self.help.current_section += 1;
                    self.help.section_state.select(Some(self.help.current_section));
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
            _ => {}
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