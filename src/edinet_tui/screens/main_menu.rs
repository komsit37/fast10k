//! Main menu screen for the EDINET TUI

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::edinet_tui::{app::Screen, ui::Styles};

/// Main menu options
#[derive(Debug, Clone)]
pub struct MenuOption {
    pub title: String,
    pub description: String,
    pub shortcut: char,
    pub screen: Screen,
}

impl MenuOption {
    pub fn new(title: &str, description: &str, shortcut: char, screen: Screen) -> Self {
        Self {
            title: title.to_string(),
            description: description.to_string(),
            shortcut,
            screen,
        }
    }
}

/// Main menu screen state
pub struct MainMenuScreen {
    pub menu_state: ListState,
    pub menu_options: Vec<MenuOption>,
}

impl MainMenuScreen {
    pub fn new() -> Self {
        let menu_options = vec![
            MenuOption::new(
                "Search Documents",
                "Search for EDINET documents by symbol, company, date, or type",
                'S',
                Screen::Search,
            ),
            MenuOption::new(
                "Database Management",
                "Manage EDINET document index, update, and statistics",
                'D',
                Screen::Database,
            ),
            MenuOption::new(
                "Help",
                "View help and keyboard shortcuts",
                'H',
                Screen::Help,
            ),
        ];

        let mut menu_state = ListState::default();
        menu_state.select(Some(0));

        Self {
            menu_state,
            menu_options,
        }
    }

    /// Handle key events for the main menu
    pub async fn handle_event(
        &mut self,
        key: KeyEvent,
        app: &mut super::super::app::App,
    ) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                let selected = self.menu_state.selected().unwrap_or(0);
                let new_selected = if selected == 0 {
                    self.menu_options.len() - 1
                } else {
                    selected - 1
                };
                self.menu_state.select(Some(new_selected));
            }
            KeyCode::Down => {
                let selected = self.menu_state.selected().unwrap_or(0);
                let new_selected = (selected + 1) % self.menu_options.len();
                self.menu_state.select(Some(new_selected));
            }
            KeyCode::Enter => {
                if let Some(selected) = self.menu_state.selected() {
                    if let Some(option) = self.menu_options.get(selected) {
                        app.navigate_to_screen(option.screen.clone());
                    }
                }
            }
            KeyCode::Char('q') => {
                app.should_quit = true;
            }
            KeyCode::Char(c) => {
                // Handle shortcut keys (case insensitive)
                let upper_c = c.to_ascii_uppercase();
                for option in &self.menu_options {
                    if option.shortcut == upper_c || option.shortcut == c {
                        app.navigate_to_screen(option.screen.clone());
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Draw the main menu screen
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        // Create layout: title at top, menu in center, instructions at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Menu
                Constraint::Length(6), // Instructions
            ])
            .split(area);

        // Draw title
        self.draw_title(f, chunks[0]);

        // Draw menu
        self.draw_menu(f, chunks[1]);

        // Draw instructions
        self.draw_instructions(f, chunks[2]);
    }

    fn draw_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("EDINET Document Manager")
            .style(Styles::title().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn draw_menu(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .menu_options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                let style = if Some(i) == self.menu_state.selected() {
                    Styles::selected()
                } else {
                    Style::default()
                };

                let content = vec![
                    Line::from(vec![
                        Span::styled(format!("[{}] ", option.shortcut), Styles::info()),
                        Span::styled(&option.title, style.add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(Span::styled(
                        format!("     {}", option.description),
                        if Some(i) == self.menu_state.selected() {
                            style
                        } else {
                            Styles::inactive()
                        },
                    )),
                ];

                ListItem::new(content)
            })
            .collect();

        let menu = List::new(items)
            .block(
                Block::default()
                    .title("Main Menu")
                    .borders(Borders::ALL)
                    .border_style(Styles::active_border()),
            )
            .highlight_style(Styles::selected());

        f.render_stateful_widget(menu, area, &mut self.menu_state);
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
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to quit from anywhere"),
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

