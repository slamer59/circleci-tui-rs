//! Confirmation modal widget
//!
//! This module provides a modal popup for confirming actions (e.g., rerunning workflows).

use crate::theme::{ACCENT, BG_PANEL, BORDER_FOCUSED, FG_BRIGHT, FG_DIM, FG_PRIMARY};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Actions that can be returned from the confirmation modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    /// No action taken yet
    None,
    /// User confirmed the action (pressed Yes/Enter/y)
    Yes,
    /// User cancelled the action (pressed No/Esc/n)
    No,
}

/// Modal popup for confirming actions
pub struct ConfirmModal {
    /// The message to display
    message: String,
    /// Whether the modal is visible
    visible: bool,
    /// Currently selected button (0 = Yes, 1 = No)
    selected_button: usize,
}

impl ConfirmModal {
    /// Create a new confirmation modal with a message
    pub fn new(message: String) -> Self {
        Self {
            message,
            visible: true,
            selected_button: 0, // Default to "Yes" button
        }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> ConfirmAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => ConfirmAction::Yes,
            KeyCode::Char('n') | KeyCode::Esc => ConfirmAction::No,
            KeyCode::Left => {
                self.selected_button = 0;
                ConfirmAction::None
            }
            KeyCode::Right | KeyCode::Tab => {
                self.selected_button = 1;
                ConfirmAction::None
            }
            _ => ConfirmAction::None,
        }
    }

    /// Render the modal to the frame
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate centered modal area (50% width, 30% height minimum)
        let modal_area = centered_rect(50, 30, area);

        // Clear the background
        f.render_widget(Clear, modal_area);

        // Create the main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .style(Style::default().bg(BG_PANEL))
            .title(" CONFIRM ACTION ")
            .title_style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD));

        let inner_area = block.inner(modal_area);
        f.render_widget(block, modal_area);

        // Split into message and buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Message area
                Constraint::Length(3), // Buttons
            ])
            .split(inner_area);

        // Render message
        let message_paragraph = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                &self.message,
                Style::default().fg(FG_PRIMARY).add_modifier(Modifier::BOLD),
            )),
        ])
        .alignment(Alignment::Center);

        f.render_widget(message_paragraph, chunks[0]);

        // Render buttons
        self.render_buttons(f, chunks[1]);
    }

    /// Render the buttons section
    fn render_buttons(&self, f: &mut Frame, area: Rect) {
        let yes_style = if self.selected_button == 0 {
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(FG_DIM)
        };

        let no_style = if self.selected_button == 1 {
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(FG_DIM)
        };

        let buttons = Line::from(vec![
            Span::styled("[", Style::default().fg(FG_PRIMARY)),
            Span::styled("Y", yes_style),
            Span::styled("es]", yes_style),
            Span::styled("  ", Style::default()),
            Span::styled("[", Style::default().fg(FG_PRIMARY)),
            Span::styled("N", no_style),
            Span::styled("o]", no_style),
        ]);

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
    fn test_confirm_modal_creation() {
        let modal = ConfirmModal::new("Test message".to_string());
        assert!(modal.is_visible());
        assert_eq!(modal.selected_button, 0);
    }

    #[test]
    fn test_confirm_modal_input() {
        let mut modal = ConfirmModal::new("Test".to_string());

        // Test Yes action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('y')));
        assert_eq!(action, ConfirmAction::Yes);

        // Test No action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('n')));
        assert_eq!(action, ConfirmAction::No);

        // Test Enter (Yes)
        let action = modal.handle_input(KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, ConfirmAction::Yes);

        // Test Esc (No)
        let action = modal.handle_input(KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, ConfirmAction::No);
    }

    #[test]
    fn test_button_navigation() {
        let mut modal = ConfirmModal::new("Test".to_string());

        // Initially on Yes button
        assert_eq!(modal.selected_button, 0);

        // Navigate to No button
        modal.handle_input(KeyEvent::from(KeyCode::Right));
        assert_eq!(modal.selected_button, 1);

        // Navigate back to Yes button
        modal.handle_input(KeyEvent::from(KeyCode::Left));
        assert_eq!(modal.selected_button, 0);
    }
}
