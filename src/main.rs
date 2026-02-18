//! CircleCI TUI - Main Entry Point
//!
//! A terminal user interface for interacting with CircleCI pipelines, workflows, and jobs.
//! This application provides a fast, keyboard-driven interface for monitoring CI/CD pipelines.

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;

mod theme;

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal);

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
    }

    Ok(())
}

/// Run the main application loop.
///
/// This function handles the event loop and rendering.
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|f| ui(f))?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only process key press events, not key release
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Draw the user interface.
///
/// For Phase 1, this simply displays a centered welcome message.
fn ui(f: &mut Frame) {
    let size = f.size();

    // Create a centered area for the message
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(10),
            Constraint::Percentage(50),
        ])
        .split(size);

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(vertical_chunks[1]);

    let center_area = horizontal_chunks[1];

    // Create the welcome message with cyberpunk styling
    let title = vec![
        Line::from(vec![
            Span::styled(
                "CircleCI TUI",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Press ",
                Style::default().fg(theme::FG_PRIMARY),
            ),
            Span::styled(
                "'q'",
                Style::default()
                    .fg(theme::ACCENT_DIM)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " or ",
                Style::default().fg(theme::FG_PRIMARY),
            ),
            Span::styled(
                "ESC",
                Style::default()
                    .fg(theme::ACCENT_DIM)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to quit",
                Style::default().fg(theme::FG_PRIMARY),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Phase 1: Terminal Setup Complete",
                Style::default()
                    .fg(theme::SUCCESS)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER_FOCUSED))
        .style(Style::default().bg(theme::BG_PANEL));

    let paragraph = Paragraph::new(title)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, center_area);

    // Add background
    let background = Block::default()
        .style(Style::default().bg(theme::BG_DARK));
    f.render_widget(background, size);
}
