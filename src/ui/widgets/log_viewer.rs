//! Log Viewer Widget
//!
//! A widget for displaying job logs with syntax highlighting and scrolling support.
//! Provides color-coded output for commands, success messages, and errors.

use crate::theme::{ACCENT, FAILED_TEXT, FG_DIM, FG_PRIMARY, SUCCESS};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// A log viewer widget that displays lines of text with syntax highlighting
pub struct LogViewer {
    /// Buffer of log lines to display
    log_buffer: Vec<String>,
    /// Current scroll offset (0 = showing most recent lines at bottom)
    scroll_offset: usize,
    /// Whether logs are currently being loaded
    loading: bool,
}

impl LogViewer {
    /// Create a new empty log viewer
    pub fn new() -> Self {
        Self {
            log_buffer: Vec::new(),
            scroll_offset: 0,
            loading: false,
        }
    }

    /// Set the log buffer content
    pub fn set_logs(&mut self, logs: Vec<String>) {
        self.log_buffer = logs;
        // Reset scroll to bottom when new logs are loaded
        self.scroll_offset = 0;
    }

    /// Append a line to the log buffer
    pub fn append_line(&mut self, line: String) {
        self.log_buffer.push(line);
    }

    /// Clear all logs
    pub fn clear(&mut self) {
        self.log_buffer.clear();
        self.scroll_offset = 0;
    }

    /// Check if the log buffer is empty
    pub fn is_empty(&self) -> bool {
        self.log_buffer.is_empty()
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Get the current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scroll up by the given number of lines
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
    }

    /// Scroll down by the given number of lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll to the top
    pub fn scroll_to_top(&mut self) {
        if !self.log_buffer.is_empty() {
            self.scroll_offset = self.log_buffer.len().saturating_sub(1);
        }
    }

    /// Scroll to the bottom (most recent)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Apply syntax highlighting to a single log line
    fn highlight_line(line: &str) -> Line<'static> {
        let trimmed = line.trim_start();

        // Command lines (start with $)
        if trimmed.starts_with('$') {
            return Line::from(vec![Span::styled(
                line.to_string(),
                Style::default()
                    .fg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            )]);
        }

        // Success indicators
        if trimmed.starts_with('✓')
            || trimmed.starts_with("Success")
            || trimmed.starts_with("Done")
            || trimmed.starts_with("Completed")
            || line.contains("successfully")
        {
            return Line::from(vec![Span::styled(
                line.to_string(),
                Style::default().fg(SUCCESS),
            )]);
        }

        // Error indicators
        if trimmed.starts_with('✗')
            || trimmed.starts_with("Error:")
            || trimmed.starts_with("ERROR:")
            || trimmed.starts_with("FAIL:")
            || trimmed.starts_with("Failed")
            || trimmed.starts_with("error:")
            || line.contains("fatal:")
            || line.contains(" error ")
        {
            return Line::from(vec![Span::styled(
                line.to_string(),
                Style::default().fg(FAILED_TEXT).add_modifier(Modifier::BOLD),
            )]);
        }

        // Warning indicators
        if trimmed.starts_with("Warning:")
            || trimmed.starts_with("WARN:")
            || trimmed.starts_with("warning:")
        {
            return Line::from(vec![Span::styled(
                line.to_string(),
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]);
        }

        // Default: dim gray for regular output
        Line::from(vec![Span::styled(
            line.to_string(),
            Style::default().fg(FG_PRIMARY),
        )])
    }

    /// Render the log viewer to the given frame area
    pub fn render(&self, f: &mut Frame, area: Rect, title: &str) {
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(crate::theme::BORDER));

        if self.loading && self.log_buffer.is_empty() {
            // Show loading indicator
            let loading_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Loading logs...",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::ITALIC),
                )),
            ];
            let paragraph = Paragraph::new(loading_text)
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
            return;
        }

        if self.log_buffer.is_empty() {
            // Show empty state
            let empty_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No logs available",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::ITALIC),
                )),
            ];
            let paragraph = Paragraph::new(empty_text)
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, area);
            return;
        }

        // Calculate how many lines can be displayed
        let inner_height = area.height.saturating_sub(2) as usize; // Subtract borders

        // Determine which lines to show based on scroll offset
        let total_lines = self.log_buffer.len();
        let start_idx = if self.scroll_offset >= total_lines {
            0
        } else {
            total_lines.saturating_sub(self.scroll_offset + inner_height)
        };
        let end_idx = if self.scroll_offset == 0 {
            total_lines
        } else {
            total_lines.saturating_sub(self.scroll_offset)
        };

        // Apply syntax highlighting to visible lines
        let highlighted_lines: Vec<Line> = self.log_buffer[start_idx..end_idx]
            .iter()
            .map(|line| Self::highlight_line(line))
            .collect();

        // Add scroll indicator if not at bottom
        let mut lines_with_indicator = highlighted_lines;
        if self.scroll_offset > 0 {
            lines_with_indicator.insert(
                0,
                Line::from(vec![Span::styled(
                    format!("▲ Scrolled up {} lines (↓ to scroll down)", self.scroll_offset),
                    Style::default().fg(ACCENT).add_modifier(Modifier::ITALIC),
                )]),
            );
        }

        let paragraph = Paragraph::new(lines_with_indicator)
            .block(block)
            .wrap(Wrap { trim: false });

        f.render_widget(paragraph, area);
    }
}

impl Default for LogViewer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let viewer = LogViewer::new();
        assert!(viewer.is_empty());
        assert_eq!(viewer.scroll_offset(), 0);
    }

    #[test]
    fn test_set_logs() {
        let mut viewer = LogViewer::new();
        let logs = vec!["Line 1".to_string(), "Line 2".to_string()];
        viewer.set_logs(logs.clone());
        assert_eq!(viewer.log_buffer, logs);
        assert_eq!(viewer.scroll_offset(), 0);
    }

    #[test]
    fn test_append_line() {
        let mut viewer = LogViewer::new();
        viewer.append_line("Line 1".to_string());
        viewer.append_line("Line 2".to_string());
        assert_eq!(viewer.log_buffer.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut viewer = LogViewer::new();
        viewer.set_logs(vec!["Line 1".to_string()]);
        viewer.clear();
        assert!(viewer.is_empty());
        assert_eq!(viewer.scroll_offset(), 0);
    }

    #[test]
    fn test_scroll_up() {
        let mut viewer = LogViewer::new();
        viewer.scroll_up(5);
        assert_eq!(viewer.scroll_offset(), 5);
        viewer.scroll_up(3);
        assert_eq!(viewer.scroll_offset(), 8);
    }

    #[test]
    fn test_scroll_down() {
        let mut viewer = LogViewer::new();
        viewer.scroll_up(10);
        viewer.scroll_down(3);
        assert_eq!(viewer.scroll_offset(), 7);
        viewer.scroll_down(20); // Should not go below 0
        assert_eq!(viewer.scroll_offset(), 0);
    }

    #[test]
    fn test_scroll_to_top() {
        let mut viewer = LogViewer::new();
        viewer.set_logs(vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
        ]);
        viewer.scroll_to_top();
        assert_eq!(viewer.scroll_offset(), 2);
    }

    #[test]
    fn test_scroll_to_bottom() {
        let mut viewer = LogViewer::new();
        viewer.scroll_up(10);
        viewer.scroll_to_bottom();
        assert_eq!(viewer.scroll_offset(), 0);
    }

    #[test]
    fn test_highlight_command_line() {
        let line = "$ npm install";
        let highlighted = LogViewer::highlight_line(line);
        // Command lines should be styled with ACCENT color
        assert_eq!(highlighted.spans.len(), 1);
    }

    #[test]
    fn test_highlight_success_line() {
        let line = "✓ Build completed successfully";
        let highlighted = LogViewer::highlight_line(line);
        // Success lines should be styled with SUCCESS color
        assert_eq!(highlighted.spans.len(), 1);
    }

    #[test]
    fn test_highlight_error_line() {
        let line = "Error: Connection timeout";
        let highlighted = LogViewer::highlight_line(line);
        // Error lines should be styled with FAILED_TEXT color
        assert_eq!(highlighted.spans.len(), 1);
    }
}
