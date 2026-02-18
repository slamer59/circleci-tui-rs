//! Job logs modal widget
//!
//! This module provides a modal popup for displaying job logs with syntax highlighting,
//! scrolling, and interactive controls.

use crate::api::models::Job;
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BORDER_FOCUSED, BG_DARK, BG_PANEL, FG_BRIGHT,
    FG_DIM, FG_PRIMARY, FAILED_TEXT, SUCCESS,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Actions that can be triggered from the log modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalAction {
    /// No action taken
    None,
    /// Close the modal
    Close,
    /// Rerun the job
    Rerun,
    /// SSH into the job
    SSH,
}

/// Modal popup for displaying job logs
pub struct LogModal {
    /// The job being displayed
    job: Job,
    /// Log lines to display
    log_lines: Vec<String>,
    /// Current scroll offset
    scroll_offset: usize,
    /// Whether the modal is visible
    visible: bool,
}

impl LogModal {
    /// Create a new log modal for the given job
    pub fn new(job: Job) -> Self {
        let log_lines = Self::load_mock_logs(&job);
        Self {
            job,
            log_lines,
            scroll_offset: 0,
            visible: true,
        }
    }

    /// Render the modal to the frame
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate the centered modal area (80% width, 80% height)
        let modal_area = centered_rect(80, 80, area);

        // Clear the background with dimming effect
        f.render_widget(Clear, modal_area);

        // Create the main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .style(Style::default().bg(BG_PANEL))
            .title(format!(" JOB LOGS: {} ", self.job.name))
            .title_style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD));

        let inner_area = block.inner(modal_area);
        f.render_widget(block, modal_area);

        // Split the inner area into header, logs, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Header
                Constraint::Min(0),    // Logs
                Constraint::Length(1), // Footer
            ])
            .split(inner_area);

        // Render header
        self.render_header(f, chunks[0]);

        // Render logs
        self.render_logs(f, chunks[1]);

        // Render footer
        self.render_footer(f, chunks[2]);
    }

    /// Render the header section with job details
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let status_icon = get_status_icon(&self.job.status);
        let status_color = get_status_color(&self.job.status);

        let header_text = vec![
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(FG_DIM)),
                Span::styled(
                    format!("{} {}", self.job.status.to_uppercase(), status_icon),
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Duration: ", Style::default().fg(FG_DIM)),
                Span::styled(self.job.duration_formatted(), Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled("Executor: ", Style::default().fg(FG_DIM)),
                Span::styled(&self.job.executor.executor_type, Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(""),
        ];

        let header = Paragraph::new(header_text).style(Style::default().bg(BG_PANEL));

        f.render_widget(header, area);
    }

    /// Render the logs section with syntax highlighting
    fn render_logs(&self, f: &mut Frame, area: Rect) {
        let visible_height = area.height as usize;
        let max_scroll = self.log_lines.len().saturating_sub(visible_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        // Get the visible log lines
        let visible_lines: Vec<Line> = self
            .log_lines
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|line| self.highlight_log_line(line))
            .collect();

        let logs = Paragraph::new(visible_lines)
            .style(Style::default().bg(BG_DARK).fg(FG_PRIMARY))
            .wrap(Wrap { trim: false });

        f.render_widget(logs, area);
    }

    /// Render the footer with keybindings
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer_text = vec![Line::from(vec![
            Span::styled("[Esc]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Close  ", Style::default().fg(FG_DIM)),
            Span::styled(
                "[↑↓⇞⇟]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Scroll  ", Style::default().fg(FG_DIM)),
            Span::styled("[r]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Rerun  ", Style::default().fg(FG_DIM)),
            Span::styled("[s]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" SSH", Style::default().fg(FG_DIM)),
        ])];

        let footer = Paragraph::new(footer_text).style(Style::default().bg(BG_PANEL));

        f.render_widget(footer, area);
    }

    /// Apply syntax highlighting to a log line
    fn highlight_log_line<'a>(&self, line: &'a str) -> Line<'a> {
        // Command lines (starting with $)
        if line.trim_start().starts_with('$') {
            return Line::from(Span::styled(
                line,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
        }

        // Success lines (containing ✓ or "success")
        if line.contains('✓') || line.to_lowercase().contains("success") {
            return Line::from(Span::styled(line, Style::default().fg(SUCCESS)));
        }

        // Error lines (containing ✗ or "error"/"failed")
        if line.contains('✗')
            || line.to_lowercase().contains("error")
            || line.to_lowercase().contains("failed")
        {
            return Line::from(Span::styled(line, Style::default().fg(FAILED_TEXT)));
        }

        // Timestamp lines (starting with [)
        if line.trim_start().starts_with('[') {
            let parts: Vec<&str> = line.splitn(2, ']').collect();
            if parts.len() == 2 {
                return Line::from(vec![
                    Span::styled(
                        format!("{}]", parts[0]),
                        Style::default().fg(FG_DIM),
                    ),
                    Span::styled(parts[1], Style::default().fg(FG_PRIMARY)),
                ]);
            }
        }

        // Default styling
        Line::from(Span::styled(line, Style::default().fg(FG_PRIMARY)))
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> ModalAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ModalAction::Close,
            KeyCode::Char('r') => ModalAction::Rerun,
            KeyCode::Char('s') => ModalAction::SSH,
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
                ModalAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down();
                ModalAction::None
            }
            KeyCode::PageUp => {
                // Scroll up by 10 lines
                for _ in 0..10 {
                    self.scroll_up();
                }
                ModalAction::None
            }
            KeyCode::PageDown => {
                // Scroll down by 10 lines
                for _ in 0..10 {
                    self.scroll_down();
                }
                ModalAction::None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                ModalAction::None
            }
            KeyCode::End => {
                self.scroll_offset = self.log_lines.len().saturating_sub(1);
                ModalAction::None
            }
            _ => ModalAction::None,
        }
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self) {
        let max_scroll = self.log_lines.len().saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Load mock logs for the job
    fn load_mock_logs(job: &Job) -> Vec<String> {
        let status = job.status.to_lowercase();

        match status.as_str() {
            "failed" | "error" => Self::generate_failed_logs(job),
            "success" | "passed" => Self::generate_success_logs(job),
            "running" | "in_progress" => Self::generate_running_logs(job),
            _ => Self::generate_generic_logs(job),
        }
    }

    /// Generate mock logs for a failed job
    fn generate_failed_logs(job: &Job) -> Vec<String> {
        vec![
            "[16:55:01] Pulling Docker image...".to_string(),
            format!("[16:55:12] ✓ Image pulled: {}", job.executor.executor_type),
            "[16:55:15] Setting up environment variables".to_string(),
            "[16:55:16] ✓ Environment ready".to_string(),
            "".to_string(),
            format!("[16:55:17] $ cd /home/circleci/project && {}", job.name),
            format!("[16:55:18] Running {}...", job.name),
            "".to_string(),
            "[16:57:20] ✗ Connection timeout".to_string(),
            "           at puppeteer.js:142".to_string(),
            "           Failed to connect to".to_string(),
            "           http://localhost:3000".to_string(),
            "".to_string(),
            "[16:57:21] Error: Test suite failed to run".to_string(),
            "           ECONNREFUSED: Connection refused".to_string(),
            "".to_string(),
            "[16:57:30] ✗ Exit code: 1".to_string(),
            "".to_string(),
            "Job failed. Check the logs above for details.".to_string(),
        ]
    }

    /// Generate mock logs for a successful job
    fn generate_success_logs(job: &Job) -> Vec<String> {
        vec![
            "[14:20:01] Pulling Docker image...".to_string(),
            format!("[14:20:08] ✓ Image pulled: {}", job.executor.executor_type),
            "[14:20:10] Setting up environment variables".to_string(),
            "[14:20:11] ✓ Environment ready".to_string(),
            "".to_string(),
            format!("[14:20:12] $ cd /home/circleci/project && {}", job.name),
            format!("[14:20:13] Running {}...", job.name),
            "".to_string(),
            "[14:20:15] Installing dependencies...".to_string(),
            "[14:20:45] ✓ Dependencies installed".to_string(),
            "".to_string(),
            "[14:20:46] Running tests...".to_string(),
            "[14:21:12] ✓ All tests passed (127 tests)".to_string(),
            "".to_string(),
            "[14:21:15] Generating coverage report...".to_string(),
            "[14:21:18] ✓ Coverage: 94.2%".to_string(),
            "".to_string(),
            "[14:21:20] ✓ Exit code: 0".to_string(),
            "".to_string(),
            "Job completed successfully!".to_string(),
        ]
    }

    /// Generate mock logs for a running job
    fn generate_running_logs(job: &Job) -> Vec<String> {
        vec![
            "[18:30:01] Pulling Docker image...".to_string(),
            format!("[18:30:07] ✓ Image pulled: {}", job.executor.executor_type),
            "[18:30:09] Setting up environment variables".to_string(),
            "[18:30:10] ✓ Environment ready".to_string(),
            "".to_string(),
            format!("[18:30:11] $ cd /home/circleci/project && {}", job.name),
            format!("[18:30:12] Running {}...", job.name),
            "".to_string(),
            "[18:30:15] Installing dependencies...".to_string(),
            "[18:31:22] ✓ Dependencies installed".to_string(),
            "".to_string(),
            "[18:31:23] Building application...".to_string(),
            "[18:31:45] Compiling modules (65/120)...".to_string(),
            "".to_string(),
            "Job is currently running...".to_string(),
        ]
    }

    /// Generate generic mock logs
    fn generate_generic_logs(job: &Job) -> Vec<String> {
        vec![
            "[10:00:01] Preparing job environment...".to_string(),
            format!("[10:00:05] Executor: {}", job.executor.executor_type),
            format!("[10:00:06] Job: {}", job.name),
            "".to_string(),
            "[10:00:10] Waiting for resources...".to_string(),
            "".to_string(),
            format!("Status: {}", job.status),
        ]
    }
}

/// Calculate a centered rectangle with given percentage dimensions
///
/// # Arguments
///
/// * `percent_x` - Width as a percentage (0-100)
/// * `percent_y` - Height as a percentage (0-100)
/// * `r` - The parent rectangle
///
/// # Returns
///
/// A centered rectangle with the specified dimensions
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
    use crate::api::models::{ExecutorInfo, mock_data};

    fn create_test_job() -> Job {
        use chrono::{Duration, Utc};

        Job {
            id: "test-job-1".to_string(),
            name: "test-job".to_string(),
            status: "failed".to_string(),
            job_number: 123,
            workflow_id: "test-workflow".to_string(),
            started_at: Some(Utc::now() - Duration::minutes(5)),
            stopped_at: Some(Utc::now() - Duration::minutes(3)),
            duration: Some(135),
            executor: ExecutorInfo {
                executor_type: "docker".to_string(),
            },
        }
    }

    #[test]
    fn test_modal_creation() {
        let job = create_test_job();
        let modal = LogModal::new(job);
        assert!(modal.visible);
        assert_eq!(modal.scroll_offset, 0);
        assert!(!modal.log_lines.is_empty());
    }

    #[test]
    fn test_modal_actions() {
        let job = create_test_job();
        let mut modal = LogModal::new(job);

        // Test close action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, ModalAction::Close);

        // Test rerun action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('r')));
        assert_eq!(action, ModalAction::Rerun);

        // Test SSH action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('s')));
        assert_eq!(action, ModalAction::SSH);
    }

    #[test]
    fn test_scrolling() {
        let job = create_test_job();
        let mut modal = LogModal::new(job);

        let initial_offset = modal.scroll_offset;

        // Scroll down
        modal.scroll_down();
        assert!(modal.scroll_offset > initial_offset);

        // Scroll up
        modal.scroll_up();
        assert_eq!(modal.scroll_offset, initial_offset);

        // Can't scroll above 0
        modal.scroll_up();
        assert_eq!(modal.scroll_offset, 0);
    }

    #[test]
    fn test_centered_rect() {
        let parent = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(80, 80, parent);

        // Should be centered with 80% width and height
        assert_eq!(centered.width, 80);
        assert_eq!(centered.height, 80);
        assert_eq!(centered.x, 10);
        assert_eq!(centered.y, 10);
    }

    #[test]
    fn test_load_mock_logs_failed() {
        use chrono::{Duration, Utc};

        let job = Job {
            id: "test".to_string(),
            name: "test".to_string(),
            status: "failed".to_string(),
            job_number: 1,
            workflow_id: "test-workflow".to_string(),
            started_at: Some(Utc::now() - Duration::minutes(5)),
            stopped_at: Some(Utc::now() - Duration::minutes(3)),
            duration: Some(60),
            executor: ExecutorInfo {
                executor_type: "docker".to_string(),
            },
        };

        let logs = LogModal::load_mock_logs(&job);
        assert!(!logs.is_empty());
        assert!(logs.iter().any(|l| l.contains("✗")));
    }

    #[test]
    fn test_load_mock_logs_success() {
        use chrono::{Duration, Utc};

        let job = Job {
            id: "test".to_string(),
            name: "test".to_string(),
            status: "success".to_string(),
            job_number: 1,
            workflow_id: "test-workflow".to_string(),
            started_at: Some(Utc::now() - Duration::minutes(5)),
            stopped_at: Some(Utc::now() - Duration::minutes(3)),
            duration: Some(60),
            executor: ExecutorInfo {
                executor_type: "docker".to_string(),
            },
        };

        let logs = LogModal::load_mock_logs(&job);
        assert!(!logs.is_empty());
        assert!(logs.iter().any(|l| l.contains("✓")));
    }
}
