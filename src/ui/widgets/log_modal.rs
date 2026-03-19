//! Job logs modal widget with collapsible step-based log viewer
//!
//! This module provides a modal popup for displaying job logs organized by steps,
//! with expand/collapse functionality, syntax highlighting, scrolling, and interactive controls.

use crate::api::models::Job;
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_DARK, BG_PANEL, BORDER_FOCUSED, FAILED_TEXT,
    FG_BRIGHT, FG_DIM, FG_PRIMARY, SUCCESS,
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
    /// Copy selected step's logs to clipboard
    CopyStepLogs,
}

/// A single step within a job's log output
pub struct LogStep {
    /// Name of the step
    pub name: String,
    /// Status: "success", "failed", "running", "pending"
    pub status: String,
    /// Duration in seconds, if known
    pub duration_secs: Option<u32>,
    /// Log lines for this step
    pub logs: Vec<String>,
    /// Whether this step is expanded (showing log lines)
    pub expanded: bool,
    /// Whether logs for this step are still being fetched
    pub is_loading: bool,
}

/// Modal popup for displaying job logs organized by steps
pub struct LogModal {
    /// The job being displayed
    pub job: Job,
    /// Steps in the job
    steps: Vec<LogStep>,
    /// Which step is currently selected
    selected_step: usize,
    /// Scroll offset within the entire rendered virtual view
    scroll_offset: usize,
    /// Whether the modal is visible
    visible: bool,
    /// Whether this job is streaming (running)
    is_streaming: bool,
    /// Last time logs were fetched
    last_fetch: Instant,
    /// Whether to auto-scroll to bottom
    auto_scroll: bool,
    /// Spinner animation frame (0-9)
    spinner_frame: usize,
    /// Whether logs are currently loading (initial metadata fetch phase)
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
    /// Initial state is loading - call `set_steps()` or `set_logs()` to populate.
    pub fn new(job: Job) -> Self {
        let is_streaming = job.is_running();
        Self {
            job,
            steps: Vec::new(),
            selected_step: 0,
            scroll_offset: 0,
            visible: true,
            is_streaming,
            last_fetch: Instant::now(),
            auto_scroll: is_streaming,
            spinner_frame: 0,
            is_loading: true,
            last_visible_height: 1,
            scroll_to_bottom_pending: false,
            load_progress: None,
            created_at: Instant::now(),
        }
    }

    /// Set step metadata. Creates LogStep entries with empty logs and is_loading: true.
    /// Auto-expands failed steps. Sets is_loading = false (steps are now visible).
    pub fn set_steps(&mut self, steps: Vec<(String, String)>) {
        self.steps = steps
            .into_iter()
            .map(|(name, status)| {
                let expanded = status == "failed";
                LogStep {
                    name,
                    status,
                    duration_secs: None,
                    logs: Vec::new(),
                    expanded,
                    is_loading: true,
                }
            })
            .collect();
        self.is_loading = false;
        self.load_progress = None;
        // Clamp selected_step
        if !self.steps.is_empty() && self.selected_step >= self.steps.len() {
            self.selected_step = self.steps.len() - 1;
        }
    }

    /// Fill in logs for a specific step and mark it as loaded.
    pub fn set_step_logs(&mut self, step_index: usize, logs: Vec<String>) {
        if let Some(step) = self.steps.get_mut(step_index) {
            step.logs = logs;
            step.is_loading = false;
        }
    }

    /// Set the log lines to display (backward compat for cache hit path).
    /// When called and steps is empty, creates a single "All Logs" step with all lines, expanded.
    pub fn set_logs(&mut self, logs: Vec<String>) {
        if self.steps.is_empty() {
            self.steps = vec![LogStep {
                name: "All Logs".to_string(),
                status: "success".to_string(),
                duration_secs: None,
                logs,
                expanded: true,
                is_loading: false,
            }];
            self.selected_step = 0;
        } else {
            // If steps already exist, put all logs into the first step
            if let Some(step) = self.steps.first_mut() {
                step.logs = logs;
                step.is_loading = false;
            }
        }
        self.is_loading = false;
        self.load_progress = None;
        self.last_fetch = Instant::now();

        // Auto-scroll to bottom on initial load
        self.scroll_to_bottom_pending = true;
    }

    /// Append log lines incrementally (for progressive loading).
    /// Appends to the first step, or creates an "All Logs" step if none exist.
    /// Mark that all log chunks have been received
    pub fn mark_loading_complete(&mut self) {
        self.is_loading = false;
        self.load_progress = None;
        self.last_fetch = Instant::now();
        // Also mark all steps as loaded
        for step in &mut self.steps {
            step.is_loading = false;
        }
    }

    /// Advance spinner animation frame
    pub fn advance_spinner(&mut self) {
        const SPINNER_FRAMES_COUNT: usize = 10;
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES_COUNT;
    }

    /// Get current spinner character
    pub fn spinner_char(&self) -> &'static str {
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
    /// Get the job number for this modal
    pub fn job_number(&self) -> u32 {
        self.job.job_number
    }

    /// Get a reference to the steps (for caching assembled logs)
    pub fn steps_ref(&self) -> &[LogStep] {
        &self.steps
    }

    // --- Virtual view helpers ---

    /// Total number of rows in the virtual view (step headers + expanded log lines)
    fn total_virtual_rows(&self) -> usize {
        let mut total = 0;
        for step in &self.steps {
            total += 1; // step header row
            if step.expanded {
                if step.is_loading {
                    total += 1; // loading indicator row
                } else {
                    total += step.logs.len().max(1); // at least 1 row for empty logs
                }
            }
        }
        total
    }

    /// Get the virtual row index of a given step's header
    fn step_header_row(&self, step_index: usize) -> usize {
        let mut row = 0;
        for (i, step) in self.steps.iter().enumerate() {
            if i == step_index {
                return row;
            }
            row += 1;
            if step.expanded {
                if step.is_loading {
                    row += 1;
                } else {
                    row += step.logs.len().max(1);
                }
            }
        }
        row
    }

    /// Format duration_secs into a human-readable string
    fn format_duration(secs: u32) -> String {
        if secs >= 60 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}s", secs)
        }
    }

    // --- ANSI parsing helpers ---

    /// Parse a single log line with ANSI codes into a ratatui Line, truncated to max_width
    fn parse_ansi_line(line: &str, max_width: usize) -> Line<'static> {
        let truncated = if line.chars().count() > max_width {
            let truncated_str: String = line.chars().take(max_width.saturating_sub(1)).collect();
            format!("{}…", truncated_str)
        } else {
            line.to_string()
        };

        match truncated.as_str().into_text() {
            Ok(text) => {
                if text.lines.is_empty() {
                    Line::from(truncated)
                } else {
                    let ansi_line = &text.lines[0];
                    let spans: Vec<Span> = ansi_line
                        .spans
                        .iter()
                        .map(|ansi_span| {
                            let mut style = Style::default();
                            if let Some(fg) = ansi_span.style.fg {
                                style = style.fg(Self::convert_color(fg));
                            }
                            if let Some(bg) = ansi_span.style.bg {
                                style = style.bg(Self::convert_color(bg));
                            }
                            style = style
                                .add_modifier(Self::convert_modifier(ansi_span.style.add_modifier));
                            style = style.remove_modifier(Self::convert_modifier(
                                ansi_span.style.sub_modifier,
                            ));
                            Span::styled(ansi_span.content.to_string(), style)
                        })
                        .collect();
                    Line::from(spans)
                }
            }
            Err(_) => Line::from(truncated),
        }
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

    // --- Rendering ---

    /// Render the modal to the frame
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Always advance spinner animation on each render for smooth animation
        self.advance_spinner();

        let modal_area = centered_rect(80, 80, area);

        // Clear the background
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

        // Split into header, steps/logs, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Steps/Logs
                Constraint::Length(1), // Footer
            ])
            .split(inner_area);

        self.render_header(f, chunks[0]);
        self.render_steps(f, chunks[1]);

        let log_area_height = chunks[1].height as usize;
        self.render_footer(f, chunks[2], log_area_height);
    }

    /// Render the header section with job details
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let status_icon = get_status_icon(&self.job.status);
        let status_color = get_status_color(&self.job.status);

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

        let mut status_line_spans = vec![
            Span::styled("Status: ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!("{} {}", self.job.status.to_uppercase(), status_icon),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

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

        let mut time_line_spans = vec![
            Span::styled("Started: ", Style::default().fg(FG_DIM)),
            Span::styled(started_str, Style::default().fg(FG_PRIMARY)),
            Span::styled("  │  ", Style::default().fg(FG_DIM)),
            Span::styled("Stopped: ", Style::default().fg(FG_DIM)),
            Span::styled(stopped_str, Style::default().fg(FG_PRIMARY)),
        ];

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

    /// Render the step-based log view (replaces render_logs)
    fn render_steps(&mut self, f: &mut Frame, area: Rect) {
        // If loading (before steps discovered), show sweeping progress bar
        if self.is_loading {
            self.render_loading_bar(f, area);
            return;
        }

        // If no steps at all, show empty message
        if self.steps.is_empty() {
            let empty = Paragraph::new("No log steps available.")
                .style(Style::default().bg(BG_DARK).fg(FG_DIM));
            f.render_widget(empty, area);
            return;
        }

        let visible_height = area.height as usize;
        self.last_visible_height = visible_height;

        let total_rows = self.total_virtual_rows();

        // Handle scroll_to_bottom_pending
        if self.scroll_to_bottom_pending {
            let max_scroll = total_rows.saturating_sub(visible_height);
            self.scroll_offset = max_scroll;
            self.scroll_to_bottom_pending = false;
        }

        let max_scroll = total_rows.saturating_sub(visible_height);
        let scroll_offset = self.scroll_offset.min(max_scroll);
        self.scroll_offset = scroll_offset;

        let max_width = area.width as usize;

        // Build all virtual rows, then slice the visible window
        let mut all_lines: Vec<Line> = Vec::with_capacity(total_rows);

        for (step_idx, step) in self.steps.iter().enumerate() {
            // Build step header line
            let is_selected = step_idx == self.selected_step;
            let arrow = if step.expanded { "▼" } else { "▶" };

            // Status icon
            let (status_icon, icon_style) = match step.status.as_str() {
                "success" => (
                    "✓",
                    Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
                ),
                "failed" => (
                    "✗",
                    Style::default()
                        .fg(FAILED_TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                "running" => (
                    self.spinner_char(),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                _ => ("○", Style::default().fg(FG_DIM)),
            };

            // Duration string
            let duration_str = if let Some(secs) = step.duration_secs {
                Self::format_duration(secs)
            } else if step.status == "running" {
                "...".to_string()
            } else {
                String::new()
            };

            // Calculate how much space the name can take
            // Format: "  ▶ ✓ StepName                          22s"
            let prefix_len = 6; // "  ▶ ✓ "
            let duration_display_len = if duration_str.is_empty() {
                0
            } else {
                duration_str.len() + 2 // "  " padding before duration
            };
            let name_max_width = max_width
                .saturating_sub(prefix_len)
                .saturating_sub(duration_display_len);
            let display_name: String = if step.name.chars().count() > name_max_width {
                let truncated: String = step
                    .name
                    .chars()
                    .take(name_max_width.saturating_sub(1))
                    .collect();
                format!("{}…", truncated)
            } else {
                step.name.clone()
            };

            // Pad name to fill available space for right-aligned duration
            let name_display_len = display_name.chars().count();
            let padding = name_max_width.saturating_sub(name_display_len);
            let padded_name = format!("{}{}", display_name, " ".repeat(padding));

            let bg_style = if is_selected {
                Style::default().bg(ACCENT).fg(BG_DARK)
            } else {
                Style::default().bg(BG_DARK)
            };

            let mut spans = vec![
                Span::styled("  ", bg_style),
                Span::styled(
                    arrow,
                    if is_selected {
                        bg_style.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(FG_DIM).bg(BG_DARK)
                    },
                ),
                Span::styled(" ", bg_style),
                Span::styled(
                    status_icon,
                    if is_selected {
                        bg_style.add_modifier(Modifier::BOLD)
                    } else {
                        icon_style.bg(BG_DARK)
                    },
                ),
                Span::styled(" ", bg_style),
                Span::styled(
                    padded_name,
                    if is_selected {
                        bg_style.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(FG_BRIGHT).bg(BG_DARK)
                    },
                ),
            ];

            if !duration_str.is_empty() {
                spans.push(Span::styled(
                    format!("  {}", duration_str),
                    if is_selected {
                        bg_style
                    } else {
                        Style::default().fg(FG_DIM).bg(BG_DARK)
                    },
                ));
            }

            all_lines.push(Line::from(spans));

            // If expanded, add log lines or loading indicator
            if step.expanded {
                if step.is_loading {
                    let spinner = self.spinner_char();
                    all_lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(FG_DIM)),
                        Span::styled(
                            format!("{} Loading...", spinner),
                            Style::default().fg(ACCENT),
                        ),
                    ]));
                } else if step.logs.is_empty() {
                    all_lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(FG_DIM)),
                        Span::styled("(no output)", Style::default().fg(FG_DIM)),
                    ]));
                } else {
                    for log_line in &step.logs {
                        let prefix_span = Span::styled("  │ ", Style::default().fg(FG_DIM));
                        let content_max = max_width.saturating_sub(4); // 4 chars for "  │ "
                        let parsed = Self::parse_ansi_line(log_line, content_max);
                        let mut spans = vec![prefix_span];
                        spans.extend(parsed.spans);
                        all_lines.push(Line::from(spans));
                    }
                }
            }
        }

        // Slice visible window
        let visible_lines: Vec<Line> = all_lines
            .into_iter()
            .skip(scroll_offset)
            .take(visible_height)
            .collect();

        let logs = Paragraph::new(visible_lines).style(Style::default().bg(BG_DARK).fg(FG_PRIMARY));

        f.render_widget(logs, area);
    }

    /// Render the sweeping progress bar (used during initial loading)
    fn render_loading_bar(&self, f: &mut Frame, area: Rect) {
        let spinner = self.spinner_char();
        let mut lines = Vec::new();
        let bar_width = (area.width as usize).saturating_sub(6).min(50);

        let sweep_width = 8.min(bar_width);
        let cycle_len = bar_width.saturating_sub(sweep_width) + 1;
        let elapsed_ms = self.created_at.elapsed().as_millis() as usize;
        let cycle_ms = 1500;
        let phase = elapsed_ms % (cycle_ms * 2);
        let t = if phase < cycle_ms {
            phase
        } else {
            cycle_ms * 2 - phase
        };
        let pos = if cycle_len > 0 && cycle_ms > 0 {
            (t * cycle_len.saturating_sub(1)) / cycle_ms
        } else {
            0
        };
        let before = pos;
        let after = bar_width.saturating_sub(pos + sweep_width);

        let message = if let Some((_, _, ref step_name)) = self.load_progress {
            format!("Fetching: {}", step_name)
        } else {
            "Loading logs...".to_string()
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", spinner),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&message, Style::default().fg(FG_PRIMARY)),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" [", Style::default().fg(FG_DIM)),
            Span::styled("─".repeat(before), Style::default().fg(FG_DIM)),
            Span::styled("━".repeat(sweep_width), Style::default().fg(ACCENT)),
            Span::styled("─".repeat(after), Style::default().fg(FG_DIM)),
            Span::styled("]", Style::default().fg(FG_DIM)),
        ]));

        let loading = Paragraph::new(lines).style(Style::default().bg(BG_DARK).fg(FG_PRIMARY));
        f.render_widget(loading, area);
    }

    /// Render the footer with keybindings and step indicator
    fn render_footer(&self, f: &mut Frame, area: Rect, _log_area_height: usize) {
        let step_count = self.steps.len();
        let step_num = if step_count > 0 {
            self.selected_step + 1
        } else {
            0
        };

        let selected_expanded = self
            .steps
            .get(self.selected_step)
            .is_some_and(|s| s.expanded);

        let enter_label = if selected_expanded {
            " Collapse  "
        } else {
            " Expand  "
        };

        let footer_text = vec![Line::from(vec![
            Span::styled(
                "[Esc]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Close  ", Style::default().fg(FG_DIM)),
            Span::styled(
                "[Enter]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(enter_label, Style::default().fg(FG_DIM)),
            Span::styled(
                "[↑↓]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Steps  ", Style::default().fg(FG_DIM)),
            Span::styled(
                "[c]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Copy  ", Style::default().fg(FG_DIM)),
            Span::styled(
                "[r]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" Rerun  ", Style::default().fg(FG_DIM)),
            Span::styled("│  ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!("Step {}/{}", step_num, step_count),
                Style::default().fg(ACCENT),
            ),
        ])];

        let footer = Paragraph::new(footer_text).style(Style::default().bg(BG_PANEL));
        f.render_widget(footer, area);
    }

    // --- Input handling ---

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> ModalAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ModalAction::Close,
            KeyCode::Char('r') => ModalAction::Rerun,
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.toggle_selected_step();
                ModalAction::None
            }
            KeyCode::Char('c') | KeyCode::Char('y') => {
                self.copy_selected_step_logs();
                ModalAction::CopyStepLogs
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selected_step_up();
                self.auto_scroll = false;
                ModalAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selected_step_down();
                ModalAction::None
            }
            KeyCode::PageUp => {
                for _ in 0..10 {
                    self.scroll_up();
                }
                self.auto_scroll = false;
                ModalAction::None
            }
            KeyCode::PageDown => {
                for _ in 0..10 {
                    self.scroll_down();
                }
                ModalAction::None
            }
            KeyCode::Home => {
                self.scroll_offset = 0;
                self.auto_scroll = false;
                ModalAction::None
            }
            KeyCode::End => {
                let total = self.total_virtual_rows();
                let max_scroll = total.saturating_sub(self.last_visible_height);
                self.scroll_offset = max_scroll;
                if self.is_streaming {
                    self.auto_scroll = true;
                }
                ModalAction::None
            }
            _ => ModalAction::None,
        }
    }

    /// Toggle expand/collapse on the selected step
    fn toggle_selected_step(&mut self) {
        if let Some(step) = self.steps.get_mut(self.selected_step) {
            step.expanded = !step.expanded;
            if step.expanded {
                // Adjust scroll_offset to show the step header at the top
                let header_row = self.step_header_row(self.selected_step);
                self.scroll_offset = header_row;
            }
        }
    }

    /// Move selected step up
    fn move_selected_step_up(&mut self) {
        if self.selected_step > 0 {
            self.selected_step -= 1;
            self.ensure_selected_visible();
        }
    }

    /// Move selected step down
    fn move_selected_step_down(&mut self) {
        if !self.steps.is_empty() && self.selected_step < self.steps.len() - 1 {
            self.selected_step += 1;
            self.ensure_selected_visible();
        }
    }

    /// Ensure the selected step header is visible in the viewport
    fn ensure_selected_visible(&mut self) {
        let header_row = self.step_header_row(self.selected_step);
        let visible_height = self.last_visible_height;

        if header_row < self.scroll_offset {
            self.scroll_offset = header_row;
        } else if header_row >= self.scroll_offset + visible_height {
            self.scroll_offset = header_row.saturating_sub(visible_height - 1);
        }
    }

    /// Copy selected step's logs to system clipboard
    fn copy_selected_step_logs(&self) {
        if let Some(step) = self.steps.get(self.selected_step) {
            let text = step.logs.join("\n");
            let _ = Self::copy_to_clipboard(&text);
        }
    }

    /// Copy text to system clipboard
    fn copy_to_clipboard(text: &str) -> Result<(), String> {
        use arboard::Clipboard;

        let mut clipboard =
            Clipboard::new().map_err(|e| format!("Clipboard unavailable: {}", e))?;

        #[cfg(target_os = "linux")]
        {
            use arboard::SetExtLinux;
            use std::time::{Duration, Instant};

            clipboard
                .set()
                .wait_until(Instant::now() + Duration::from_millis(300))
                .text(text.to_owned())
                .map_err(|e| format!("Failed to copy: {}", e))
        }

        #[cfg(not(target_os = "linux"))]
        {
            clipboard
                .set_text(text)
                .map_err(|e| format!("Failed to copy: {}", e))
        }
    }

    /// Scroll up by one line in the virtual view
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down by one line in the virtual view
    pub fn scroll_down(&mut self) {
        let total = self.total_virtual_rows();
        let max_scroll = total.saturating_sub(self.last_visible_height);
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
        assert!(modal.steps.is_empty());
        assert!(modal.is_loading);
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

        // Set up steps
        modal.set_steps(vec![
            ("Step 1".to_string(), "success".to_string()),
            ("Step 2".to_string(), "failed".to_string()),
            ("Step 3".to_string(), "success".to_string()),
        ]);

        // Step 2 should be auto-expanded (failed)
        assert!(!modal.steps[0].expanded);
        assert!(modal.steps[1].expanded);
        assert!(!modal.steps[2].expanded);

        // Set logs for expanded step
        modal.set_step_logs(
            1,
            vec![
                "Error line 1".to_string(),
                "Error line 2".to_string(),
                "Error line 3".to_string(),
            ],
        );

        // A step is expanded, so up/down scrolls
        modal.scroll_offset = 0;
        modal.scroll_down();
        assert!(modal.scroll_offset > 0);

        modal.scroll_up();
        assert_eq!(modal.scroll_offset, 0);

        // Can't scroll above 0
        modal.scroll_up();
        assert_eq!(modal.scroll_offset, 0);
    }

    #[test]
    fn test_set_logs_backward_compat() {
        let job = create_test_job();
        let mut modal = LogModal::new(job);

        // set_logs should create a single "All Logs" step
        modal.set_logs(vec![
            "Line 1".to_string(),
            "Line 2".to_string(),
            "Line 3".to_string(),
        ]);

        assert_eq!(modal.steps.len(), 1);
        assert_eq!(modal.steps[0].name, "All Logs");
        assert!(modal.steps[0].expanded);
        assert_eq!(modal.steps[0].logs.len(), 3);
        assert!(!modal.is_loading);
    }

    #[test]
    fn test_step_expand_collapse() {
        let job = create_test_job();
        let mut modal = LogModal::new(job);

        modal.set_steps(vec![
            ("Step 1".to_string(), "success".to_string()),
            ("Step 2".to_string(), "success".to_string()),
        ]);

        assert!(!modal.steps[0].expanded);
        assert!(!modal.steps[1].expanded);

        // Toggle expand on selected step (step 0)
        modal.toggle_selected_step();
        assert!(modal.steps[0].expanded);

        // Toggle collapse
        modal.toggle_selected_step();
        assert!(!modal.steps[0].expanded);
    }

    #[test]
    fn test_total_virtual_rows() {
        let job = create_test_job();
        let mut modal = LogModal::new(job);

        modal.set_steps(vec![
            ("Step 1".to_string(), "success".to_string()),
            ("Step 2".to_string(), "failed".to_string()),
        ]);

        // Step 2 is auto-expanded (failed), step 1 is collapsed
        // Step 1: 1 header row
        // Step 2: 1 header row + 1 loading row (is_loading: true)
        assert_eq!(modal.total_virtual_rows(), 3);

        // Set logs for step 2
        modal.set_step_logs(1, vec!["line1".to_string(), "line2".to_string()]);
        // Step 1: 1 header
        // Step 2: 1 header + 2 log lines
        assert_eq!(modal.total_virtual_rows(), 4);
    }

    #[test]
    fn test_centered_rect() {
        let parent = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(80, 80, parent);

        assert_eq!(centered.width, 80);
        assert_eq!(centered.height, 80);
        assert_eq!(centered.x, 10);
        assert_eq!(centered.y, 10);
    }
}
