//! CircleCI TUI - Main Entry Point
//!
//! A terminal user interface for interacting with CircleCI pipelines, workflows, and jobs.
//! This application provides a fast, keyboard-driven interface for monitoring CI/CD pipelines.

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod api;
mod app;
mod config;
mod events;
mod models;
mod theme;
mod ui;

use app::App;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("Failed to load configuration: {}", err);
            eprintln!("\nPlease create a .env file with the following variables:");
            eprintln!("  CIRCLECI_TOKEN=your_token_here");
            eprintln!("  PROJECT_SLUG=gh/owner/repo");
            std::process::exit(1);
        }
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = match App::new(config) {
        Ok(app) => app,
        Err(err) => {
            // Restore terminal before showing error
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            eprintln!("Failed to initialize app: {}", err);
            std::process::exit(1);
        }
    };

    // Load initial data
    if let Err(err) = app.load_pipelines().await {
        // Restore terminal before showing error
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        eprintln!("Failed to load pipelines: {}", err);
        std::process::exit(1);
    }

    // Run app
    let result = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }

    Ok(())
}
