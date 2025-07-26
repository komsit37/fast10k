//! Help screen for the EDINET TUI

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::edinet_tui::ui::Styles;

/// Help sections
#[derive(Debug, Clone, PartialEq)]
pub enum HelpSection {
    Overview,
    Navigation,
    Database,
    Search,
    Results,
    Viewer,
    Shortcuts,
}

impl HelpSection {
    pub fn as_str(&self) -> &str {
        match self {
            HelpSection::Overview => "Overview",
            HelpSection::Navigation => "Navigation",
            HelpSection::Database => "Database Management",
            HelpSection::Search => "Document Search",
            HelpSection::Results => "Search Results",
            HelpSection::Viewer => "Document Viewer",
            HelpSection::Shortcuts => "Keyboard Shortcuts",
        }
    }
}

/// Help screen state
pub struct HelpScreen {
    pub current_section: usize,
    pub sections: Vec<HelpSection>,
    pub section_state: ListState,
    pub scroll_offset: usize,
}

impl HelpScreen {
    pub fn new() -> Self {
        let sections = vec![
            HelpSection::Overview,
            HelpSection::Navigation,
            HelpSection::Database,
            HelpSection::Search,
            HelpSection::Results,
            HelpSection::Viewer,
            HelpSection::Shortcuts,
        ];

        let mut section_state = ListState::default();
        section_state.select(Some(0));

        Self {
            current_section: 0,
            sections,
            section_state,
            scroll_offset: 0,
        }
    }

    /// Handle key events for the help screen
    pub async fn handle_event(
        &mut self,
        key: KeyEvent,
        _app: &mut super::super::app::App,
    ) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                if self.current_section > 0 {
                    self.current_section -= 1;
                    self.section_state.select(Some(self.current_section));
                    self.scroll_offset = 0;
                }
            }
            KeyCode::Down => {
                if self.current_section < self.sections.len() - 1 {
                    self.current_section += 1;
                    self.section_state.select(Some(self.current_section));
                    self.scroll_offset = 0;
                }
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.scroll_offset += 10;
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
            }
            _ => {}
        }
        Ok(())
    }

    /// Get content for current section
    fn get_section_content(&self) -> Vec<Line> {
        match self.sections[self.current_section] {
            HelpSection::Overview => self.get_overview_content(),
            HelpSection::Navigation => self.get_navigation_content(),
            HelpSection::Database => self.get_database_content(),
            HelpSection::Search => self.get_search_content(),
            HelpSection::Results => self.get_results_content(),
            HelpSection::Viewer => self.get_viewer_content(),
            HelpSection::Shortcuts => self.get_shortcuts_content(),
        }
    }

    fn get_overview_content(&self) -> Vec<Line> {
        vec![
            Line::from(Span::styled(
                "EDINET TUI - Document Manager",
                Styles::title(),
            )),
            Line::from(""),
            Line::from("This application provides a terminal-based interface for managing"),
            Line::from("EDINET (Japan Financial Services Agency) documents."),
            Line::from(""),
            Line::from(Span::styled("Features:", Styles::info())),
            Line::from("• Database management and indexing"),
            Line::from("• Document search by multiple criteria"),
            Line::from("• Document viewing and content preview"),
            Line::from("• Bulk document downloading"),
            Line::from("• Context-sensitive keyboard navigation"),
            Line::from(""),
            Line::from(Span::styled("Getting Started:", Styles::info())),
            Line::from("1. Use Database Management to build your document index"),
            Line::from("2. Search for documents by symbol, company, or date"),
            Line::from("3. View and download documents from search results"),
            Line::from(""),
            Line::from("Navigate using the Tab key or arrow keys between sections."),
        ]
    }

    fn get_navigation_content(&self) -> Vec<Line> {
        vec![
            Line::from(Span::styled("Navigation", Styles::title())),
            Line::from(""),
            Line::from(Span::styled("Global Navigation:", Styles::info())),
            Line::from("• ESC - Go back to previous screen or main menu"),
            Line::from("• q - Quit application from anywhere"),
            Line::from("• F1 or ? - Toggle help popup"),
            Line::from(""),
            Line::from(Span::styled("Screen Navigation:", Styles::info())),
            Line::from("• Arrow keys (↑/↓) - Navigate lists and menus"),
            Line::from("• Tab/Shift+Tab - Switch between input fields"),
            Line::from("• Enter - Select/execute current option"),
            Line::from("• Page Up/Page Down - Scroll content or navigate pages"),
            Line::from("• Home/End - Go to beginning/end of content"),
            Line::from(""),
            Line::from(Span::styled("Text Input:", Styles::info())),
            Line::from("• Type directly in focused input fields"),
            Line::from("• Backspace/Delete - Remove characters"),
            Line::from("• Left/Right arrows - Move cursor"),
            Line::from("• Ctrl+A/Home - Move to beginning of field"),
            Line::from("• Ctrl+E/End - Move to end of field"),
        ]
    }

    fn get_database_content(&self) -> Vec<Line> {
        vec![
            Line::from(Span::styled("Database Management", Styles::title())),
            Line::from(""),
            Line::from("Manage your EDINET document index and statistics."),
            Line::from(""),
            Line::from(Span::styled("Operations:", Styles::info())),
            Line::from("• Show Statistics (s) - Display current index status"),
            Line::from("• Update Index (u) - Add recent documents (last 7 days)"),
            Line::from("• Build Index (b) - Index documents for date range"),
            Line::from("• Clear Index (c) - Remove all data and rebuild"),
            Line::from(""),
            Line::from(Span::styled("Shortcuts:", Styles::info())),
            Line::from("• s - Show statistics"),
            Line::from("• u - Update index"),
            Line::from("• b - Build index (will prompt for date range)"),
            Line::from("• c - Clear index"),
            Line::from(""),
            Line::from(Span::styled("Build Index:", Styles::info())),
            Line::from("When building an index, you'll be prompted for:"),
            Line::from("• From Date - Start date (YYYY-MM-DD format)"),
            Line::from("• To Date - End date (YYYY-MM-DD format)"),
            Line::from(""),
            Line::from("Note: Index operations require an EDINET API key."),
        ]
    }

    fn get_search_content(&self) -> Vec<Line> {
        vec![
            Line::from(Span::styled("Document Search", Styles::title())),
            Line::from(""),
            Line::from("Search for EDINET documents using multiple criteria."),
            Line::from(""),
            Line::from(Span::styled("Search Fields:", Styles::info())),
            Line::from("• Ticker Symbol - Company stock symbol (e.g., 7203, 6758)"),
            Line::from("• Company Name - Full or partial company name"),
            Line::from("• Filing Type - Document type (annual, quarterly, etc.)"),
            Line::from("• Date From/To - Date range (YYYY-MM-DD format)"),
            Line::from("• Text Search - Search within document content"),
            Line::from(""),
            Line::from(Span::styled("Navigation:", Styles::info())),
            Line::from("• Tab/Shift+Tab - Move between fields"),
            Line::from("• ↑/↓ - Navigate between fields"),
            Line::from("• Enter - Execute search or open dropdown"),
            Line::from(""),
            Line::from(Span::styled("Filing Types:", Styles::info())),
            Line::from("Press Enter on Filing Type field to see available options:"),
            Line::from("• Annual Securities Report"),
            Line::from("• Quarterly Securities Report"),
            Line::from("• Semi-Annual Securities Report"),
            Line::from("• Extraordinary Report"),
            Line::from("• Internal Control Report"),
        ]
    }

    fn get_results_content(&self) -> Vec<Line> {
        vec![
            Line::from(Span::styled("Search Results", Styles::title())),
            Line::from(""),
            Line::from("Browse and interact with search results."),
            Line::from(""),
            Line::from(Span::styled("Navigation:", Styles::info())),
            Line::from("• ↑/↓ - Navigate through documents"),
            Line::from("• Page Up/Down - Navigate pages"),
            Line::from("• Home/End - Go to first/last page"),
            Line::from(""),
            Line::from(Span::styled("Actions:", Styles::info())),
            Line::from("• Enter or v - View selected document"),
            Line::from("• d - Download selected document"),
            Line::from("• / - Start new search"),
            Line::from("• r - Refresh current search"),
            Line::from(""),
            Line::from(Span::styled("Display Format:", Styles::info())),
            Line::from("Results are displayed in a table format showing:"),
            Line::from("• Date - Document filing date"),
            Line::from("• Symbol - Company ticker symbol"),
            Line::from("• Company - Company name (truncated)"),
            Line::from("• Type - Filing type"),
            Line::from("• Format - Document format"),
            Line::from(""),
            Line::from("Use the pagination info at the bottom right to track"),
            Line::from("your position in the results."),
        ]
    }

    fn get_viewer_content(&self) -> Vec<Line> {
        vec![
            Line::from(Span::styled("Document Viewer", Styles::title())),
            Line::from(""),
            Line::from("View document information and content."),
            Line::from(""),
            Line::from(Span::styled("View Modes:", Styles::info())),
            Line::from("• Info - Document metadata and information"),
            Line::from("• Content - Document sections and text content"),
            Line::from("• Download - Download options and status"),
            Line::from(""),
            Line::from(Span::styled("Navigation:", Styles::info())),
            Line::from("• Tab - Switch between view modes"),
            Line::from("• ↑/↓ - Scroll content or navigate sections"),
            Line::from("• Page Up/Down - Scroll content quickly"),
            Line::from("• Home/End - Go to beginning/end"),
            Line::from(""),
            Line::from(Span::styled("Actions:", Styles::info())),
            Line::from("• Enter - Load content (Content mode) or download"),
            Line::from("• d - Download document"),
            Line::from("• r - Reload content (Content mode)"),
            Line::from("• s - Save content to file (planned)"),
            Line::from(""),
            Line::from(Span::styled("Content Viewing:", Styles::info())),
            Line::from("• Documents must be downloaded before content can be viewed"),
            Line::from("• Content is parsed from EDINET ZIP files"),
            Line::from("• Navigate between sections using ↑/↓ in Content mode"),
            Line::from("• Sections include headers, business info, financials, etc."),
        ]
    }

    fn get_shortcuts_content(&self) -> Vec<Line> {
        vec![
            Line::from(Span::styled(
                "Keyboard Shortcuts Reference",
                Styles::title(),
            )),
            Line::from(""),
            Line::from(Span::styled("Global Shortcuts:", Styles::info())),
            Line::from("┌─────────────┬─────────────────────────────────┐"),
            Line::from("│ ESC         │ Go back / Main menu             │"),
            Line::from("│ q           │ Quit application                │"),
            Line::from("│ F1 or ?     │ Toggle help popup               │"),
            Line::from("└─────────────┴─────────────────────────────────┘"),
            Line::from(""),
            Line::from(Span::styled("Main Menu:", Styles::info())),
            Line::from("┌─────────────┬─────────────────────────────────┐"),
            Line::from("│ ↑/↓         │ Navigate menu items             │"),
            Line::from("│ Enter       │ Select menu item                │"),
            Line::from("│ 1-3         │ Direct selection                │"),
            Line::from("│ q           │ Quit                            │"),
            Line::from("└─────────────┴─────────────────────────────────┘"),
            Line::from(""),
            Line::from(Span::styled("Database Management:", Styles::info())),
            Line::from("┌─────────────┬─────────────────────────────────┐"),
            Line::from("│ s           │ Show statistics                 │"),
            Line::from("│ u           │ Update index                    │"),
            Line::from("│ b           │ Build index (date range)        │"),
            Line::from("│ c           │ Clear index                     │"),
            Line::from("└─────────────┴─────────────────────────────────┘"),
            Line::from(""),
            Line::from(Span::styled("Search & Results:", Styles::info())),
            Line::from("┌─────────────┬─────────────────────────────────┐"),
            Line::from("│ Tab         │ Next field / Switch modes       │"),
            Line::from("│ Enter       │ Search / Select / View          │"),
            Line::from("│ d           │ Download document               │"),
            Line::from("│ v           │ View document                   │"),
            Line::from("│ /           │ New search                      │"),
            Line::from("│ r           │ Refresh/reload                  │"),
            Line::from("└─────────────┴─────────────────────────────────┘"),
        ]
    }

    /// Draw the help screen
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(area);

        // Draw section list
        self.draw_section_list(f, chunks[0]);

        // Draw content
        self.draw_content(f, chunks[1]);
    }

    fn draw_section_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .sections
            .iter()
            .enumerate()
            .map(|(i, section)| {
                let style = if i == self.current_section {
                    Styles::selected()
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(section.as_str(), style)))
            })
            .collect();

        let section_list = List::new(items)
            .block(
                Block::default()
                    .title("Help Sections")
                    .borders(Borders::ALL)
                    .border_style(Styles::active_border()),
            )
            .highlight_style(Styles::selected());

        f.render_stateful_widget(section_list, area, &mut self.section_state);
    }

    fn draw_content(&self, f: &mut Frame, area: Rect) {
        let content_lines = self.get_section_content();

        // Apply scrolling
        let visible_lines: Vec<Line> = content_lines.into_iter().skip(self.scroll_offset).collect();

        let content_widget = Paragraph::new(visible_lines)
            .block(
                Block::default()
                    .title(format!(
                        "Help - {}",
                        self.sections[self.current_section].as_str()
                    ))
                    .borders(Borders::ALL)
                    .border_style(Styles::active_border()),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(content_widget, area);
    }
}

