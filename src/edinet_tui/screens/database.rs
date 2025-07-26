//! Database management screen for the EDINET TUI

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Gauge},
    Frame,
};
use chrono::{NaiveDate, Local};

use crate::{
    config::Config,
    edinet_indexer,
    storage,
    models::Source,
    edinet_tui::ui::{Styles, InputField},
};

/// Database management operations
#[derive(Debug, Clone)]
pub enum DatabaseOperation {
    ShowStats,
    UpdateIndex,
    BuildIndex,
    ClearIndex,
}

impl DatabaseOperation {
    pub fn as_str(&self) -> &str {
        match self {
            DatabaseOperation::ShowStats => "Show Statistics",
            DatabaseOperation::UpdateIndex => "Update Index (last 7 days)",
            DatabaseOperation::BuildIndex => "Build Index (date range)",
            DatabaseOperation::ClearIndex => "Clear/Rebuild Index",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            DatabaseOperation::ShowStats => "Display current index statistics and status",
            DatabaseOperation::UpdateIndex => "Update index with recent documents",
            DatabaseOperation::BuildIndex => "Build index for a specific date range",
            DatabaseOperation::ClearIndex => "Clear all data and rebuild from scratch",
        }
    }

    pub fn shortcut(&self) -> char {
        match self {
            DatabaseOperation::ShowStats => 's',
            DatabaseOperation::UpdateIndex => 'u',
            DatabaseOperation::BuildIndex => 'b',
            DatabaseOperation::ClearIndex => 'c',
        }
    }
}

/// Current database statistics
#[derive(Debug, Clone, Default)]
pub struct DatabaseStats {
    pub total_documents: i64,
    pub edinet_documents: i64,
    pub date_range: Option<(String, String)>,
    pub last_updated: Option<String>,
    pub database_size: Option<String>,
}

/// Database management screen state
pub struct DatabaseScreen {
    pub config: Config,
    pub operation_state: ListState,
    pub operations: Vec<DatabaseOperation>,
    pub stats: DatabaseStats,
    pub is_loading: bool,
    pub current_operation: Option<String>,
    pub progress: Option<f64>,
    
    // For build index date range input
    pub input_mode: bool,
    pub from_date_input: InputField,
    pub to_date_input: InputField,
    pub current_input_field: usize,
}

impl DatabaseScreen {
    pub fn new(config: Config) -> Self {
        let operations = vec![
            DatabaseOperation::ShowStats,
            DatabaseOperation::UpdateIndex,
            DatabaseOperation::BuildIndex,
            DatabaseOperation::ClearIndex,
        ];

        let mut operation_state = ListState::default();
        operation_state.select(Some(0));

        Self {
            config,
            operation_state,
            operations,
            stats: DatabaseStats::default(),
            is_loading: false,
            current_operation: None,
            progress: None,
            input_mode: false,
            from_date_input: InputField::new("From Date (YYYY-MM-DD)")
                .with_placeholder("2024-01-01"),
            to_date_input: InputField::new("To Date (YYYY-MM-DD)")
                .with_placeholder(&Local::now().format("%Y-%m-%d").to_string()),
            current_input_field: 0,
        }
    }

    /// Handle key events for the database screen
    pub async fn handle_event(&mut self, key: KeyEvent, app: &mut super::super::app::App) -> Result<()> {
        if self.input_mode {
            return self.handle_input_mode_event(key, app).await;
        }

        match key.code {
            KeyCode::Up => {
                let selected = self.operation_state.selected().unwrap_or(0);
                let new_selected = if selected == 0 {
                    self.operations.len() - 1
                } else {
                    selected - 1
                };
                self.operation_state.select(Some(new_selected));
            }
            KeyCode::Down => {
                let selected = self.operation_state.selected().unwrap_or(0);
                let new_selected = (selected + 1) % self.operations.len();
                self.operation_state.select(Some(new_selected));
            }
            KeyCode::Enter => {
                if let Some(selected) = self.operation_state.selected() {
                    if let Some(operation) = self.operations.get(selected) {
                        self.execute_operation(operation.clone(), app).await?;
                    }
                }
            }
            KeyCode::Char(c) => {
                // Handle shortcut keys
                for operation in &self.operations {
                    if operation.shortcut() == c {
                        self.execute_operation(operation.clone(), app).await?;
                        break;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle input mode events for date range input
    async fn handle_input_mode_event(&mut self, key: KeyEvent, app: &mut super::super::app::App) -> Result<()> {
        match key.code {
            KeyCode::Tab => {
                self.current_input_field = (self.current_input_field + 1) % 2;
                self.update_input_focus();
            }
            KeyCode::BackTab => {
                self.current_input_field = if self.current_input_field == 0 { 1 } else { 0 };
                self.update_input_focus();
            }
            KeyCode::Enter => {
                // Validate and execute build index
                if let (Ok(from_date), Ok(to_date)) = (
                    NaiveDate::parse_from_str(&self.from_date_input.value, "%Y-%m-%d"),
                    NaiveDate::parse_from_str(&self.to_date_input.value, "%Y-%m-%d"),
                ) {
                    self.input_mode = false;
                    self.execute_build_index(from_date, to_date, app).await?;
                } else {
                    app.set_error("Invalid date format. Please use YYYY-MM-DD".to_string());
                }
            }
            KeyCode::Esc => {
                self.input_mode = false;
                self.update_input_focus();
            }
            KeyCode::Char(c) => {
                self.get_current_input_field().insert_char(c);
            }
            KeyCode::Backspace => {
                self.get_current_input_field().delete_char();
            }
            KeyCode::Delete => {
                self.get_current_input_field().delete_char_forward();
            }
            KeyCode::Left => {
                self.get_current_input_field().move_cursor_left();
            }
            KeyCode::Right => {
                self.get_current_input_field().move_cursor_right();
            }
            KeyCode::Home => {
                self.get_current_input_field().move_cursor_to_start();
            }
            KeyCode::End => {
                self.get_current_input_field().move_cursor_to_end();
            }
            _ => {}
        }
        Ok(())
    }

    fn update_input_focus(&mut self) {
        self.from_date_input.set_focus(self.current_input_field == 0 && self.input_mode);
        self.to_date_input.set_focus(self.current_input_field == 1 && self.input_mode);
    }

    fn get_current_input_field(&mut self) -> &mut InputField {
        match self.current_input_field {
            0 => &mut self.from_date_input,
            1 => &mut self.to_date_input,
            _ => &mut self.from_date_input,
        }
    }

    /// Execute a database operation
    async fn execute_operation(&mut self, operation: DatabaseOperation, app: &mut super::super::app::App) -> Result<()> {
        match operation {
            DatabaseOperation::ShowStats => {
                self.refresh_stats(app).await?;
            }
            DatabaseOperation::UpdateIndex => {
                self.execute_update_index(app).await?;
            }
            DatabaseOperation::BuildIndex => {
                self.input_mode = true;
                self.current_input_field = 0;
                self.update_input_focus();
                app.set_status("Enter date range for index build".to_string());
            }
            DatabaseOperation::ClearIndex => {
                self.execute_clear_index(app).await?;
            }
        }
        Ok(())
    }

    /// Refresh database statistics
    async fn refresh_stats(&mut self, app: &mut super::super::app::App) -> Result<()> {
        app.set_status("Loading database statistics...".to_string());
        
        // Get document counts
        match storage::count_documents_by_source(&Source::Edinet, self.config.database_path_str()).await {
            Ok(count) => {
                self.stats.edinet_documents = count;
                self.stats.total_documents = count; // For now, only EDINET
            }
            Err(e) => {
                app.set_error(format!("Failed to get document count: {}", e));
                return Ok(());
            }
        }

        // Get date range
        match storage::get_date_range_for_source(&Source::Edinet, self.config.database_path_str()).await {
            Ok((start, end)) => {
                self.stats.date_range = Some((start, end));
            }
            Err(_) => {
                self.stats.date_range = None;
            }
        }

        app.set_status("Database statistics updated".to_string());
        Ok(())
    }

    /// Execute index update
    async fn execute_update_index(&mut self, app: &mut super::super::app::App) -> Result<()> {
        self.is_loading = true;
        self.current_operation = Some("Updating index...".to_string());
        
        app.set_status("Updating EDINET index...".to_string());
        
        match edinet_indexer::update_edinet_index(self.config.database_path_str(), 7).await {
            Ok(count) => {
                app.set_status(format!("Successfully updated index with {} documents", count));
                self.refresh_stats(app).await?;
            }
            Err(e) => {
                app.set_error(format!("Index update failed: {}", e));
            }
        }
        
        self.is_loading = false;
        self.current_operation = None;
        Ok(())
    }

    /// Execute build index for date range
    async fn execute_build_index(&mut self, from_date: NaiveDate, to_date: NaiveDate, app: &mut super::super::app::App) -> Result<()> {
        self.is_loading = true;
        self.current_operation = Some(format!("Building index from {} to {}...", from_date, to_date));
        
        app.set_status("Building EDINET index...".to_string());
        
        match edinet_indexer::build_edinet_index_by_date(self.config.database_path_str(), from_date, to_date).await {
            Ok(count) => {
                app.set_status(format!("Successfully indexed {} documents", count));
                self.refresh_stats(app).await?;
            }
            Err(e) => {
                app.set_error(format!("Index build failed: {}", e));
            }
        }
        
        self.is_loading = false;
        self.current_operation = None;
        Ok(())
    }

    /// Execute clear index
    async fn execute_clear_index(&mut self, app: &mut super::super::app::App) -> Result<()> {
        self.is_loading = true;
        self.current_operation = Some("Clearing index...".to_string());
        
        app.set_status("Clearing EDINET index...".to_string());
        
        // For now, we'll just show a message. In a real implementation,
        // you'd want to add a confirmation dialog and actual clear functionality
        app.set_status("Clear index functionality not implemented yet".to_string());
        
        self.is_loading = false;
        self.current_operation = None;
        Ok(())
    }

    /// Draw the database management screen
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        if self.input_mode {
            self.draw_input_mode(f, area);
        } else {
            self.draw_normal_mode(f, area);
        }
    }

    fn draw_normal_mode(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left side: Operations
        self.draw_operations(f, chunks[0]);
        
        // Right side: Statistics and status
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[1]);
        
        self.draw_statistics(f, right_chunks[0]);
        self.draw_status(f, right_chunks[1]);
    }

    fn draw_input_mode(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(3),  // From date
                Constraint::Length(3),  // To date
                Constraint::Length(3),  // Instructions
                Constraint::Min(0),     // Statistics (smaller)
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Build Index - Date Range")
            .style(Styles::title())
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Input fields
        self.from_date_input.render(f, chunks[1]);
        self.to_date_input.render(f, chunks[2]);

        // Instructions
        let instructions = Paragraph::new("Tab: Next field | Enter: Build | Esc: Cancel")
            .style(Styles::info())
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(instructions, chunks[3]);

        // Statistics (smaller)
        self.draw_statistics(f, chunks[4]);
    }

    fn draw_operations(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .operations
            .iter()
            .enumerate()
            .map(|(i, operation)| {
                let style = if Some(i) == self.operation_state.selected() {
                    Styles::selected()
                } else {
                    Style::default()
                };

                let content = vec![
                    Line::from(vec![
                        Span::styled(format!("[{}] ", operation.shortcut()), Styles::info()),
                        Span::styled(operation.as_str(), style.add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(Span::styled(format!("     {}", operation.description()), 
                        if Some(i) == self.operation_state.selected() {
                            style
                        } else {
                            Styles::inactive()
                        }
                    )),
                ];

                ListItem::new(content)
            })
            .collect();

        let operations_list = List::new(items)
            .block(Block::default()
                .title("Database Operations")
                .borders(Borders::ALL)
                .border_style(Styles::active_border()))
            .highlight_style(Styles::selected());

        f.render_stateful_widget(operations_list, area, &mut self.operation_state);
    }

    fn draw_statistics(&self, f: &mut Frame, area: Rect) {
        let stats_text = vec![
            Line::from(vec![
                Span::styled("Total Documents: ", Styles::info()),
                Span::raw(self.stats.total_documents.to_string()),
            ]),
            Line::from(vec![
                Span::styled("EDINET Documents: ", Styles::info()),
                Span::raw(self.stats.edinet_documents.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Date Range: ", Styles::info()),
                Span::raw(
                    self.stats.date_range
                        .as_ref()
                        .map(|(start, end)| format!("{} to {}", start, end))
                        .unwrap_or_else(|| "No data".to_string())
                ),
            ]),
        ];

        let statistics = Paragraph::new(stats_text)
            .block(Block::default()
                .title("Statistics")
                .borders(Borders::ALL)
                .border_style(Styles::active_border()));

        f.render_widget(statistics, area);
    }

    fn draw_status(&self, f: &mut Frame, area: Rect) {
        if self.is_loading {
            let status_text = self.current_operation
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("Working...");
            
            let status = Paragraph::new(status_text)
                .style(Styles::info())
                .block(Block::default()
                    .title("Status")
                    .borders(Borders::ALL));
            
            f.render_widget(status, area);
            
            // Show progress bar if available
            if let Some(progress) = self.progress {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(1)])
                    .split(area);
                
                let gauge = Gauge::default()
                    .ratio(progress)
                    .style(Styles::info());
                f.render_widget(gauge, chunks[1]);
            }
        } else {
            let instructions = vec![
                Line::from("↑/↓: Navigate | Enter: Execute"),
                Line::from("s/u/b/c: Direct shortcuts"),
            ];

            let help = Paragraph::new(instructions)
                .style(Style::default())
                .block(Block::default()
                    .title("Instructions")
                    .borders(Borders::ALL)
                    .border_style(Styles::inactive_border()));

            f.render_widget(help, area);
        }
    }
}