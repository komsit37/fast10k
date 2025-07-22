use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;
use tracing::info;
use crate::models::{SearchQuery, Document};
use crate::storage;

#[derive(Debug)]
enum AppState {
    Search,
    Documents,
    Downloads,
}

#[derive(Debug)]
struct App {
    state: AppState,
    tab_index: usize,
    documents: Vec<Document>,
    list_state: ListState,
    search_query: String,
    database_path: String,
}

impl App {
    fn new(database_path: &str) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        App {
            state: AppState::Search,
            tab_index: 0,
            documents: vec![],
            list_state,
            search_query: String::new(),
            database_path: database_path.to_string(),
        }
    }
    
    fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % 3;
        self.state = match self.tab_index {
            0 => AppState::Search,
            1 => AppState::Documents,
            2 => AppState::Downloads,
            _ => AppState::Search,
        };
    }
    
    fn previous_tab(&mut self) {
        if self.tab_index > 0 {
            self.tab_index -= 1;
        } else {
            self.tab_index = 2;
        }
        self.state = match self.tab_index {
            0 => AppState::Search,
            1 => AppState::Documents,
            2 => AppState::Downloads,
            _ => AppState::Search,
        };
    }
    
    fn next_document(&mut self) {
        if !self.documents.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => (i + 1) % self.documents.len(),
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
    
    fn previous_document(&mut self) {
        if !self.documents.is_empty() {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.documents.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }
    
    async fn search_documents(&mut self) -> Result<()> {
        let query = SearchQuery {
            ticker: if self.search_query.is_empty() { None } else { Some(self.search_query.clone()) },
            company_name: None,
            filing_type: None,
            source: None,
            date_from: None,
            date_to: None,
            text_query: None,
        };
        
        self.documents = storage::search_documents(&query, &self.database_path, 100).await?;
        
        // Reset list selection
        if !self.documents.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
        
        Ok(())
    }
}

pub async fn run_tui(database_path: &str) -> Result<()> {
    info!("Starting TUI interface");
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app state
    let mut app = App::new(database_path);
    
    // Load initial documents
    if let Err(e) = app.search_documents().await {
        info!("Failed to load initial documents: {}", e);
    }
    
    let result = run_app(&mut terminal, &mut app).await;
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    result
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.previous_tab(),
                    KeyCode::Down | KeyCode::Char('j') => app.next_document(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous_document(),
                    KeyCode::Char(c) => {
                        if matches!(app.state, AppState::Search) {
                            app.search_query.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        if matches!(app.state, AppState::Search) {
                            app.search_query.pop();
                        }
                    }
                    KeyCode::Enter => {
                        if matches!(app.state, AppState::Search) {
                            if let Err(e) = app.search_documents().await {
                                info!("Search failed: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let size = f.size();
    
    // Create layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(size);
    
    // Render tabs
    let titles: Vec<Line> = vec!["Search", "Documents", "Downloads"]
        .iter()
        .cloned()
        .map(Line::from)
        .collect();
    
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Fast10K TUI"))
        .select(app.tab_index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black)
        );
    
    f.render_widget(tabs, chunks[0]);
    
    // Render content based on current tab
    match app.state {
        AppState::Search => render_search_tab(f, app, chunks[1]),
        AppState::Documents => render_documents_tab(f, app, chunks[1]),
        AppState::Downloads => render_downloads_tab(f, app, chunks[1]),
    }
}

fn render_search_tab(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    
    // Search input
    let search_input = Paragraph::new(app.search_query.as_str())
        .block(Block::default().borders(Borders::ALL).title("Search (Enter to search, Tab to switch)"))
        .style(Style::default().fg(Color::Yellow));
    
    f.render_widget(search_input, chunks[0]);
    
    // Results
    render_document_list(f, app, chunks[1], "Search Results");
}

fn render_documents_tab(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    render_document_list(f, app, area, "All Documents");
}

fn render_downloads_tab(f: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let placeholder = Paragraph::new("Downloads monitoring not yet implemented\n\nPress 'q' to quit, Tab to switch tabs")
        .block(Block::default().borders(Borders::ALL).title("Downloads"))
        .style(Style::default().fg(Color::Gray));
    
    f.render_widget(placeholder, area);
}

fn render_document_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect, title: &str) {
    let items: Vec<ListItem> = app
        .documents
        .iter()
        .map(|doc| {
            let content = Line::from(vec![
                Span::styled(
                    format!("{} ", doc.ticker),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{} ", doc.company_name)),
                Span::styled(
                    format!("({}) ", doc.filing_type.as_str()),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    format!("[{}] ", doc.source.as_str()),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(doc.date.to_string()),
            ]);
            ListItem::new(content)
        })
        .collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    
    let mut list_state = app.list_state.clone();
    f.render_stateful_widget(list, area, &mut list_state);
}