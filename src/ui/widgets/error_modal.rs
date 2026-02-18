//! Error modal widget
//!
//! This module provides a modal popup for displaying API errors and other error messages.

use crate::theme::{ACCENT_WARN, BG_PANEL, BORDER_FOCUSED, FAILED_TEXT, FG_BRIGHT, FG_DIM, FG_PRIMARY};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Actions that can be returned from the error modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// No action taken yet
    None,
    /// User closed the modal (pressed Esc/Enter/c)
    Close,
    /// User wants to retry the operation (pressed r)
    Retry,
}

/// Modal popup for displaying errors
pub struct ErrorModal {
    /// The title of the modal
    title: String,
    /// The main error message
    error_message: String,
    /// Optional detailed error information
    details: Option<String>,
    /// Whether the modal is visible
    visible: bool,
    /// Whether details are expanded
    details_expanded: bool,
    /// Whether retry is available
    can_retry: bool,
}

impl ErrorModal {
    /// Create a new error modal with a title and error message
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the error modal (e.g., "API Error", "Error")
    /// * `error_message` - The main error message to display
    ///
    /// # Examples
    ///
    /// ```
    /// let modal = ErrorModal::new(
    ///     "API Error".to_string(),
    ///     "Failed to connect to CircleCI API".to_string()
    /// );
    /// ```
    pub fn new(title: String, error_message: String) -> Self {
        Self {
            title,
            error_message,
            details: None,
            visible: true,
            details_expanded: false,
            can_retry: false,
        }
    }

    /// Create a new error modal with title, message, and detailed information
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the error modal
    /// * `error_message` - The main error message
    /// * `details` - Detailed error information (stack trace, technical details, etc.)
    ///
    /// # Examples
    ///
    /// ```
    /// let modal = ErrorModal::with_details(
    ///     "API Error".to_string(),
    ///     "API returned error 404: Not Found".to_string(),
    ///     "GET /api/v2/pipeline/abc123\nResponse: {\"message\":\"not found\"}".to_string()
    /// );
    /// ```
    pub fn with_details(title: String, error_message: String, details: String) -> Self {
        Self {
            title,
            error_message,
            details: Some(details),
            visible: true,
            details_expanded: false,
            can_retry: false,
        }
    }

    /// Enable retry functionality for this error
    ///
    /// When enabled, the modal will show a [Retry] button and handle 'r' key press.
    pub fn with_retry(mut self) -> Self {
        self.can_retry = true;
        self
    }

    /// Handle keyboard input
    ///
    /// # Arguments
    ///
    /// * `key` - The key event to handle
    ///
    /// # Returns
    ///
    /// An ErrorAction indicating what action should be taken
    pub fn handle_input(&mut self, key: KeyEvent) -> ErrorAction {
        match key.code {
            KeyCode::Char('c') | KeyCode::Enter | KeyCode::Esc => ErrorAction::Close,
            KeyCode::Char('r') if self.can_retry => ErrorAction::Retry,
            KeyCode::Char('d') if self.details.is_some() => {
                self.details_expanded = !self.details_expanded;
                ErrorAction::None
            }
            _ => ErrorAction::None,
        }
    }

    /// Render the modal to the frame
    ///
    /// # Arguments
    ///
    /// * `f` - The frame to render to
    /// * `area` - The area to render within
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate modal height based on content
        let base_height = 12; // Minimum height
        let details_height = if self.details_expanded && self.details.is_some() {
            // Calculate how many lines the details will take
            let details_text = self.details.as_ref().unwrap();
            let lines = details_text.lines().count().max(3) as u16;
            lines.min(15) // Cap at 15 lines
        } else {
            0
        };
        let total_height = (base_height + details_height).min(80); // Cap at 80%

        // Calculate centered modal area (50% width, dynamic height)
        let modal_area = centered_rect(50, total_height, area);

        // Clear the background (dimmed effect)
        f.render_widget(Clear, modal_area);

        // Create the main block with error styling
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT_WARN))
            .style(Style::default().bg(BG_PANEL))
            .title(format!(" {} ", self.title))
            .title_style(Style::default().fg(FAILED_TEXT).add_modifier(Modifier::BOLD));

        let inner_area = block.inner(modal_area);
        f.render_widget(block, modal_area);

        // Split into sections: message, details (optional), buttons
        let mut constraints = vec![
            Constraint::Min(3), // Message area
        ];

        if self.details.is_some() {
            if self.details_expanded {
                constraints.push(Constraint::Min(5)); // Details area (expanded)
            } else {
                constraints.push(Constraint::Length(1)); // Details hint
            }
        }

        constraints.push(Constraint::Length(3)); // Buttons

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        // Render error message
        self.render_message(f, chunks[0]);

        // Render details section if present
        if self.details.is_some() {
            let details_idx = 1;
            self.render_details(f, chunks[details_idx]);
            self.render_buttons(f, chunks[details_idx + 1]);
        } else {
            self.render_buttons(f, chunks[1]);
        }
    }

    /// Render the error message section
    fn render_message(&self, f: &mut Frame, area: Rect) {
        let message_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "✗",
                Style::default().fg(FAILED_TEXT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                &self.error_message,
                Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
            )),
        ];

        let message_paragraph = Paragraph::new(message_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(message_paragraph, area);
    }

    /// Render the details section
    fn render_details(&self, f: &mut Frame, area: Rect) {
        if let Some(details) = &self.details {
            if self.details_expanded {
                // Show full details
                let details_paragraph = Paragraph::new(details.as_str())
                    .style(Style::default().fg(FG_DIM))
                    .wrap(Wrap { trim: true })
                    .block(
                        Block::default()
                            .borders(Borders::TOP)
                            .border_style(Style::default().fg(FG_DIM))
                            .title(" Details ")
                            .title_style(Style::default().fg(FG_DIM)),
                    );

                f.render_widget(details_paragraph, area);
            } else {
                // Show hint to expand details
                let hint = Line::from(vec![
                    Span::styled("Press ", Style::default().fg(FG_DIM)),
                    Span::styled("[d]", Style::default().fg(ACCENT_WARN).add_modifier(Modifier::BOLD)),
                    Span::styled(" to show details", Style::default().fg(FG_DIM)),
                ]);

                let hint_paragraph = Paragraph::new(hint).alignment(Alignment::Center);

                f.render_widget(hint_paragraph, area);
            }
        }
    }

    /// Render the buttons section
    fn render_buttons(&self, f: &mut Frame, area: Rect) {
        let mut button_spans = vec![];

        if self.can_retry {
            button_spans.extend(vec![
                Span::styled("[", Style::default().fg(FG_PRIMARY)),
                Span::styled("R", Style::default().fg(ACCENT_WARN).add_modifier(Modifier::BOLD)),
                Span::styled("etry]", Style::default().fg(ACCENT_WARN).add_modifier(Modifier::BOLD)),
                Span::styled("  ", Style::default()),
            ]);
        }

        button_spans.extend(vec![
            Span::styled("[", Style::default().fg(FG_PRIMARY)),
            Span::styled("C", Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD)),
            Span::styled("lose]", Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD)),
        ]);

        let buttons = Line::from(button_spans);

        let buttons_paragraph = Paragraph::new(vec![Line::from(""), buttons])
            .alignment(Alignment::Center);

        f.render_widget(buttons_paragraph, area);
    }

    /// Check if the modal is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Hide the modal
    pub fn hide(&mut self) {
        self.visible = false;
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_modal_creation() {
        let modal = ErrorModal::new("Error".to_string(), "Test error".to_string());
        assert!(modal.is_visible());
        assert!(!modal.details_expanded);
        assert!(!modal.can_retry);
        assert!(modal.details.is_none());
    }

    #[test]
    fn test_error_modal_with_details() {
        let modal = ErrorModal::with_details(
            "Error".to_string(),
            "Test error".to_string(),
            "Stack trace here".to_string(),
        );
        assert!(modal.is_visible());
        assert!(modal.details.is_some());
        assert!(!modal.details_expanded);
    }

    #[test]
    fn test_error_modal_with_retry() {
        let modal = ErrorModal::new("Error".to_string(), "Test error".to_string())
            .with_retry();
        assert!(modal.can_retry);
    }

    #[test]
    fn test_error_modal_input() {
        let mut modal = ErrorModal::new("Error".to_string(), "Test error".to_string());

        // Test Close action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(action, ErrorAction::Close);

        // Test Enter (Close)
        let action = modal.handle_input(KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, ErrorAction::Close);

        // Test Esc (Close)
        let action = modal.handle_input(KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, ErrorAction::Close);
    }

    #[test]
    fn test_error_modal_retry() {
        let mut modal = ErrorModal::new("Error".to_string(), "Test error".to_string())
            .with_retry();

        // Test Retry action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('r')));
        assert_eq!(action, ErrorAction::Retry);

        // Test that retry doesn't work without can_retry flag
        let mut modal_no_retry = ErrorModal::new("Error".to_string(), "Test error".to_string());
        let action = modal_no_retry.handle_input(KeyEvent::from(KeyCode::Char('r')));
        assert_eq!(action, ErrorAction::None);
    }

    #[test]
    fn test_error_modal_details_toggle() {
        let mut modal = ErrorModal::with_details(
            "Error".to_string(),
            "Test error".to_string(),
            "Details".to_string(),
        );

        // Initially not expanded
        assert!(!modal.details_expanded);

        // Toggle details
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('d')));
        assert_eq!(action, ErrorAction::None);
        assert!(modal.details_expanded);

        // Toggle again
        modal.handle_input(KeyEvent::from(KeyCode::Char('d')));
        assert!(!modal.details_expanded);
    }

    #[test]
    fn test_error_modal_hide() {
        let mut modal = ErrorModal::new("Error".to_string(), "Test error".to_string());
        assert!(modal.is_visible());

        modal.hide();
        assert!(!modal.is_visible());
    }
}
