//! Spinner widget for loading states
//!
//! This module provides an animated spinner widget that can be used to indicate
//! loading or processing states in the UI.

use crate::theme::{ACCENT, FG_DIM, FG_PRIMARY};
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::time::{Duration, Instant};

/// Spinner frames for animation
const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Frame duration for spinner animation (100ms per frame)
const FRAME_DURATION: Duration = Duration::from_millis(100);

/// Animated spinner widget
pub struct Spinner {
    /// Current frame index
    current_frame: usize,
    /// Last update time
    last_update: Instant,
    /// Message to display next to spinner
    message: String,
    /// Time when spinner was created (for elapsed time tracking)
    created_at: Instant,
    /// Whether to show elapsed time
    show_elapsed: bool,
}

impl Spinner {
    /// Create a new spinner with the given message
    pub fn new(message: impl Into<String>) -> Self {
        let now = Instant::now();
        Self {
            current_frame: 0,
            last_update: now,
            message: message.into(),
            created_at: now,
            show_elapsed: false,
        }
    }

    /// Enable showing elapsed time
    pub fn with_elapsed_time(mut self) -> Self {
        self.show_elapsed = true;
        self
    }

    /// Update the spinner animation
    ///
    /// Call this periodically to advance the spinner animation.
    /// Returns true if the frame was updated.
    pub fn tick(&mut self) -> bool {
        if self.last_update.elapsed() >= FRAME_DURATION {
            self.current_frame = (self.current_frame + 1) % SPINNER_FRAMES.len();
            self.last_update = Instant::now();
            true
        } else {
            false
        }
    }

    /// Get the current spinner frame
    pub fn current_frame(&self) -> &'static str {
        SPINNER_FRAMES[self.current_frame]
    }

    /// Update the message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Get elapsed time in seconds
    fn elapsed_seconds(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }

    /// Render the spinner as a paragraph
    ///
    /// Returns a Paragraph widget that can be rendered to the terminal.
    pub fn render(&self) -> Paragraph<'_> {
        let mut spans = vec![
            Span::styled(
                format!("{} ", self.current_frame()),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.message, Style::default().fg(FG_PRIMARY)),
        ];

        // Add elapsed time if enabled
        if self.show_elapsed {
            let elapsed = self.elapsed_seconds();
            if elapsed > 0 {
                spans.push(Span::styled(
                    format!(" ({}s)", elapsed),
                    Style::default().fg(FG_DIM),
                ));
            }
        }

        let line = Line::from(spans);
        Paragraph::new(line).alignment(Alignment::Center)
    }

    /// Render the spinner with a border block
    ///
    /// Returns a Paragraph widget with a border block around it.
    pub fn render_with_block(&self, title: impl Into<String>) -> Paragraph<'_> {
        let line = Line::from(vec![
            Span::styled(
                format!("{} ", self.current_frame()),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.message, Style::default().fg(FG_PRIMARY)),
        ]);

        let block = Block::default()
            .title(title.into())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(FG_DIM));

        Paragraph::new(line)
            .block(block)
            .alignment(Alignment::Center)
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new("Loading...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let spinner = Spinner::new("Loading pipelines...");
        assert_eq!(spinner.current_frame, 0);
        assert_eq!(spinner.message, "Loading pipelines...");
        assert_eq!(spinner.current_frame(), SPINNER_FRAMES[0]);
    }

    #[test]
    fn test_spinner_tick() {
        let mut spinner = Spinner::new("Loading...");
        let initial_frame = spinner.current_frame();

        // Wait for frame duration
        std::thread::sleep(FRAME_DURATION + Duration::from_millis(10));

        // Tick should advance the frame
        let updated = spinner.tick();
        assert!(updated);
        assert_ne!(spinner.current_frame(), initial_frame);
    }

    #[test]
    fn test_spinner_wraps_around() {
        let mut spinner = Spinner::new("Loading...");

        // Advance through all frames
        for _ in 0..SPINNER_FRAMES.len() {
            spinner.current_frame = (spinner.current_frame + 1) % SPINNER_FRAMES.len();
        }

        // Should wrap back to 0
        assert_eq!(spinner.current_frame, 0);
    }

    #[test]
    fn test_set_message() {
        let mut spinner = Spinner::new("Loading...");
        spinner.set_message("Refreshing...");
        assert_eq!(spinner.message, "Refreshing...");
    }
}
