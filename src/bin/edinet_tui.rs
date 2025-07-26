//! EDINET TUI binary entry point

use anyhow::Result;
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
};

#[tokio::main]
async fn main() -> Result<()> {
    // Set default log level to INFO if not specified
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "edinet_tui=info,fast10k=info");
    }
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting EDINET TUI...");

    // Load configuration
    let config = Config::from_env()?;
    config.validate()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run the application
    let mut app = App::new(config)?;
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