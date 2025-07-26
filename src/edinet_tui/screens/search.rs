//! Search screen for the EDINET TUI

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use chrono::{NaiveDate, Local};

use crate::{
    models::{SearchQuery, Source, FilingType, DocumentFormat},
    storage,
    edinet_tui::ui::{Styles, InputField, SelectableList}, edinet_tui::app::Screen,
};

/// Search form fields
#[derive(Debug, Clone, PartialEq)]
pub enum SearchField {
    Ticker,
    CompanyName,
    FilingType,
    DateFrom,
    DateTo,
    TextQuery,
}

impl SearchField {
    pub fn as_str(&self) -> &str {
        match self {
            SearchField::Ticker => "Ticker Symbol",
            SearchField::CompanyName => "Company Name",
            SearchField::FilingType => "Filing Type",
            SearchField::DateFrom => "Date From",
            SearchField::DateTo => "Date To",
            SearchField::TextQuery => "Text Search",
        }
    }
}

/// Search screen state
pub struct SearchScreen {
    pub current_field: usize,
    pub fields: Vec<SearchField>,
    
    // Input fields
    pub ticker_input: InputField,
    pub company_input: InputField,
    pub date_from_input: InputField,
    pub date_to_input: InputField,
    pub text_query_input: InputField,
    
    // Dropdown selections
    pub filing_type_list: SelectableList<FilingType>,
    pub show_filing_dropdown: bool,
    
    // Search state
    pub is_searching: bool,
    pub last_query: Option<SearchQuery>,
}

impl SearchScreen {
    pub fn new() -> Self {
        let fields = vec![
            SearchField::Ticker,
            SearchField::CompanyName,
            SearchField::FilingType,
            SearchField::DateFrom,
            SearchField::DateTo,
            SearchField::TextQuery,
        ];

        // Available filing types for EDINET
        let filing_types = vec![
            FilingType::AnnualSecuritiesReport,     // 有価証券報告書
            FilingType::QuarterlySecuritiesReport,  // 四半期報告書
            FilingType::SemiAnnualSecuritiesReport, // 半期報告書
            FilingType::ExtraordinaryReport,        // 臨時報告書
            FilingType::Other("Internal Control Report".to_string()), // 内部統制報告書
        ];

        let mut search_screen = Self {
            current_field: 0,
            fields,
            
            ticker_input: InputField::new("Ticker Symbol")
                .with_placeholder("e.g., 7203, 6758"),
            company_input: InputField::new("Company Name")
                .with_placeholder("e.g., Toyota, Sony"),
            date_from_input: InputField::new("Date From (YYYY-MM-DD)")
                .with_placeholder("2024-01-01"),
            date_to_input: InputField::new("Date To (YYYY-MM-DD)")
                .with_placeholder(&Local::now().format("%Y-%m-%d").to_string()),
            text_query_input: InputField::new("Text Search")
                .with_placeholder("Search in document content"),
            
            filing_type_list: {
                let mut list = SelectableList::new(filing_types);
                list.select(None); // No filing type selected by default
                list
            },
            show_filing_dropdown: false,
            
            is_searching: false,
            last_query: None,
        };

        search_screen.update_field_focus();
        search_screen
    }

    /// Handle key events for the search screen
    pub async fn handle_event(&mut self, key: KeyEvent, app: &mut super::super::app::App) -> Result<()> {
        if self.show_filing_dropdown {
            return self.handle_filing_dropdown_event(key, app).await;
        }

        match key.code {
            KeyCode::Tab => {
                self.current_field = (self.current_field + 1) % self.fields.len();
                self.update_field_focus();
            }
            KeyCode::BackTab => {
                self.current_field = if self.current_field == 0 {
                    self.fields.len() - 1
                } else {
                    self.current_field - 1
                };
                self.update_field_focus();
            }
            KeyCode::Up => {
                if self.current_field > 0 {
                    self.current_field -= 1;
                    self.update_field_focus();
                }
            }
            KeyCode::Down => {
                if self.current_field < self.fields.len() - 1 {
                    self.current_field += 1;
                    self.update_field_focus();
                }
            }
            KeyCode::Enter => {
                if self.fields[self.current_field] == SearchField::FilingType {
                    self.show_filing_dropdown = true;
                } else {
                    self.execute_search(app).await?;
                }
            }
            KeyCode::Char(c) => {
                self.handle_char_input(c);
            }
            KeyCode::Backspace => {
                self.handle_backspace();
            }
            KeyCode::Delete => {
                self.handle_delete();
            }
            KeyCode::Left => {
                self.handle_cursor_left();
            }
            KeyCode::Right => {
                self.handle_cursor_right();
            }
            KeyCode::Home => {
                self.handle_cursor_home();
            }
            KeyCode::End => {
                self.handle_cursor_end();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle filing type dropdown events
    async fn handle_filing_dropdown_event(&mut self, key: KeyEvent, app: &mut super::super::app::App) -> Result<()> {
        match key.code {
            KeyCode::Up => {
                self.filing_type_list.previous();
            }
            KeyCode::Down => {
                self.filing_type_list.next();
            }
            KeyCode::Enter => {
                self.show_filing_dropdown = false;
            }
            KeyCode::Esc => {
                self.show_filing_dropdown = false;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn update_field_focus(&mut self) {
        // Clear all focus
        self.ticker_input.set_focus(false);
        self.company_input.set_focus(false);
        self.date_from_input.set_focus(false);
        self.date_to_input.set_focus(false);
        self.text_query_input.set_focus(false);

        // Set focus on current field
        match self.fields[self.current_field] {
            SearchField::Ticker => self.ticker_input.set_focus(true),
            SearchField::CompanyName => self.company_input.set_focus(true),
            SearchField::DateFrom => self.date_from_input.set_focus(true),
            SearchField::DateTo => self.date_to_input.set_focus(true),
            SearchField::TextQuery => self.text_query_input.set_focus(true),
            SearchField::FilingType => {} // Handled separately
        }
    }

    pub fn handle_char_input(&mut self, c: char) {
        eprintln!("Handling char input '{}' for field {:?}", c, self.fields[self.current_field]);
        match self.fields[self.current_field] {
            SearchField::Ticker => {
                self.ticker_input.insert_char(c);
                eprintln!("Ticker input now: '{}'", self.ticker_input.value);
            },
            SearchField::CompanyName => self.company_input.insert_char(c),
            SearchField::DateFrom => self.date_from_input.insert_char(c),
            SearchField::DateTo => self.date_to_input.insert_char(c),
            SearchField::TextQuery => self.text_query_input.insert_char(c),
            SearchField::FilingType => {} // Handled by dropdown
        }
    }

    pub fn handle_backspace(&mut self) {
        match self.fields[self.current_field] {
            SearchField::Ticker => self.ticker_input.delete_char(),
            SearchField::CompanyName => self.company_input.delete_char(),
            SearchField::DateFrom => self.date_from_input.delete_char(),
            SearchField::DateTo => self.date_to_input.delete_char(),
            SearchField::TextQuery => self.text_query_input.delete_char(),
            SearchField::FilingType => {}
        }
    }

    fn handle_delete(&mut self) {
        match self.fields[self.current_field] {
            SearchField::Ticker => self.ticker_input.delete_char_forward(),
            SearchField::CompanyName => self.company_input.delete_char_forward(),
            SearchField::DateFrom => self.date_from_input.delete_char_forward(),
            SearchField::DateTo => self.date_to_input.delete_char_forward(),
            SearchField::TextQuery => self.text_query_input.delete_char_forward(),
            SearchField::FilingType => {}
        }
    }

    fn handle_cursor_left(&mut self) {
        match self.fields[self.current_field] {
            SearchField::Ticker => self.ticker_input.move_cursor_left(),
            SearchField::CompanyName => self.company_input.move_cursor_left(),
            SearchField::DateFrom => self.date_from_input.move_cursor_left(),
            SearchField::DateTo => self.date_to_input.move_cursor_left(),
            SearchField::TextQuery => self.text_query_input.move_cursor_left(),
            SearchField::FilingType => {}
        }
    }

    fn handle_cursor_right(&mut self) {
        match self.fields[self.current_field] {
            SearchField::Ticker => self.ticker_input.move_cursor_right(),
            SearchField::CompanyName => self.company_input.move_cursor_right(),
            SearchField::DateFrom => self.date_from_input.move_cursor_right(),
            SearchField::DateTo => self.date_to_input.move_cursor_right(),
            SearchField::TextQuery => self.text_query_input.move_cursor_right(),
            SearchField::FilingType => {}
        }
    }

    fn handle_cursor_home(&mut self) {
        match self.fields[self.current_field] {
            SearchField::Ticker => self.ticker_input.move_cursor_to_start(),
            SearchField::CompanyName => self.company_input.move_cursor_to_start(),
            SearchField::DateFrom => self.date_from_input.move_cursor_to_start(),
            SearchField::DateTo => self.date_to_input.move_cursor_to_start(),
            SearchField::TextQuery => self.text_query_input.move_cursor_to_start(),
            SearchField::FilingType => {}
        }
    }

    fn handle_cursor_end(&mut self) {
        match self.fields[self.current_field] {
            SearchField::Ticker => self.ticker_input.move_cursor_to_end(),
            SearchField::CompanyName => self.company_input.move_cursor_to_end(),
            SearchField::DateFrom => self.date_from_input.move_cursor_to_end(),
            SearchField::DateTo => self.date_to_input.move_cursor_to_end(),
            SearchField::TextQuery => self.text_query_input.move_cursor_to_end(),
            SearchField::FilingType => {}
        }
    }

    /// Execute search with current form values
    async fn execute_search(&mut self, app: &mut super::super::app::App) -> Result<()> {
        // Validate date inputs
        if !self.date_from_input.is_empty() {
            if NaiveDate::parse_from_str(&self.date_from_input.value, "%Y-%m-%d").is_err() {
                app.set_error("Invalid 'Date From' format. Please use YYYY-MM-DD".to_string());
                return Ok(());
            }
        }
        
        if !self.date_to_input.is_empty() {
            if NaiveDate::parse_from_str(&self.date_to_input.value, "%Y-%m-%d").is_err() {
                app.set_error("Invalid 'Date To' format. Please use YYYY-MM-DD".to_string());
                return Ok(());
            }
        }

        // Build search query
        let search_query = SearchQuery {
            ticker: if self.ticker_input.is_empty() { None } else { Some(self.ticker_input.value.clone()) },
            company_name: if self.company_input.is_empty() { None } else { Some(self.company_input.value.clone()) },
            filing_type: self.filing_type_list.selected().cloned(),
            source: Some(Source::Edinet),
            date_from: if self.date_from_input.is_empty() { 
                None 
            } else { 
                NaiveDate::parse_from_str(&self.date_from_input.value, "%Y-%m-%d").ok() 
            },
            date_to: if self.date_to_input.is_empty() { 
                None 
            } else { 
                NaiveDate::parse_from_str(&self.date_to_input.value, "%Y-%m-%d").ok() 
            },
            text_query: if self.text_query_input.is_empty() { None } else { Some(self.text_query_input.value.clone()) },
        };

        // Debug: Log the search query
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("tui_debug.log") {
            writeln!(file, "TUI Search Query: ticker={:?}, company={:?}, filing_type={:?}, source={:?}", 
                search_query.ticker, search_query.company_name, search_query.filing_type, search_query.source).ok();
        }
        eprintln!("TUI Search Query: ticker={:?}, company={:?}, filing_type={:?}, source={:?}", 
            search_query.ticker, search_query.company_name, search_query.filing_type, search_query.source);

        // Check if search has any criteria
        if search_query.ticker.is_none() 
            && search_query.company_name.is_none()
            && search_query.filing_type.is_none()
            && search_query.date_from.is_none() 
            && search_query.date_to.is_none()
            && search_query.text_query.is_none() {
            app.set_error("Please enter at least one search criteria".to_string());
            return Ok(());
        }

        self.is_searching = true;
        app.set_status("Searching documents...".to_string());

        match storage::search_documents(&search_query, app.config.database_path_str(), 100).await {
            Ok(documents) => {
                app.set_status(format!("Found {} documents", documents.len()));
                
                // Store results in the results screen
                app.results.set_documents(documents);
                self.last_query = Some(search_query);
                
                // Navigate to results screen
                app.navigate_to_screen(Screen::Results);
            }
            Err(e) => {
                app.set_error(format!("Search failed: {}", e));
            }
        }

        self.is_searching = false;
        Ok(())
    }

    /// Clear all search fields
    pub fn clear_search(&mut self) {
        self.ticker_input.clear();
        self.company_input.clear();
        self.date_from_input.clear();
        self.date_to_input.clear();
        self.text_query_input.clear();
        self.filing_type_list.select(None);
        self.current_field = 0;
        self.update_field_focus();
    }

    /// Draw the search screen
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Form
                Constraint::Length(4),  // Instructions
            ])
            .split(area);

        // Draw title
        self.draw_title(f, chunks[0]);
        
        // Draw search form
        self.draw_form(f, chunks[1]);
        
        // Draw instructions
        self.draw_instructions(f, chunks[2]);

        // Draw filing type dropdown if active
        if self.show_filing_dropdown {
            self.draw_filing_dropdown(f, area);
        }
    }

    fn draw_title(&self, f: &mut Frame, area: Rect) {
        let title = if self.is_searching {
            "Document Search - Searching..."
        } else {
            "Document Search"
        };
        
        let title_widget = Paragraph::new(title)
            .style(if self.is_searching { Styles::warning() } else { Styles::title() })
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title_widget, area);
    }

    fn draw_form(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Ticker
                Constraint::Length(3), // Company
                Constraint::Length(3), // Filing Type
                Constraint::Length(3), // Date From
                Constraint::Length(3), // Date To
                Constraint::Length(3), // Text Query
            ])
            .split(area);

        // Render input fields
        self.ticker_input.render(f, chunks[0]);
        self.company_input.render(f, chunks[1]);
        
        // Filing type field (special handling)
        self.draw_filing_type_field(f, chunks[2]);
        
        self.date_from_input.render(f, chunks[3]);
        self.date_to_input.render(f, chunks[4]);
        self.text_query_input.render(f, chunks[5]);
    }

    fn draw_filing_type_field(&self, f: &mut Frame, area: Rect) {
        let selected_type = self.filing_type_list.selected()
            .map(|ft| ft.as_str())
            .unwrap_or("Any");

        let style = if self.fields[self.current_field] == SearchField::FilingType {
            Styles::active_border()
        } else {
            Styles::inactive_border()
        };

        let field = Paragraph::new(selected_type)
            .block(Block::default()
                .title("Filing Type (Enter to select)")
                .borders(Borders::ALL)
                .border_style(style));

        f.render_widget(field, area);
    }

    fn draw_instructions(&self, f: &mut Frame, area: Rect) {
        let instructions = vec![
            Line::from("Tab/Shift+Tab: Navigate fields | ↑/↓: Navigate | Enter: Search/Select"),
            Line::from("Enter on Filing Type: Show dropdown | Clear fields: Ctrl+L"),
        ];

        let instructions_widget = Paragraph::new(instructions)
            .style(Styles::info())
            .block(Block::default()
                .title("Instructions")
                .borders(Borders::ALL)
                .border_style(Styles::inactive_border()));

        f.render_widget(instructions_widget, area);
    }

    fn draw_filing_dropdown(&mut self, f: &mut Frame, area: Rect) {
        use crate::edinet_tui::ui::centered_rect;
        
        let popup_area = centered_rect(50, 50, area);
        
        let items: Vec<ListItem> = self.filing_type_list.items
            .iter()
            .enumerate()
            .map(|(i, filing_type)| {
                let style = if Some(i) == self.filing_type_list.selected_index() {
                    Styles::selected()
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(filing_type.as_str(), style)))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title("Select Filing Type")
                .borders(Borders::ALL)
                .border_style(Styles::active_border()))
            .highlight_style(Styles::selected());

        f.render_widget(ratatui::widgets::Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut self.filing_type_list.state);
    }
}