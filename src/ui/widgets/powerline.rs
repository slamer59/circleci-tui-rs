//! Powerline status bar widget for displaying job context and notifications
//!
//! This module provides a persistent status bar that shows context-aware information
//! about the currently selected job and temporary notifications.

use crate::theme::{BG_DARK, FAILED_TEXT, SUCCESS};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::{Duration, Instant};

/// Content to display in the powerline
#[derive(Debug, Clone)]
pub enum PowerlineContent {
    /// Temporary notification message
    Notification {
        message: String,
        level: NotificationLevel,
        created_at: Instant,
        duration: Duration,
    },
    /// Loading state
    Loading { message: String },
    /// Empty state (nothing to show)
    Empty,
}

/// Notification severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Success message (✓)
    Success,
    /// Error message (✗)
    Error,
}

/// Powerline status bar widget
pub struct PowerlineBar {
    pub content: PowerlineContent,
}

impl PowerlineBar {
    /// Create a new powerline bar in empty state
    pub fn new() -> Self {
        Self {
            content: PowerlineContent::Empty,
        }
    }

    /// Set a temporary notification message
    pub fn set_notification(&mut self, message: String, level: NotificationLevel, duration: Duration) {
        self.content = PowerlineContent::Notification {
            message,
            level,
            created_at: Instant::now(),
            duration,
        };
    }

    /// Set loading state
    pub fn set_loading(&mut self, message: String) {
        self.content = PowerlineContent::Loading { message };
    }

    /// Clear content to empty state
    pub fn clear(&mut self) {
        self.content = PowerlineContent::Empty;
    }

    /// Update powerline state - clears expired notifications
    ///
    /// Returns true if the content changed (notification expired)
    pub fn tick(&mut self) -> bool {
        if let PowerlineContent::Notification {
            created_at,
            duration,
            ..
        } = &self.content
        {
            if created_at.elapsed() >= *duration {
                self.content = PowerlineContent::Empty;
                return true;
            }
        }
        false
    }

    /// Render the powerline to a frame
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let paragraph = match &self.content {
            PowerlineContent::Notification { message, level, .. } => {
                self.render_notification(message, *level)
            }

            PowerlineContent::Loading { message } => self.render_loading(message),

            PowerlineContent::Empty => return, // Don't render anything
        };

        frame.render_widget(paragraph, area);
    }

    /// Render notification content
    fn render_notification(&self, message: &str, level: NotificationLevel) -> Paragraph<'static> {
        let (icon, bg_color, fg_color) = match level {
            NotificationLevel::Success => ("✓", SUCCESS, BG_DARK),
            NotificationLevel::Error => ("✗", FAILED_TEXT, BG_DARK),
        };

        let style = Style::default()
            .bg(bg_color)
            .fg(fg_color)
            .add_modifier(Modifier::BOLD);

        let spans = vec![
            Span::raw(" ".to_string()),
            Span::styled(icon.to_string(), style),
            Span::raw(" ".to_string()),
            Span::styled(message.to_string(), style),
            Span::raw(" ".to_string()),
        ];

        Paragraph::new(Line::from(spans)).style(Style::default().bg(bg_color).fg(fg_color))
    }

    /// Render loading state
    fn render_loading(&self, message: &str) -> Paragraph<'static> {
        let style = Style::default()
            .bg(SUCCESS)
            .fg(BG_DARK)
            .add_modifier(Modifier::BOLD);

        let spans = vec![
            Span::raw(" ".to_string()),
            Span::styled("⠋".to_string(), style),
            Span::raw(" ".to_string()),
            Span::styled(message.to_string(), style),
            Span::raw(" ".to_string()),
        ];

        Paragraph::new(Line::from(spans)).style(Style::default().bg(SUCCESS).fg(BG_DARK))
    }

}

impl Default for PowerlineBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_powerline_creation() {
        let powerline = PowerlineBar::new();
        matches!(powerline.content, PowerlineContent::Empty);
    }

    #[test]
    fn test_set_notification() {
        let mut powerline = PowerlineBar::new();
        powerline.set_notification(
            "Copied 42 lines".to_string(),
            NotificationLevel::Success,
            Duration::from_secs(2),
        );

        match &powerline.content {
            PowerlineContent::Notification { message, level, .. } => {
                assert_eq!(message, "Copied 42 lines");
                assert_eq!(*level, NotificationLevel::Success);
            }
            _ => panic!("Expected Notification"),
        }
    }

    #[test]
    fn test_notification_expiry() {
        let mut powerline = PowerlineBar::new();
        powerline.set_notification(
            "Test".to_string(),
            NotificationLevel::Success,
            Duration::from_millis(10),
        );

        // Should not expire immediately
        assert!(!powerline.tick());

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(20));

        // Should expire and clear
        assert!(powerline.tick());
        matches!(powerline.content, PowerlineContent::Empty);
    }

    #[test]
    fn test_set_loading() {
        let mut powerline = PowerlineBar::new();
        powerline.set_loading("Loading logs...".to_string());

        match &powerline.content {
            PowerlineContent::Loading { message } => {
                assert_eq!(message, "Loading logs...");
            }
            _ => panic!("Expected Loading"),
        }
    }

    #[test]
    fn test_clear() {
        let mut powerline = PowerlineBar::new();
        powerline.set_loading("Loading...".to_string());
        powerline.clear();

        matches!(powerline.content, PowerlineContent::Empty);
    }
}
