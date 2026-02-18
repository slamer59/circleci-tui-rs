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
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Instant;
use ansi_to_tui::IntoText;

/// Actions that can be triggered from the log modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalAction {
    /// No action taken
    None,
    /// Close the modal
    Close,
    /// Rerun the job
    Rerun,
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
    /// Whether this job is streaming (running)
    is_streaming: bool,
    /// Last time logs were fetched
    last_fetch: Instant,
    /// Whether to auto-scroll to bottom
    auto_scroll: bool,
    /// Spinner animation frame (0-7)
    spinner_frame: usize,
    /// Whether logs are currently loading
    is_loading: bool,
}

impl LogModal {
    /// Create a new log modal for the given job
    ///
    /// Initial logs are empty - call `set_logs()` to populate them.
    pub fn new(job: Job) -> Self {
        let is_streaming = job.is_running();
        Self {
            job,
            log_lines: vec!["Loading logs...".to_string()],
            scroll_offset: 0,
            visible: true,
            is_streaming,
            last_fetch: Instant::now(),
            auto_scroll: is_streaming,
            spinner_frame: 0,
            is_loading: true,
        }
    }

    /// Set the log lines to display
    pub fn set_logs(&mut self, logs: Vec<String>) {
        eprintln!("[DEBUG] Setting logs: {} lines", logs.len());
        let prev_lines = self.log_lines.len();
        self.log_lines = logs;
        self.last_fetch = Instant::now();
        self.is_loading = false;
        eprintln!("[DEBUG] is_loading set to false, prev_lines={}, new_lines={}", prev_lines, self.log_lines.len());

        // Auto-scroll to bottom if:
        // 1. This is the initial load (prev_lines was 1 - the "Loading logs..." message)
        // 2. Auto-scroll is enabled (job is streaming and user hasn't manually scrolled up)
        if prev_lines <= 1 || self.auto_scroll {
            self.scroll_to_bottom();
            eprintln!("[DEBUG] Auto-scrolled to bottom");
        }
    }

    /// Advance spinner animation frame
    fn advance_spinner(&mut self) {
        const SPINNER_FRAMES_COUNT: usize = 10;
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES_COUNT;
    }

    /// Get current spinner character
    fn spinner_char(&self) -> &'static str {
        const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        SPINNER_FRAMES[self.spinner_frame % SPINNER_FRAMES.len()]
    }

    /// Check if logs should be refreshed (for streaming jobs)
    pub fn should_refresh(&self) -> bool {
        self.is_streaming && self.last_fetch.elapsed().as_secs() >= 2
    }

    /// Get the job number for this modal
    pub fn job_number(&self) -> u32 {
        self.job.job_number
    }

    /// Check if this job is streaming
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }

    /// Mark job as no longer streaming (completed)
    pub fn set_completed(&mut self) {
        eprintln!("[DEBUG] Job #{} marked as completed, stopping streaming", self.job.job_number);
        self.is_streaming = false;
        self.auto_scroll = false; // Stop auto-scrolling when job completes
    }

    /// Update the job information (useful when refreshing to check if job completed)
    pub fn update_job(&mut self, job: Job) {
        eprintln!("[DEBUG] Updating job #{} status: {}", job.job_number, job.status);
        let was_streaming = self.is_streaming;
        self.job = job.clone();
        self.is_streaming = job.is_running();

        // If job was streaming but is no longer running, mark as completed
        if was_streaming && !self.is_streaming {
            eprintln!("[DEBUG] Job #{} transitioned from running to {}", job.job_number, job.status);
            self.auto_scroll = false;
        }
    }

    /// Scroll to the bottom of the logs
    fn scroll_to_bottom(&mut self) {
        // Set scroll offset high enough to show the last lines
        // It will be clamped to max_scroll in render_logs
        self.scroll_offset = self.log_lines.len();
    }

    /// Render the modal to the frame
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Always advance spinner animation on each render for smooth animation
        // This ensures the spinner animates both during initial load AND during streaming
        self.advance_spinner();

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
                Constraint::Length(3),  // Header (3 lines with timestamps)
                Constraint::Min(0),     // Logs (spinner renders here when loading)
                Constraint::Length(1),  // Footer
            ])
            .split(inner_area);

        // Render header
        self.render_header(f, chunks[0]);

        // Render logs or loading spinner in the same area
        self.render_logs(f, chunks[1]);

        // Render footer
        self.render_footer(f, chunks[2]);
    }

    /// Render loading indicator with animated spinner
    fn render_loading_indicator(&self, f: &mut Frame, area: Rect) {
        let spinner = self.spinner_char();
        let loading_line = Line::from(vec![
            Span::styled(
                format!("{} ", spinner),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Loading logs...", Style::default().fg(FG_DIM)),
        ]);

        let loading = Paragraph::new(vec![loading_line])
            .style(Style::default().bg(BG_PANEL));

        f.render_widget(loading, area);
    }

    /// Render the header section with job details
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let status_icon = get_status_icon(&self.job.status);
        let status_color = get_status_color(&self.job.status);

        // Format timestamps
        let started_str = if let Some(started) = self.job.started_at {
            started.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "Not started".to_string()
        };

        let stopped_str = if let Some(stopped) = self.job.stopped_at {
            stopped.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "Still running".to_string()
        };

        // Build status line with streaming indicator
        let mut status_line_spans = vec![
            Span::styled("Status: ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!("{} {}", self.job.status.to_uppercase(), status_icon),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        // Add streaming indicator if job is running
        if self.is_streaming {
            status_line_spans.push(Span::styled("  ", Style::default()));
            status_line_spans.push(Span::styled(
                "● Live",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
        }

        status_line_spans.extend(vec![
            Span::styled("  │  ", Style::default().fg(FG_DIM)),
            Span::styled("Duration: ", Style::default().fg(FG_DIM)),
            Span::styled(self.job.duration_formatted(), Style::default().fg(FG_PRIMARY)),
            Span::styled("  │  ", Style::default().fg(FG_DIM)),
            Span::styled("Executor: ", Style::default().fg(FG_DIM)),
            Span::styled(&self.job.executor.executor_type, Style::default().fg(FG_PRIMARY)),
        ]);

        // Build second line with timestamps and last update
        let mut time_line_spans = vec![
            Span::styled("Started: ", Style::default().fg(FG_DIM)),
            Span::styled(started_str, Style::default().fg(FG_PRIMARY)),
            Span::styled("  │  ", Style::default().fg(FG_DIM)),
            Span::styled("Stopped: ", Style::default().fg(FG_DIM)),
            Span::styled(stopped_str, Style::default().fg(FG_PRIMARY)),
        ];

        // Add last update time for streaming logs
        if self.is_streaming {
            let seconds_ago = self.last_fetch.elapsed().as_secs();
            time_line_spans.extend(vec![
                Span::styled("  │  ", Style::default().fg(FG_DIM)),
                Span::styled("Updated: ", Style::default().fg(FG_DIM)),
                Span::styled(
                    format!("{}s ago", seconds_ago),
                    Style::default().fg(ACCENT),
                ),
            ]);
        }

        let header_text = vec![
            Line::from(status_line_spans),
            Line::from(time_line_spans),
            Line::from(""),
        ];

        let header = Paragraph::new(header_text).style(Style::default().bg(BG_PANEL));

        f.render_widget(header, area);
    }

    /// Render the logs section with ANSI color support
    fn render_logs(&self, f: &mut Frame, area: Rect) {
        // If loading, show spinner in the center of the log area
        if self.is_loading {
            let spinner = self.spinner_char();
            let loading_line = Line::from(vec![
                Span::styled(
                    format!("{} ", spinner),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Loading logs...", Style::default().fg(FG_DIM)),
            ]);

            let loading = Paragraph::new(vec![loading_line])
                .style(Style::default().bg(BG_DARK).fg(FG_PRIMARY));

            f.render_widget(loading, area);
            return;
        }

        let visible_height = area.height as usize;

        // Get all log lines (no filtering needed since loading is handled above)
        let display_lines: Vec<&String> = self.log_lines.iter().collect();

        let max_scroll = display_lines.len().saturating_sub(visible_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);

        // Get available width for truncation (area width)
        let max_width = area.width as usize;

        // Get the visible log lines with ANSI parsing and truncation
        let visible_lines: Vec<Line> = display_lines
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .map(|line| {
                // Truncate line to fit within the available width
                // Use char-based truncation to avoid splitting multi-byte UTF-8 characters
                let truncated = if line.chars().count() > max_width {
                    let truncated_str: String = line.chars()
                        .take(max_width.saturating_sub(1))
                        .collect();
                    format!("{}…", truncated_str)
                } else {
                    line.to_string()
                };

                // Parse ANSI codes and convert to Ratatui styled text
                match truncated.as_str().into_text() {
                    Ok(text) => {
                        // Convert ansi-to-tui Line to ratatui Line
                        if text.lines.is_empty() {
                            Line::from(truncated)
                        } else {
                            // Extract spans from ansi-to-tui Line and create ratatui Line
                            let ansi_line = &text.lines[0];
                            let spans: Vec<Span> = ansi_line.spans.iter().map(|ansi_span| {
                                // Convert ansi-to-tui Span to ratatui Span
                                // Just use the content - colors will be parsed but basic styling is enough
                                Span::raw(ansi_span.content.to_string())
                            }).collect();
                            Line::from(spans)
                        }
                    }
                    Err(_) => {
                        // Fallback to plain text if ANSI parsing fails
                        Line::from(truncated)
                    }
                }
            })
            .collect();

        let logs = Paragraph::new(visible_lines)
            .style(Style::default().bg(BG_DARK).fg(FG_PRIMARY))
            .scroll((0, 0)); // Disable automatic scrolling

        f.render_widget(logs, area);
    }

    /// Render the footer with keybindings and scroll indicator
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let visible_height = area.height as usize;
        let max_scroll = self.log_lines.len().saturating_sub(visible_height);
        let current_line = self.scroll_offset.min(max_scroll) + 1;
        let total_lines = self.log_lines.len();

        // Calculate scroll percentage for progress bar
        let scroll_percent = if total_lines > 0 {
            ((current_line as f32 / total_lines as f32) * 100.0) as u16
        } else {
            0
        };

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
            Span::styled("│ ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!("Scroll: {}/{} lines ({}%)", current_line, total_lines, scroll_percent),
                Style::default().fg(ACCENT),
            ),
        ])];

        let footer = Paragraph::new(footer_text).style(Style::default().bg(BG_PANEL));

        f.render_widget(footer, area);
    }

    /// Apply syntax highlighting to a log line (returns owned Line)
    fn highlight_log_line(&self, line: &str) -> Line<'static> {
        // Command lines (starting with $)
        if line.trim_start().starts_with('$') {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
        }

        // Success lines (containing ✓ or "success")
        if line.contains('✓') || line.to_lowercase().contains("success") {
            return Line::from(Span::styled(line.to_string(), Style::default().fg(SUCCESS)));
        }

        // Error lines (containing ✗ or "error"/"failed")
        if line.contains('✗')
            || line.to_lowercase().contains("error")
            || line.to_lowercase().contains("failed")
        {
            return Line::from(Span::styled(line.to_string(), Style::default().fg(FAILED_TEXT)));
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
                    Span::styled(parts[1].to_string(), Style::default().fg(FG_PRIMARY)),
                ]);
            }
        }

        // Default styling
        Line::from(Span::styled(line.to_string(), Style::default().fg(FG_PRIMARY)))
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> ModalAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ModalAction::Close,
            KeyCode::Char('r') => ModalAction::Rerun,
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
                // Disable auto-scroll when user manually scrolls up
                self.auto_scroll = false;
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
                // Disable auto-scroll when user manually scrolls up
                self.auto_scroll = false;
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
                // Disable auto-scroll when user manually scrolls
                self.auto_scroll = false;
                ModalAction::None
            }
            KeyCode::End => {
                self.scroll_offset = self.log_lines.len().saturating_sub(1);
                // Re-enable auto-scroll when user scrolls to end
                if self.is_streaming {
                    self.auto_scroll = true;
                }
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
    }

    #[test]
    fn test_scrolling() {
        let job = create_test_job();
        let mut modal = LogModal::new(job);

        // Add some logs to enable scrolling
        modal.set_logs(vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
            "Line 4".to_string(),
            "Line 5".to_string(),
        ]);

        // set_logs auto-scrolls to bottom, so start from top
        modal.scroll_offset = 0;
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
