//! Job logs modal widget
//!
//! This module provides a modal popup for displaying job logs with syntax highlighting,
//! scrolling, and interactive controls.

use crate::api::models::Job;
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_DARK, BG_PANEL, BORDER_FOCUSED, FG_BRIGHT,
    FG_DIM, FG_PRIMARY,
};
use ansi_to_tui::IntoText;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Instant;

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
    pub job: Job,
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
    /// Last rendered visible height (for accurate scroll calculations)
    last_visible_height: usize,
    /// Pending scroll to bottom on next render
    scroll_to_bottom_pending: bool,
    /// Loading progress: (current, total, step_name)
    load_progress: Option<(usize, usize, String)>,
    /// When the modal was created (for animations)
    created_at: Instant,
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
            last_visible_height: 1, // Will be updated on first render
            scroll_to_bottom_pending: false,
            load_progress: None,
            created_at: Instant::now(),
        }
    }

    /// Set the log lines to display
    pub fn set_logs(&mut self, logs: Vec<String>) {
        let prev_lines = self.log_lines.len();
        self.log_lines = logs;
        self.last_fetch = Instant::now();
        self.is_loading = false;
        self.load_progress = None;

        // Auto-scroll to bottom if:
        // 1. This is the initial load (prev_lines was 1 - the "Loading logs..." message)
        // 2. Auto-scroll is enabled (job is streaming and user hasn't manually scrolled up)
        if prev_lines <= 1 || self.auto_scroll {
            // Defer scroll to next render when we know the visible height
            self.scroll_to_bottom_pending = true;
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

    /// Mark that a refresh has been started (prevents duplicate spawns)
    pub fn mark_refresh_started(&mut self) {
        self.last_fetch = Instant::now();
    }

    /// Update loading progress
    pub fn set_progress(&mut self, current: usize, total: usize, step_name: String) {
        self.load_progress = Some((current, total, step_name));
    }

    /// Get the job number for this modal
    pub fn job_number(&self) -> u32 {
        self.job.job_number
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
                Constraint::Length(3), // Header (3 lines with timestamps)
                Constraint::Min(0),    // Logs (spinner renders here when loading)
                Constraint::Length(1), // Footer
            ])
            .split(inner_area);

        // Render header
        self.render_header(f, chunks[0]);

        // Render logs or loading spinner in the same area
        self.render_logs(f, chunks[1]);

        // Render footer - pass the log area height for accurate scroll calculation
        let log_area_height = chunks[1].height as usize;
        self.render_footer(f, chunks[2], log_area_height);
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
            Span::styled(
                self.job.duration_formatted(),
                Style::default().fg(FG_PRIMARY),
            ),
            Span::styled("  │  ", Style::default().fg(FG_DIM)),
            Span::styled("Executor: ", Style::default().fg(FG_DIM)),
            Span::styled(
                &self.job.executor.executor_type,
                Style::default().fg(FG_PRIMARY),
            ),
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
                Span::styled(format!("{}s ago", seconds_ago), Style::default().fg(ACCENT)),
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
    fn render_logs(&mut self, f: &mut Frame, area: Rect) {
        // If loading, show progress bar in the log area
        if self.is_loading {
            let spinner = self.spinner_char();
            let mut lines = Vec::new();
            let bar_width = (area.width as usize).saturating_sub(6).min(50);

            if let Some((current, total, ref step_name)) = self.load_progress {
                // Determinate progress bar: step-based
                let filled = if total > 0 {
                    (current * bar_width) / total
                } else {
                    0
                };
                let empty = bar_width.saturating_sub(filled);
                let percent = if total > 0 {
                    (current * 100) / total
                } else {
                    0
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", spinner),
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Fetching logs...", Style::default().fg(FG_PRIMARY)),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(" [", Style::default().fg(FG_DIM)),
                    Span::styled("━".repeat(filled), Style::default().fg(ACCENT)),
                    Span::styled("─".repeat(empty), Style::default().fg(FG_DIM)),
                    Span::styled(
                        format!("] {}/{}  {}%", current, total, percent),
                        Style::default().fg(FG_DIM),
                    ),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!(" {}", step_name),
                    Style::default().fg(FG_DIM),
                )));
            } else {
                // Indeterminate progress bar: sweeping back and forth
                let sweep_width = 8.min(bar_width);
                let cycle_len = bar_width.saturating_sub(sweep_width) + 1;
                // Time-based animation: ~1.5s per sweep, bounces back and forth
                let elapsed_ms = self.created_at.elapsed().as_millis() as usize;
                let cycle_ms = 1500;
                let phase = elapsed_ms % (cycle_ms * 2);
                let t = if phase < cycle_ms {
                    // Moving right
                    phase
                } else {
                    // Moving left (bounce)
                    cycle_ms * 2 - phase
                };
                let pos = if cycle_len > 0 && cycle_ms > 0 {
                    (t * cycle_len.saturating_sub(1)) / cycle_ms
                } else {
                    0
                };
                let before = pos;
                let after = bar_width.saturating_sub(pos + sweep_width);

                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{} ", spinner),
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Loading logs...", Style::default().fg(FG_PRIMARY)),
                ]));
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(" [", Style::default().fg(FG_DIM)),
                    Span::styled("─".repeat(before), Style::default().fg(FG_DIM)),
                    Span::styled("━".repeat(sweep_width), Style::default().fg(ACCENT)),
                    Span::styled("─".repeat(after), Style::default().fg(FG_DIM)),
                    Span::styled("]", Style::default().fg(FG_DIM)),
                ]));
            }

            let loading = Paragraph::new(lines).style(Style::default().bg(BG_DARK).fg(FG_PRIMARY));

            f.render_widget(loading, area);
            return;
        }

        let visible_height = area.height as usize;

        // Update last visible height for scroll calculations
        self.last_visible_height = visible_height;

        // If scroll to bottom is pending, do it now that we know the visible height
        if self.scroll_to_bottom_pending {
            let max_scroll = self.log_lines.len().saturating_sub(visible_height);
            self.scroll_offset = max_scroll;
            self.scroll_to_bottom_pending = false;
        }

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
                    let truncated_str: String =
                        line.chars().take(max_width.saturating_sub(1)).collect();
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
                            let spans: Vec<Span> = ansi_line
                                .spans
                                .iter()
                                .map(|ansi_span| {
                                    // Build ratatui Style from ansi-to-tui style components
                                    let mut style = Style::default();

                                    // Convert foreground color if present
                                    if let Some(fg) = ansi_span.style.fg {
                                        style = style.fg(Self::convert_color(fg));
                                    }

                                    // Convert background color if present
                                    if let Some(bg) = ansi_span.style.bg {
                                        style = style.bg(Self::convert_color(bg));
                                    }

                                    // Convert modifiers
                                    style = style.add_modifier(Self::convert_modifier(
                                        ansi_span.style.add_modifier,
                                    ));
                                    style = style.remove_modifier(Self::convert_modifier(
                                        ansi_span.style.sub_modifier,
                                    ));

                                    Span::styled(ansi_span.content.to_string(), style)
                                })
                                .collect();
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
    fn render_footer(&self, f: &mut Frame, area: Rect, log_area_height: usize) {
        let total_lines = self.log_lines.len();
        let visible_height = log_area_height;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let current_scroll = self.scroll_offset.min(max_scroll);

        // Calculate first and last visible line numbers (1-indexed for display)
        let first_line = current_scroll + 1;
        let last_line = (current_scroll + visible_height).min(total_lines);

        // Calculate scroll percentage
        // At bottom: scroll_offset >= max_scroll → 100%
        // At top: scroll_offset = 0 → depends on how much is visible
        let scroll_percent = if total_lines <= visible_height {
            // All content fits on screen
            100
        } else if current_scroll >= max_scroll {
            // At the bottom
            100
        } else if current_scroll == 0 {
            // At the top
            0
        } else {
            // In between: calculate based on scroll position
            ((current_scroll as f32 / max_scroll as f32) * 100.0) as u16
        };

        let footer_text = vec![Line::from(vec![
            Span::styled(
                "[Esc]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Close  ", Style::default().fg(FG_DIM)),
            Span::styled(
                "[↑↓⇞⇟]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Scroll  ", Style::default().fg(FG_DIM)),
            Span::styled(
                "[r]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Rerun  ", Style::default().fg(FG_DIM)),
            Span::styled("│ ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!(
                    "Lines {}-{}/{} ({}%)",
                    first_line, last_line, total_lines, scroll_percent
                ),
                Style::default().fg(ACCENT),
            ),
        ])];

        let footer = Paragraph::new(footer_text).style(Style::default().bg(BG_PANEL));

        f.render_widget(footer, area);
    }

    /// Convert ratatui-core Color to ratatui Color
    fn convert_color(color: ratatui_core::style::Color) -> ratatui::style::Color {
        use ratatui::style::Color;
        use ratatui_core::style::Color as CoreColor;

        match color {
            CoreColor::Reset => Color::Reset,
            CoreColor::Black => Color::Black,
            CoreColor::Red => Color::Red,
            CoreColor::Green => Color::Green,
            CoreColor::Yellow => Color::Yellow,
            CoreColor::Blue => Color::Blue,
            CoreColor::Magenta => Color::Magenta,
            CoreColor::Cyan => Color::Cyan,
            CoreColor::Gray => Color::Gray,
            CoreColor::DarkGray => Color::DarkGray,
            CoreColor::LightRed => Color::LightRed,
            CoreColor::LightGreen => Color::LightGreen,
            CoreColor::LightYellow => Color::LightYellow,
            CoreColor::LightBlue => Color::LightBlue,
            CoreColor::LightMagenta => Color::LightMagenta,
            CoreColor::LightCyan => Color::LightCyan,
            CoreColor::White => Color::White,
            CoreColor::Rgb(r, g, b) => Color::Rgb(r, g, b),
            CoreColor::Indexed(i) => Color::Indexed(i),
        }
    }

    /// Convert ratatui-core Modifier to ratatui Modifier
    fn convert_modifier(modifier: ratatui_core::style::Modifier) -> ratatui::style::Modifier {
        use ratatui::style::Modifier;
        use ratatui_core::style::Modifier as CoreModifier;

        let mut result = Modifier::empty();
        if modifier.contains(CoreModifier::BOLD) {
            result |= Modifier::BOLD;
        }
        if modifier.contains(CoreModifier::DIM) {
            result |= Modifier::DIM;
        }
        if modifier.contains(CoreModifier::ITALIC) {
            result |= Modifier::ITALIC;
        }
        if modifier.contains(CoreModifier::UNDERLINED) {
            result |= Modifier::UNDERLINED;
        }
        if modifier.contains(CoreModifier::SLOW_BLINK) {
            result |= Modifier::SLOW_BLINK;
        }
        if modifier.contains(CoreModifier::RAPID_BLINK) {
            result |= Modifier::RAPID_BLINK;
        }
        if modifier.contains(CoreModifier::REVERSED) {
            result |= Modifier::REVERSED;
        }
        if modifier.contains(CoreModifier::HIDDEN) {
            result |= Modifier::HIDDEN;
        }
        if modifier.contains(CoreModifier::CROSSED_OUT) {
            result |= Modifier::CROSSED_OUT;
        }
        result
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
                // Scroll to bottom using proper max scroll
                let max_scroll = self
                    .log_lines
                    .len()
                    .saturating_sub(self.last_visible_height);
                self.scroll_offset = max_scroll;
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
        // Use last_visible_height for accurate max scroll calculation
        let max_scroll = self
            .log_lines
            .len()
            .saturating_sub(self.last_visible_height);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
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
    use crate::api::models::ExecutorInfo;

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
}
