/// Screen 1: Shows list of pipelines for the project
///
/// This is the first screen in the hierarchy: Pipeline → Workflow → Job
/// Users navigate from here to the workflows list screen by pressing Enter.
use crate::api::models::{mock_data, Pipeline};
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_PANEL, BORDER, BORDER_FOCUSED,
    FG_BRIGHT, FG_DIM, FG_PRIMARY,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Pipeline screen with dense list and filters
pub struct PipelineScreen {
    /// All pipelines (from mock data)
    pub pipelines: Vec<Pipeline>,
    /// Filtered pipelines (after applying filters)
    pub filtered_pipelines: Vec<Pipeline>,
    /// List selection state
    pub list_state: ListState,
    /// Currently selected index (in filtered list)
    pub selected_index: Option<usize>,
    /// Loading state
    pub loading: bool,
    /// Text filter input
    pub filter_text: String,
    /// Branch filter (None = All)
    pub branch_filter: Option<String>,
    /// Status filter (None = All)
    pub status_filter: Option<String>,
    /// Active input focus (0 = none, 1 = text filter, 2 = branch, 3 = status)
    pub filter_focus: u8,
}

impl PipelineScreen {
    /// Create a new pipeline screen with mock data
    pub fn new() -> Self {
        let pipelines = mock_data::mock_pipelines();
        let filtered_pipelines = pipelines.clone();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            pipelines,
            filtered_pipelines,
            list_state,
            selected_index: Some(0),
            loading: false,
            filter_text: String::new(),
            branch_filter: None,
            status_filter: None,
            filter_focus: 0,
        }
    }

    /// Apply filters to the pipeline list
    pub fn apply_filters(&mut self) {
        self.filtered_pipelines = self.pipelines.iter()
            .filter(|p| {
                // Text filter (searches in commit message, branch, author)
                let text_match = if self.filter_text.is_empty() {
                    true
                } else {
                    let query = self.filter_text.to_lowercase();
                    p.vcs.commit_subject.to_lowercase().contains(&query)
                        || p.vcs.branch.to_lowercase().contains(&query)
                        || p.vcs.commit_author_name.to_lowercase().contains(&query)
                };

                // Branch filter
                let branch_match = if let Some(ref branch) = self.branch_filter {
                    &p.vcs.branch == branch
                } else {
                    true
                };

                // Status filter
                let status_match = if let Some(ref status) = self.status_filter {
                    &p.state == status
                } else {
                    true
                };

                text_match && branch_match && status_match
            })
            .cloned()
            .collect();

        // Reset selection if needed
        if self.filtered_pipelines.is_empty() {
            self.selected_index = None;
            self.list_state.select(None);
        } else if self.selected_index.is_some() {
            let idx = self.selected_index.unwrap().min(self.filtered_pipelines.len() - 1);
            self.selected_index = Some(idx);
            self.list_state.select(Some(idx));
        }
    }

    /// Render the pipeline screen
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Main layout: Header | Filter Bar | List | Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with title
                Constraint::Length(3), // Filter bar
                Constraint::Min(0),    // Pipeline list (full width)
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header with title
        self.render_header(f, main_chunks[0]);

        // Render filter bar
        self.render_filter_bar(f, main_chunks[1]);

        // Render pipeline list (full width, multi-line items)
        self.render_pipeline_list(f, main_chunks[2]);

        // Render footer with actions
        self.render_footer(f, main_chunks[3]);
    }

    /// Render the header with title
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let project_slug = self
            .pipelines
            .first()
            .map(|p| p.project_slug.as_str())
            .unwrap_or("gh/acme/api-service");

        let block = Block::default()
            .title(format!(" CircleCI Pipelines - {} ", project_slug))
            .title_style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_PANEL));

        f.render_widget(block, area);
    }

    /// Render filter bar with text input, branch dropdown, status dropdown
    fn render_filter_bar(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_PANEL));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Split into three columns: Filter text | Branch | Status
        let filter_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Text filter
                Constraint::Percentage(25), // Branch filter
                Constraint::Percentage(25), // Status filter
            ])
            .split(inner);

        // Text filter
        let text_filter = format!(
            "Filter: {}{}",
            self.filter_text,
            if self.filter_text.is_empty() { "_" } else { "" }
        );
        let text_style = if self.filter_focus == 1 {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG_PRIMARY)
        };
        let text_widget = Paragraph::new(text_filter).style(text_style);
        f.render_widget(text_widget, filter_chunks[0]);

        // Branch filter
        let branch_text = format!(
            "  Branch: [{}▼]",
            self.branch_filter.as_deref().unwrap_or("All")
        );
        let branch_style = if self.filter_focus == 2 {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG_PRIMARY)
        };
        let branch_widget = Paragraph::new(branch_text).style(branch_style);
        f.render_widget(branch_widget, filter_chunks[1]);

        // Status filter
        let status_text = format!(
            "  Status: [{}▼]",
            self.status_filter.as_deref().unwrap_or("All")
        );
        let status_style = if self.filter_focus == 3 {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG_PRIMARY)
        };
        let status_widget = Paragraph::new(status_text).style(status_style);
        f.render_widget(status_widget, filter_chunks[2]);
    }

    /// Render pipeline list with multi-line items (glim-style dense layout)
    fn render_pipeline_list(&mut self, f: &mut Frame, area: Rect) {
        let selected_idx = self.selected_index;

        // Create multi-line list items (3 lines per pipeline)
        let items: Vec<ListItem> = self
            .filtered_pipelines
            .iter()
            .enumerate()
            .map(|(idx, pipeline)| {
                let is_selected = selected_idx == Some(idx);
                render_pipeline_multiline(pipeline, is_selected)
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .style(Style::default().bg(BG_PANEL));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default());

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render footer with actions
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("[↑↓]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[⏎]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Open  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[r]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Refresh  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[/]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Filter  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[q]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Quit", Style::default().fg(FG_PRIMARY)),
        ]))
        .alignment(Alignment::Center);

        f.render_widget(footer, area);
    }

    /// Handle keyboard input
    ///
    /// Returns true if the user wants to open the selected pipeline
    pub fn handle_input(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => {
                self.select_previous();
                false
            }
            KeyCode::Down => {
                self.select_next();
                false
            }
            KeyCode::Enter => {
                // Only open if we have a selection
                self.selected_index.is_some()
            }
            KeyCode::Char('r') => {
                // Refresh: reload mock data and reapply filters
                self.pipelines = mock_data::mock_pipelines();
                self.apply_filters();
                false
            }
            KeyCode::Char('/') => {
                // Activate filter text input
                self.filter_focus = 1;
                false
            }
            KeyCode::Char(c) if self.filter_focus == 1 => {
                // Add character to filter text
                self.filter_text.push(c);
                self.apply_filters();
                false
            }
            KeyCode::Backspace if self.filter_focus == 1 => {
                // Remove character from filter text
                self.filter_text.pop();
                self.apply_filters();
                false
            }
            KeyCode::Esc if self.filter_focus > 0 => {
                // Exit filter mode
                self.filter_focus = 0;
                false
            }
            _ => false,
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.filtered_pipelines.is_empty() {
            return;
        }

        let i = match self.selected_index {
            Some(i) => {
                if i >= self.filtered_pipelines.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.selected_index = Some(i);
        self.list_state.select(Some(i));
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.filtered_pipelines.is_empty() {
            return;
        }

        let i = match self.selected_index {
            Some(i) => {
                if i == 0 {
                    self.filtered_pipelines.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_index = Some(i);
        self.list_state.select(Some(i));
    }

    /// Get the currently selected pipeline (from filtered list)
    pub fn get_selected_pipeline(&self) -> Option<&Pipeline> {
        self.selected_index
            .and_then(|i| self.filtered_pipelines.get(i))
    }
}

impl Default for PipelineScreen {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a pipeline as multi-line item (3 lines: status/time/branch, commit msg, summary)
fn render_pipeline_multiline(pipeline: &Pipeline, selected: bool) -> ListItem<'_> {
    let icon = get_status_icon(&pipeline.state);
    let status_col = get_status_color(&pipeline.state);

    // Calculate time ago
    let time_ago = format_time_ago(&pipeline.created_at);
    let time_str = pipeline.created_at.format("%H:%M").to_string();

    // Calculate duration (from created to updated)
    let duration = format_duration(pipeline.created_at, pipeline.updated_at);

    // Line 1: ● [time] Pipeline #[num] [duration] ● [branch]
    let line1 = if selected {
        Line::from(vec![
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(status_col).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<5} ", time_str),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("Pipeline #{:<6} ", pipeline.number),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<10} ", time_ago),
                Style::default().fg(FG_DIM),
            ),
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(status_col),
            ),
            Span::styled(
                format!(" {}", pipeline.vcs.branch),
                Style::default().fg(ACCENT),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(status_col),
            ),
            Span::styled(
                format!("{:<5} ", time_str),
                Style::default().fg(FG_DIM),
            ),
            Span::styled(
                format!("Pipeline #{:<6} ", pipeline.number),
                Style::default().fg(FG_PRIMARY),
            ),
            Span::styled(
                format!("{:<10} ", time_ago),
                Style::default().fg(FG_DIM),
            ),
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(status_col),
            ),
            Span::styled(
                format!(" {}", pipeline.vcs.branch),
                Style::default().fg(FG_DIM),
            ),
        ])
    };

    // Line 2: Indented commit message
    let line2 = if selected {
        Line::from(vec![
            Span::styled(
                format!("          {}", pipeline.vcs.commit_subject),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!("          {}", pipeline.vcs.commit_subject),
                Style::default().fg(FG_PRIMARY),
            ),
        ])
    };

    // Line 3: Indented summary (mock: 3 workflows • 24 jobs • 2 failed)
    let summary = format!(
        "          3 workflows • 24 jobs • {} ({})",
        pipeline.state,
        duration
    );
    let line3 = Line::from(vec![
        Span::styled(
            summary,
            Style::default().fg(FG_DIM),
        ),
    ]);

    // Combine all lines into a ListItem
    ListItem::new(vec![line1, line2, line3])
}

/// Format time ago (e.g., "2h ago", "45m ago")
fn format_time_ago(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    use chrono::Utc;
    let now = Utc::now();
    let duration = now.signed_duration_since(*timestamp);

    let secs = duration.num_seconds();
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

/// Format duration between two timestamps
fn format_duration(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> String {
    let duration = end.signed_duration_since(start);
    let secs = duration.num_seconds();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_screen_new() {
        let screen = PipelineScreen::new();
        assert!(!screen.pipelines.is_empty());
        assert_eq!(screen.selected_index, Some(0));
    }

    #[test]
    fn test_select_next() {
        let mut screen = PipelineScreen::new();
        let initial_count = screen.pipelines.len();
        screen.select_next();
        assert_eq!(screen.selected_index, Some(1));

        // Test wrap around
        screen.selected_index = Some(initial_count - 1);
        screen.select_next();
        assert_eq!(screen.selected_index, Some(0));
    }

    #[test]
    fn test_select_previous() {
        let mut screen = PipelineScreen::new();
        let initial_count = screen.pipelines.len();

        // Test wrap around at beginning
        screen.selected_index = Some(0);
        screen.select_previous();
        assert_eq!(screen.selected_index, Some(initial_count - 1));

        // Test normal previous
        screen.select_previous();
        assert_eq!(screen.selected_index, Some(initial_count - 2));
    }

    #[test]
    fn test_get_selected_pipeline() {
        let screen = PipelineScreen::new();
        let pipeline = screen.get_selected_pipeline();
        assert!(pipeline.is_some());
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long string", 10), "this is...");
    }
}

/// Truncate a string to a maximum length and add "..." if truncated
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
