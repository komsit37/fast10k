//! EDINET TUI binary entry point

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;
use tracing::{info, error};

use fast10k::{
    config::Config,
    edinet_tui::App,
    models::{SearchQuery, Source},
};

#[derive(Parser)]
#[command(name = "edinet-tui")]
#[command(about = "EDINET Terminal User Interface")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Run in CLI mode (print output and exit, no interactive TUI)
    #[arg(long, global = true)]
    pub cli: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search for documents by ticker symbol
    Search {
        /// Company ticker symbol
        #[arg(long)]
        sym: String,
    },
    /// Alias for search command
    S {
        /// Company ticker symbol
        #[arg(long)]
        sym: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Set default log level to INFO if not specified
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "edinet_tui=info,fast10k=info");
    }
    
    // Initialize logging to file for TUI mode to avoid interfering with display
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("edinet_tui.log")?;
    
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    info!("Starting EDINET TUI...");

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    // Handle CLI mode - print output and exit without TUI
    if cli.cli {
        if let Some(command) = cli.command {
            return handle_cli_command(command, &config).await;
        } else {
            eprintln!("Error: CLI mode requires a command");
            std::process::exit(1);
        }
    }

    // Setup terminal for TUI mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run the application
    let mut app = App::new(config)?;
    
    // Handle command line arguments for TUI mode
    if let Some(command) = cli.command {
        handle_startup_command(&mut app, command).await?;
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

    // Handle any errors that occurred during execution
    match result {
        Ok(_) => {
            info!("EDINET TUI exited successfully");
        }
        Err(e) => {
            error!("EDINET TUI encountered an error: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle CLI mode commands - print output and exit
async fn handle_cli_command(command: Commands, config: &Config) -> Result<()> {
    use fast10k::{storage, models::Document};
    
    match command {
        Commands::Search { sym } | Commands::S { sym } => {
            // Set up the search query
            let search_query = SearchQuery {
                ticker: Some(sym.clone()),
                company_name: None,
                filing_type: None,
                source: Some(Source::Edinet),
                date_from: None,
                date_to: None,
                text_query: None,
            };
            
            // Execute the search
            match storage::search_documents(&search_query, config.database_path_str(), 100).await {
                Ok(documents) => {
                    if documents.is_empty() {
                        println!("No documents found for symbol: {}", sym);
                    } else {
                        println!("Found {} documents for symbol: {}", documents.len(), sym);
                        println!();
                        println!("{:<12} {:<40} {:<15} {:<12} {:<20}", "Ticker", "Company", "Filing Type", "Date", "Path");
                        println!("{}", "-".repeat(100));
                        
                        for doc in &documents {
                            let ticker = &doc.ticker;
                            let company = truncate_string(&doc.company_name, 38);
                            let filing_type = doc.filing_type.as_str();
                            let date = doc.date.format("%Y-%m-%d").to_string();
                            let path = doc.content_path.to_str().unwrap_or("N/A");
                            
                            println!("{:<12} {:<40} {:<15} {:<12} {:<20}", 
                                ticker, company, filing_type, date, path);
                        }
                        
                        println!();
                        println!("Total: {} documents", documents.len());
                    }
                }
                Err(e) => {
                    eprintln!("Search failed for symbol {}: {}", sym, e);
                    std::process::exit(1);
                }
            }
        }
    }
    
    Ok(())
}

/// Truncate string to specified length with ellipsis
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Handle startup commands from command line arguments
async fn handle_startup_command(app: &mut App, command: Commands) -> Result<()> {
    use fast10k::{storage, edinet_tui::app::Screen};
    
    match command {
        Commands::Search { sym } | Commands::S { sym } => {
            info!("Executing search for symbol: {}", sym);
            
            // Set up the search query
            let search_query = SearchQuery {
                ticker: Some(sym.clone()),
                company_name: None,
                filing_type: None,
                source: Some(Source::Edinet),
                date_from: None,
                date_to: None,
                text_query: None,
            };
            
            // Pre-populate the search form
            app.search.ticker_input.value = sym.clone();
            
            // Execute the search
            match storage::search_documents(&search_query, app.config.database_path_str(), 100).await {
                Ok(documents) => {
                    info!("Found {} documents for symbol {}", documents.len(), sym);
                    app.set_status(format!("Found {} documents for {}", documents.len(), sym));
                    
                    // Store results and navigate to results screen
                    app.results.set_documents(documents);
                    app.search.last_query = Some(search_query);
                    app.navigate_to_screen(Screen::Results);
                }
                Err(e) => {
                    error!("Search failed for symbol {}: {}", sym, e);
                    app.set_error(format!("Search failed: {}", e));
                    // Stay on search screen with error message
                    app.navigate_to_screen(Screen::Search);
                }
            }
        }
    }
    
    Ok(())
}

/// Run the main application loop
async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    info!("Starting main application loop");
    
    // Run the application
    app.run(terminal).await?;
    
    info!("Application loop completed");
    Ok(())
}