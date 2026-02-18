/// Pipeline screen implementation
use crate::api::models::{mock_data, Pipeline, Workflow};
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_INPUT, BG_PANEL, BORDER, BORDER_FOCUSED,
    FG_BRIGHT, FG_DIM, FG_PRIMARY,
};
use crate::ui::widgets::breadcrumb::render_breadcrumb;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Pipeline screen with two-pane layout
pub struct PipelineScreen {
    /// List of pipelines (from mock data)
    pub pipelines: Vec<Pipeline>,
    /// List selection state
    pub list_state: ListState,
    /// Currently selected index
    pub selected_index: Option<usize>,
    /// Loading state
    pub loading: bool,
    /// Search query for future search functionality
    pub search_query: String,
}

impl PipelineScreen {
    /// Create a new pipeline screen with mock data
    pub fn new() -> Self {
        let pipelines = mock_data::mock_pipelines();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            pipelines,
            list_state,
            selected_index: Some(0),
            loading: false,
            search_query: String::new(),
        }
    }

    /// Render the pipeline screen
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Main layout: Header | Breadcrumb | Filter Panel | Body | Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Length(1), // Breadcrumb
                Constraint::Length(3), // Filter Panel
                Constraint::Min(0),    // Body
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header
        self.render_header(f, main_chunks[0]);

        // Render breadcrumb
        self.render_breadcrumb(f, main_chunks[1]);

        // Render filter panel placeholder
        self.render_filter_panel(f, main_chunks[2]);

        // Split body into left (pipelines list) and right (details) panels
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[3]);

        // Render left panel (search + pipeline list)
        self.render_left_panel(f, body_chunks[0]);

        // Render right panel (pipeline details)
        self.render_right_panel(f, body_chunks[1]);

        // Render footer
        self.render_footer(f, main_chunks[4]);
    }

    /// Render the header
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let project_slug = self
            .pipelines
            .first()
            .map(|p| p.project_slug.as_str())
            .unwrap_or("gh/acme/api-service");

        let header = Paragraph::new(Line::from(vec![
            Span::styled("cci", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" › ", Style::default().fg(FG_DIM)),
            Span::styled(project_slug, Style::default().fg(FG_PRIMARY)),
        ]));

        f.render_widget(header, area);
    }

    /// Render the breadcrumb
    fn render_breadcrumb(&self, f: &mut Frame, area: Rect) {
        let breadcrumb = render_breadcrumb(&["Pipelines"]);
        f.render_widget(breadcrumb, area);
    }

    /// Render filter panel placeholder
    fn render_filter_panel(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Filters ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_PANEL));

        let content = Paragraph::new("Filter panel placeholder")
            .block(block)
            .style(Style::default().fg(FG_DIM));

        f.render_widget(content, area);
    }

    /// Render left panel (search + pipeline list)
    fn render_left_panel(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Render search input
        self.render_search_input(f, chunks[0]);

        // Render pipeline list
        self.render_pipeline_list(f, chunks[1]);
    }

    /// Render search input
    fn render_search_input(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_INPUT));

        let placeholder = if self.search_query.is_empty() {
            "Filter pipelines..."
        } else {
            &self.search_query
        };

        let content = Paragraph::new(placeholder)
            .block(block)
            .style(Style::default().fg(FG_DIM));

        f.render_widget(content, area);
    }

    /// Render pipeline list
    fn render_pipeline_list(&mut self, f: &mut Frame, area: Rect) {
        let selected_idx = self.selected_index;
        let items: Vec<ListItem> = self
            .pipelines
            .iter()
            .enumerate()
            .map(|(idx, pipeline)| {
                let is_selected = selected_idx == Some(idx);
                let line = render_pipeline_line(pipeline, is_selected);
                ListItem::new(line)
            })
            .collect();

        let block = Block::default()
            .title(" Pipelines ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.selected_index.is_some() {
                BORDER_FOCUSED
            } else {
                BORDER
            }))
            .style(Style::default().bg(BG_PANEL));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        f.render_stateful_widget(list, area, &mut self.list_state);
    }


    /// Render right panel (pipeline details)
    fn render_right_panel(&self, f: &mut Frame, area: Rect) {
        if let Some(pipeline) = self.get_selected_pipeline() {
            // Split into details section and workflows section
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(12), Constraint::Min(0)])
                .split(area);

            self.render_pipeline_details(f, chunks[0], pipeline);
            self.render_workflows(f, chunks[1], pipeline);
        } else {
            // No selection
            let block = Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER))
                .style(Style::default().bg(BG_PANEL));

            let content =
                Paragraph::new("No pipeline selected").style(Style::default().fg(FG_DIM));

            f.render_widget(content.block(block), area);
        }
    }

    /// Render pipeline details
    fn render_pipeline_details(&self, f: &mut Frame, area: Rect, pipeline: &Pipeline) {
        let icon = get_status_icon(&pipeline.state);
        let status_col = get_status_color(&pipeline.state);

        let details = vec![
            Line::from(vec![
                Span::styled(icon, Style::default().fg(status_col)),
                Span::raw(" "),
                Span::styled(
                    &pipeline.state,
                    Style::default()
                        .fg(status_col)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Branch: ", Style::default().fg(FG_DIM)),
                Span::styled(&pipeline.vcs.branch, Style::default().fg(FG_BRIGHT)),
            ]),
            Line::from(vec![
                Span::styled("SHA: ", Style::default().fg(FG_DIM)),
                Span::styled(&pipeline.vcs.revision[..7.min(pipeline.vcs.revision.len())], Style::default().fg(FG_BRIGHT)),
            ]),
            Line::from(vec![
                Span::styled("Author: ", Style::default().fg(FG_DIM)),
                Span::styled(
                    &pipeline.vcs.commit_author_name,
                    Style::default().fg(FG_BRIGHT),
                ),
            ]),
            Line::from(vec![
                Span::styled("Trigger: ", Style::default().fg(FG_DIM)),
                Span::styled(
                    &pipeline.trigger.trigger_type,
                    Style::default().fg(FG_BRIGHT),
                ),
            ]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(FG_DIM)),
                Span::styled(
                    format_timestamp(&pipeline.created_at),
                    Style::default().fg(FG_BRIGHT),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("[", Style::default().fg(FG_DIM)),
                Span::styled("Open", Style::default().fg(ACCENT)),
                Span::styled(" ⏎", Style::default().fg(FG_DIM)),
                Span::styled("] [", Style::default().fg(FG_DIM)),
                Span::styled("Rerun", Style::default().fg(FG_DIM)),
                Span::styled("] [", Style::default().fg(FG_DIM)),
                Span::styled("SSH", Style::default().fg(FG_DIM)),
                Span::styled("]", Style::default().fg(FG_DIM)),
            ]),
        ];

        let block = Block::default()
            .title(" Pipeline Details ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_PANEL));

        let paragraph = Paragraph::new(details).block(block);
        f.render_widget(paragraph, area);
    }

    /// Render workflows
    fn render_workflows(&self, f: &mut Frame, area: Rect, pipeline: &Pipeline) {
        let workflows = mock_data::mock_workflows(&pipeline.id);

        let items: Vec<ListItem> = workflows
            .iter()
            .map(|workflow| {
                let line = render_workflow_line(workflow);
                ListItem::new(line)
            })
            .collect();

        let block = Block::default()
            .title(" Workflows ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_PANEL));

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }

    /// Render footer
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("q", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Quit | ", Style::default().fg(FG_PRIMARY)),
            Span::styled("r", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Refresh | ", Style::default().fg(FG_PRIMARY)),
            Span::styled("↑↓", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate | ", Style::default().fg(FG_PRIMARY)),
            Span::styled("⏎", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Open", Style::default().fg(FG_PRIMARY)),
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
            KeyCode::Enter => true, // Signal to open the selected pipeline
            KeyCode::Char('r') => {
                // Refresh: reload mock data
                self.pipelines = mock_data::mock_pipelines();
                false
            }
            _ => false,
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.pipelines.is_empty() {
            return;
        }

        let i = match self.selected_index {
            Some(i) => {
                if i >= self.pipelines.len() - 1 {
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
        if self.pipelines.is_empty() {
            return;
        }

        let i = match self.selected_index {
            Some(i) => {
                if i == 0 {
                    self.pipelines.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.selected_index = Some(i);
        self.list_state.select(Some(i));
    }

    /// Get the currently selected pipeline
    pub fn get_selected_pipeline(&self) -> Option<&Pipeline> {
        self.selected_index
            .and_then(|i| self.pipelines.get(i))
    }
}

impl Default for PipelineScreen {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a single pipeline line
fn render_pipeline_line(pipeline: &Pipeline, selected: bool) -> Line {
    let icon = get_status_icon(&pipeline.state);
    let status_col = get_status_color(&pipeline.state);

    // Format: [icon] #number [commit:40] [branch] [sha:7]
    let commit_msg = truncate_string(&pipeline.vcs.commit_subject, 40);
    let short_sha = if pipeline.vcs.revision.len() > 7 {
        &pipeline.vcs.revision[..7]
    } else {
        &pipeline.vcs.revision
    };

    if selected {
        Line::from(vec![
            Span::styled(
                format!("{} ", icon),
                Style::default()
                    .fg(status_col)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("#{:<4} ", pipeline.number),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<40} ", commit_msg),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}] ", pipeline.vcs.branch),
                Style::default().fg(ACCENT),
            ),
            Span::styled(short_sha, Style::default().fg(FG_DIM)),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
            Span::styled(
                format!("#{:<4} ", pipeline.number),
                Style::default().fg(FG_DIM),
            ),
            Span::styled(
                format!("{:<40} ", commit_msg),
                Style::default().fg(FG_PRIMARY),
            ),
            Span::styled(
                format!("[{}] ", pipeline.vcs.branch),
                Style::default().fg(FG_DIM),
            ),
            Span::styled(short_sha, Style::default().fg(FG_DIM)),
        ])
    }
}

/// Render a workflow line
fn render_workflow_line(workflow: &Workflow) -> Line {
    let icon = get_status_icon(&workflow.status);
    let status_col = get_status_color(&workflow.status);

    Line::from(vec![
        Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
        Span::styled(&workflow.name, Style::default().fg(FG_PRIMARY)),
        Span::raw(" "),
        Span::styled(
            format!("({})", workflow.duration_formatted()),
            Style::default().fg(FG_DIM),
        ),
    ])
}

/// Truncate a string to a maximum length, adding "..." if truncated
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Format a timestamp for display
fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    use chrono::Local;
    let local: chrono::DateTime<Local> = (*timestamp).into();
    local.format("%Y-%m-%d %H:%M:%S").to_string()
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
