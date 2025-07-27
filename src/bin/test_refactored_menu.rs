//! Test binary to demonstrate the refactored MainMenuScreen
//!
//! This shows how the new architecture simplifies screen implementation
//! and provides consistent behavior.

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

use fast10k::{
    config::Config,
    edinet_tui::{
        screens::MainMenuScreenRefactored,
        traits::{Screen, ScreenAction},
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create refactored main menu screen
    let config = Config::from_env()?;
    let mut screen = MainMenuScreenRefactored::new();
    
    // Demonstrate customization
    screen.set_title(
        "EDINET TUI - Refactored Demo".to_string(),
        "Demonstrating the new component-based architecture".to_string()
    );
    
    screen.on_enter();

    let res = run_app(&mut terminal, &mut screen).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    screen: &mut MainMenuScreenRefactored,
) -> Result<()> {
    loop {
        // Draw
        terminal.draw(|f| screen.draw(f, f.size()))?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            match screen.handle_key_event(key).await? {
                ScreenAction::Quit => {
                    break;
                }
                ScreenAction::NavigateTo(screen_type) => {
                    // In a real app, this would navigate to the target screen
                    screen.status_mut().set_info(format!(
                        "Would navigate to: {:?}",
                        screen_type
                    ));
                }
                ScreenAction::NavigateBack => {
                    // Main menu doesn't support going back
                    screen.status_mut().set_warning(
                        "Cannot go back from main menu".to_string()
                    );
                }
                ScreenAction::SetStatus(msg) => {
                    screen.status_mut().set_info(msg);
                }
                ScreenAction::SetError(msg) => {
                    screen.status_mut().set_error(msg);
                }
                ScreenAction::ClearMessages => {
                    screen.status_mut().clear();
                }
                ScreenAction::None => {}
            }
        }
    }

    Ok(())
}