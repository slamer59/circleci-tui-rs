//! CircleCI TUI - Main Entry Point
//!
//! A terminal user interface for interacting with CircleCI pipelines, workflows, and jobs.
//! This application provides a fast, keyboard-driven interface for monitoring CI/CD pipelines.

use anyhow::Result;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

mod api;
mod app;
mod cache;
mod config;
mod events;
mod git;
mod models;
mod preferences;
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
        // Process completed background tasks (non-blocking)
        app.process_bg_results();

        // Draw the UI
        terminal.draw(|f| app.render(f))?;

        // Check if we need to load logs (initial load for newly opened modal)
        if let Some(job_number) = app.pending_log_load.take() {
            app.spawn_log_load(job_number);
        }

        // Check if we need to refresh logs for streaming jobs
        if let Some(job_number) = app.should_refresh_logs() {
            // Mark refresh started to prevent duplicate spawns
            if let Some(modal) = &mut app.log_modal {
                modal.mark_refresh_started();
            }
            app.spawn_log_load(job_number);
        }

        // Check if we need to load workflows
        if let Some(pipeline_id) = app.pending_workflow_load.take() {
            app.spawn_workflow_load(pipeline_id);
        }

        // Check if we need to load jobs
        if let Some(workflow_id) = app.pending_job_load.take() {
            app.spawn_job_load(workflow_id);
        }

        // Check if we need to load more jobs (pagination)
        if let Some(workflow_id) = app.pending_load_more_jobs.take() {
            // Get page token before spawning
            let page_token = app
                .pipeline_detail_screen
                .as_ref()
                .and_then(|d| d.next_page_token.clone());
            if let Some(token) = page_token {
                app.spawn_more_jobs_load(workflow_id, token);
            }
        }

        // Check if pipeline detail screen needs to fetch logs for powerline
        if let Some(job_number) = app
            .pipeline_detail_screen
            .as_ref()
            .and_then(|detail| detail.pending_log_fetch)
        {
            app.spawn_powerline_log_load(job_number);
        }

        // Tick powerline to handle notification expiry
        app.tick_powerline();

        // Process prefetch results (non-blocking)
        app.process_prefetch_results();

        // Trigger prefetch for visible jobs (viewport-based)
        let terminal_height = terminal.size()?.height;
        app.trigger_prefetch(terminal_height);

        // Handle input events with a timeout (50ms for smooth animations)
        if event::poll(Duration::from_millis(50))? {
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
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app (now async)
    let mut app = match App::new(config).await {
        Ok(app) => app,
        Err(err) => {
            // Restore terminal before showing error
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture,
                cursor::Show
            )?;

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
            DisableMouseCapture,
            cursor::Show
        )?;

        eprintln!("Failed to load pipelines: {}", err);
        std::process::exit(1);
    }

    // Run app with async event loop
    let result = run_app(&mut app, &mut terminal).await;

    // Save preferences before exit
    if let Err(e) = app.save_preferences() {
        eprintln!("Warning: Failed to save preferences: {}", e);
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        cursor::Show
    )?;

    // Handle any errors
    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
        std::process::exit(1);
    }

    Ok(())
}
