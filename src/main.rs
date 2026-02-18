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
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::Backend;
use std::time::Duration;

/// Run the application with async support for log streaming
async fn run_app<B: Backend>(app: &mut App, terminal: &mut Terminal<B>) -> Result<()> {
    loop {
        // Draw the UI
        terminal.draw(|f| app.render(f))?;

        // Check if we need to load logs (initial load for newly opened modal)
        if let Some(job_number) = app.pending_log_load {
            eprintln!("[DEBUG] Triggering log load for job #{}", job_number);
            app.pending_log_load = None;
            match app.load_job_logs(job_number).await {
                Ok(_) => eprintln!("[DEBUG] Successfully loaded logs for job #{}", job_number),
                Err(e) => eprintln!("[ERROR] Failed to load logs for job #{}: {}", job_number, e),
            }
        }

        // Check if we need to refresh logs for streaming jobs
        if let Some(job_number) = app.should_refresh_logs() {
            // Spawn a task to load logs without blocking the UI
            if let Err(e) = app.load_job_logs(job_number).await {
                // Silently ignore errors for now, could add error handling later
                eprintln!("Error loading logs: {}", e);
            }
        }

        // Check if we need to load workflows
        if let Some(pipeline_id) = app.should_load_workflows() {
            // Clear the pending flag
            app.pending_workflow_load = None;
            // Load workflows
            if let Err(e) = app.load_workflows(&pipeline_id).await {
                eprintln!("Error loading workflows: {}", e);
            }
        }

        // Check if we need to load jobs
        if let Some(workflow_id) = app.should_load_jobs() {
            // Clear the pending flag
            app.pending_job_load = None;
            // Load jobs
            if let Err(e) = app.load_jobs(&workflow_id).await {
                eprintln!("Error loading jobs: {}", e);
            }
        }

        // Handle input events with a timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events, not key release
                if key.kind == KeyEventKind::Press {
                    app.handle_event(key)?;
                }
            }
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

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

    // Run app with async event loop
    let result = run_app(&mut app, &mut terminal).await;

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
