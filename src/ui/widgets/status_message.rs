//! Status message widget for displaying success/info/error messages
//!
//! This module provides a status bar that shows temporary messages at the top of the screen.
//! Messages auto-hide after a configurable duration.

use crate::theme::{ACCENT, FAILED_TEXT, FG_BRIGHT, SUCCESS};
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::{Duration, Instant};

/// Message severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    /// Success message (green)
    _Success,
    /// Info message (blue/accent)
    Info,
    /// Error message (red/pink)
    _Error,
}

/// Status message with auto-hide functionality
pub struct StatusMessage {
    /// Message text
    text: String,
    /// Message level
    level: MessageLevel,
    /// Time when message was created
    created_at: Instant,
    /// Duration before auto-hide
    duration: Duration,
}

impl StatusMessage {
    /// Create a new status message
    pub fn new(text: impl Into<String>, level: MessageLevel) -> Self {
        Self {
            text: text.into(),
            level,
            created_at: Instant::now(),
            duration: Duration::from_secs(5),
        }
    }

    /// Create an info message
    pub fn info(text: impl Into<String>) -> Self {
        Self::new(text, MessageLevel::Info)
    }

    /// Check if the message should be hidden (expired)
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Render the status message as a paragraph
    pub fn render(&self) -> Paragraph<'_> {
        let (icon, color) = match self.level {
            MessageLevel::_Success => ("✓", SUCCESS),
            MessageLevel::Info => ("ℹ", ACCENT),
            MessageLevel::_Error => ("✗", FAILED_TEXT),
        };

        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", icon),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &self.text,
                Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
            ),
        ]);

        Paragraph::new(line).alignment(Alignment::Center)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_message_creation() {
        let msg = StatusMessage::success("Operation successful");
        assert_eq!(msg.text, "Operation successful");
        assert_eq!(msg.level, MessageLevel::_Success);
        assert!(!msg.is_expired());
    }

    #[test]
    fn test_message_levels() {
        let success = StatusMessage::success("Success");
        assert_eq!(success.level, MessageLevel::_Success);

        let info = StatusMessage::info("Info");
        assert_eq!(info.level, MessageLevel::Info);

        let error = StatusMessage::error("Error");
        assert_eq!(error.level, MessageLevel::_Error);
    }

    #[test]
    fn test_expiration() {
        let msg = StatusMessage::success("Test").with_duration(Duration::from_millis(10));
        assert!(!msg.is_expired());

        std::thread::sleep(Duration::from_millis(20));
        assert!(msg.is_expired());
    }
}
