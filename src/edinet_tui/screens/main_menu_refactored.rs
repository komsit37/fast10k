//! Refactored main menu screen using the new TUI architecture
//!
//! This demonstrates the new pattern for implementing screens with composable
//! components and standardized event handling.

use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::edinet_tui::{
    app::Screen as ScreenType,
    components::{
        list_view::{MenuListView, MenuItem},
        status_display::StatusDisplay,
    },
    handlers::{CommonKeyHandler, MenuHandler},
    traits::{Navigable, Screen, ScreenAction},
    ui::Styles,
};

/// Main menu screen data
#[derive(Debug, Clone)]
pub struct MainMenuData {
    pub title: String,
    pub subtitle: String,
}

impl Default for MainMenuData {
    fn default() -> Self {
        Self {
            title: "EDINET Document Manager".to_string(),
            subtitle: "Japanese Financial Document Search & Analysis".to_string(),
        }
    }
}

/// Refactored main menu screen using new architecture
pub struct MainMenuScreenRefactored {
    data: MainMenuData,
    menu: MenuListView,
    status: StatusDisplay,
    screen_type: ScreenType,
}

impl MainMenuScreenRefactored {
    pub fn new() -> Self {
        let menu_items = vec![
            MenuItem::new("Search Documents")
                .with_shortcut('S')
                .with_description("Search for EDINET documents by symbol, company, date, or type"),
            MenuItem::new("Database Management")
                .with_shortcut('D')
                .with_description("Manage EDINET document index, update, and statistics"),
            MenuItem::new("Help")
                .with_shortcut('H')
                .with_description("View help and keyboard shortcuts"),
        ];

        let menu = MenuListView::new(menu_items, "Main Menu");
        let status = StatusDisplay::new();

        Self {
            data: MainMenuData::default(),
            menu,
            status,
            screen_type: ScreenType::MainMenu,
        }
    }

    /// Get screen actions that correspond to menu items
    fn get_menu_actions(&self) -> Vec<ScreenAction> {
        vec![
            ScreenAction::NavigateTo(ScreenType::Search),
            ScreenAction::NavigateTo(ScreenType::Database),
            ScreenAction::NavigateTo(ScreenType::Help),
        ]
    }

    /// Get shortcut mappings
    fn get_shortcuts(&self) -> Vec<(char, ScreenAction)> {
        vec![
            ('S', ScreenAction::NavigateTo(ScreenType::Search)),
            ('s', ScreenAction::NavigateTo(ScreenType::Search)),
            ('D', ScreenAction::NavigateTo(ScreenType::Database)),
            ('d', ScreenAction::NavigateTo(ScreenType::Database)),
            ('H', ScreenAction::NavigateTo(ScreenType::Help)),
            ('h', ScreenAction::NavigateTo(ScreenType::Help)),
        ]
    }

    fn draw_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new(vec![
            Line::from(Span::styled(
                &self.data.title,
                Styles::title().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(&self.data.subtitle, Styles::info())),
        ])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn draw_instructions(&self, f: &mut Frame, area: Rect) {
        let instructions = vec![
            Line::from(vec![
                Span::styled("Navigation: ", Styles::info()),
                Span::raw("↑/↓ to move, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to select"),
            ]),
            Line::from(vec![
                Span::styled("Shortcuts: ", Styles::info()),
                Span::styled("S/D/H", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" for direct access, "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to quit"),
            ]),
            Line::from(vec![
                Span::styled("Global: ", Styles::info()),
                Span::styled("F1/?", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" for help, "),
                Span::styled("ESC", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to go back"),
            ]),
        ];

        let instructions_paragraph = Paragraph::new(instructions).block(
            Block::default()
                .title("Instructions")
                .borders(Borders::ALL)
                .border_style(Styles::inactive_border()),
        );

        f.render_widget(instructions_paragraph, area);
    }
}

impl Screen for MainMenuScreenRefactored {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        // Create layout: title at top, menu in center, instructions at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Title (2 lines + borders)
                Constraint::Min(0),    // Menu
                Constraint::Length(6), // Instructions
            ])
            .split(area);

        // Draw components
        self.draw_title(f, chunks[0]);
        self.menu.render(f, chunks[1]);
        self.draw_instructions(f, chunks[2]);
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<ScreenAction> {
        // Try common navigation first
        if let Some(action) = CommonKeyHandler::handle_navigation_keys(&mut self.menu, key) {
            if action != ScreenAction::None {
                return Ok(action);
            }
        }

        // Try menu selection
        let menu_actions = self.get_menu_actions();
        if let Some(action) = MenuHandler::handle_menu_selection(&self.menu, key, &menu_actions) {
            return Ok(action);
        }

        // Try shortcuts
        let shortcuts = self.get_shortcuts();
        if let Some(action) = MenuHandler::handle_menu_shortcuts(key, &shortcuts) {
            return Ok(action);
        }

        // Try global keys
        if let Some(action) = CommonKeyHandler::handle_global_keys(key) {
            return Ok(action);
        }

        // No action taken
        Ok(ScreenAction::None)
    }

    fn screen_type(&self) -> ScreenType {
        self.screen_type.clone()
    }

    fn can_navigate_back(&self) -> bool {
        false // Main menu is the root screen
    }

    fn on_enter(&mut self) {
        self.status.set_info("Welcome to EDINET Document Manager".to_string());
    }

    async fn refresh(&mut self) -> Result<()> {
        // Could refresh any dynamic data here
        Ok(())
    }
}

impl Navigable for MainMenuScreenRefactored {
    fn navigate_up(&mut self) {
        self.menu.previous();
    }

    fn navigate_down(&mut self) {
        self.menu.next();
    }

    fn get_selected_index(&self) -> Option<usize> {
        self.menu.selected().map(|_| self.menu.list_view.selected_index().unwrap_or(0))
    }

    fn set_selected_index(&mut self, index: Option<usize>) {
        self.menu.list_view.select(index);
    }

    fn get_item_count(&self) -> usize {
        self.menu.items.len()
    }
}

// Demonstrate how simple it is to extend functionality
impl MainMenuScreenRefactored {
    /// Add a new menu item dynamically
    pub fn add_menu_item(&mut self, item: MenuItem) {
        self.menu.items.push(item);
        self.menu.list_view.set_items(self.menu.items.clone());
    }

    /// Remove a menu item by index
    pub fn remove_menu_item(&mut self, index: usize) {
        if index < self.menu.items.len() {
            self.menu.items.remove(index);
            self.menu.list_view.set_items(self.menu.items.clone());
        }
    }

    /// Update screen title and subtitle
    pub fn set_title(&mut self, title: String, subtitle: String) {
        self.data.title = title;
        self.data.subtitle = subtitle;
    }

    /// Get current status display for external updates
    pub fn status_mut(&mut self) -> &mut StatusDisplay {
        &mut self.status
    }
}