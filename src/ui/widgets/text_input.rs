//! Simple text input widget for search/filtering
//!
//! A lightweight text input field that can be focused, accepts typed text,
//! and supports basic cursor movement and editing.

use crate::theme;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// A simple text input widget
pub struct TextInput {
    /// The current text value
    value: String,
    /// Placeholder text when empty
    placeholder: String,
    /// Cursor position (index in value string)
    cursor_pos: usize,
    /// Whether this input is currently focused
    focused: bool,
    /// Configurable border sides
    borders: Borders,
}

impl TextInput {
    /// Create a new text input with a placeholder
    pub fn new(placeholder: &str) -> Self {
        Self {
            value: String::new(),
            placeholder: placeholder.to_string(),
            cursor_pos: 0,
            focused: false,
            borders: Borders::ALL, // Default to all borders
        }
    }

    /// Configure which borders to display (builder pattern)
    pub fn with_borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    /// Get the current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set the value programmatically
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor_pos = self.value.chars().count();
    }

    /// Clear the input
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_pos = 0;
    }

    /// Check if the input is empty
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        let char_count = self.value.chars().count();
        if focused && self.cursor_pos > char_count {
            self.cursor_pos = char_count;
        }
    }

    /// Check if focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Handle keyboard input
    ///
    /// Returns `true` if the key was handled, `false` otherwise.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        if !self.focused {
            return false;
        }

        match key {
            KeyCode::Char(c) => {
                // Use character-based indexing to handle UTF-8 safely
                let char_count = self.value.chars().count();
                if self.cursor_pos <= char_count {
                    let byte_pos = self.value.chars().take(self.cursor_pos).map(|c| c.len_utf8()).sum();
                    self.value.insert(byte_pos, c);
                    self.cursor_pos += 1;
                }
                true
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    let char_count = self.value.chars().count();
                    if self.cursor_pos <= char_count {
                        let byte_pos = self.value.chars().take(self.cursor_pos - 1).map(|c| c.len_utf8()).sum();
                        self.value.remove(byte_pos);
                        self.cursor_pos -= 1;
                    }
                }
                true
            }
            KeyCode::Delete => {
                let char_count = self.value.chars().count();
                if self.cursor_pos < char_count {
                    let byte_pos = self.value.chars().take(self.cursor_pos).map(|c| c.len_utf8()).sum();
                    self.value.remove(byte_pos);
                }
                true
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                true
            }
            KeyCode::Right => {
                let char_count = self.value.chars().count();
                if self.cursor_pos < char_count {
                    self.cursor_pos += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                true
            }
            KeyCode::End => {
                self.cursor_pos = self.value.chars().count();
                true
            }
            _ => false,
        }
    }

    /// Render the text input to the given frame area
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let text = if self.value.is_empty() && !self.focused {
            // Show placeholder
            Line::from(vec![Span::styled(
                &self.placeholder,
                Style::default()
                    .fg(theme::FG_DIM)
                    .add_modifier(Modifier::ITALIC),
            )])
        } else {
            // Show actual text with cursor
            let mut spans = vec![];

            if self.focused && self.cursor_pos == 0 {
                // Cursor at start
                spans.push(Span::styled(
                    "│",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            for (idx, ch) in self.value.chars().enumerate() {
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(theme::FG_BRIGHT),
                ));

                if self.focused && idx + 1 == self.cursor_pos {
                    // Cursor after this character
                    spans.push(Span::styled(
                        "│",
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }

            Line::from(spans)
        };

        let border_style = if self.focused {
            Style::default().fg(theme::BORDER_FOCUSED)
        } else {
            Style::default().fg(theme::BORDER)
        };

        let block = Block::default()
            .borders(self.borders)
            .border_style(border_style);

        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
    }

    /// Render the text input without borders (for use inside other panels)
    pub fn render_plain(&self, f: &mut Frame, area: Rect) {
        let text = if self.value.is_empty() && !self.focused {
            // Show placeholder
            Line::from(vec![Span::styled(
                &self.placeholder,
                Style::default()
                    .fg(theme::FG_DIM)
                    .add_modifier(Modifier::ITALIC),
            )])
        } else {
            // Show actual text with cursor
            let mut spans = vec![];

            if self.focused && self.cursor_pos == 0 {
                // Cursor at start
                spans.push(Span::styled(
                    "│",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            for (idx, ch) in self.value.chars().enumerate() {
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(theme::FG_BRIGHT),
                ));

                if self.focused && idx + 1 == self.cursor_pos {
                    // Cursor after this character
                    spans.push(Span::styled(
                        "│",
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }

            Line::from(spans)
        };

        let paragraph = Paragraph::new(text);
        f.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let input = TextInput::new("Placeholder");
        assert_eq!(input.value(), "");
        assert_eq!(input.placeholder, "Placeholder");
        assert_eq!(input.cursor_pos, 0);
        assert!(!input.is_focused());
    }

    #[test]
    fn test_set_value() {
        let mut input = TextInput::new("Placeholder");
        input.set_value("hello".to_string());
        assert_eq!(input.value(), "hello");
        assert_eq!(input.cursor_pos, 5);
    }

    #[test]
    fn test_clear() {
        let mut input = TextInput::new("Placeholder");
        input.set_value("hello".to_string());
        input.clear();
        assert_eq!(input.value(), "");
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_handle_char() {
        let mut input = TextInput::new("Placeholder");
        input.set_focused(true);

        input.handle_key(KeyCode::Char('h'));
        input.handle_key(KeyCode::Char('i'));
        assert_eq!(input.value(), "hi");
        assert_eq!(input.cursor_pos, 2);
    }

    #[test]
    fn test_handle_backspace() {
        let mut input = TextInput::new("Placeholder");
        input.set_focused(true);
        input.set_value("hello".to_string());

        input.handle_key(KeyCode::Backspace);
        assert_eq!(input.value(), "hell");
        assert_eq!(input.cursor_pos, 4);

        input.handle_key(KeyCode::Backspace);
        assert_eq!(input.value(), "hel");
        assert_eq!(input.cursor_pos, 3);
    }

    #[test]
    fn test_handle_delete() {
        let mut input = TextInput::new("Placeholder");
        input.set_focused(true);
        input.set_value("hello".to_string());
        input.cursor_pos = 0;

        input.handle_key(KeyCode::Delete);
        assert_eq!(input.value(), "ello");
        assert_eq!(input.cursor_pos, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = TextInput::new("Placeholder");
        input.set_focused(true);
        input.set_value("hello".to_string());

        input.handle_key(KeyCode::Home);
        assert_eq!(input.cursor_pos, 0);

        input.handle_key(KeyCode::Right);
        assert_eq!(input.cursor_pos, 1);

        input.handle_key(KeyCode::End);
        assert_eq!(input.cursor_pos, 5);

        input.handle_key(KeyCode::Left);
        assert_eq!(input.cursor_pos, 4);
    }

    #[test]
    fn test_unfocused_ignores_input() {
        let mut input = TextInput::new("Placeholder");
        // Don't set focused

        let handled = input.handle_key(KeyCode::Char('h'));
        assert!(!handled);
        assert_eq!(input.value(), "");
    }
}
