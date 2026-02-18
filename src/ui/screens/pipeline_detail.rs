/// Screen 2: Pipeline Detail with Workflow Tree + Jobs List
///
/// This screen shows:
/// - Left Panel (30%): Workflow tree with arrows indicating selection
/// - Right Panel (70%): Filtered jobs list for the selected workflow
use crate::api::models::{Job, Pipeline, Workflow};
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

/// Focus state for the pipeline detail screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    /// Left panel (workflows tree)
    Workflows,
    /// Right panel (jobs list)
    Jobs,
}

/// Action returned from input handling
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineDetailAction {
    /// No action
    None,
    /// Open job log modal for the specified job
    OpenJobLog(Job),
    /// Go back to pipelines screen
    Back,
}

/// Pipeline detail screen with workflow tree and jobs list
pub struct PipelineDetailScreen {
    /// Parent pipeline data
    pub pipeline: Pipeline,
    /// List of workflows in this pipeline
    pub workflows: Vec<Workflow>,
    /// Jobs for the currently selected workflow
    pub jobs: Vec<Job>,
    /// Currently selected workflow index
    pub selected_workflow_index: usize,
    /// Currently selected job index (within the filtered list)
    pub selected_job_index: Option<usize>,
    /// Current panel focus
    pub focus: PanelFocus,
    /// Job filter text
    pub job_filter: String,
    /// Show only failed jobs
    pub show_only_failed: bool,
    /// List state for workflow selection
    pub workflow_list_state: ListState,
    /// List state for job selection
    pub job_list_state: ListState,
}

impl PipelineDetailScreen {
    /// Create a new pipeline detail screen
    pub fn new(pipeline: Pipeline) -> Self {
        let workflows = crate::api::models::mock_data::mock_workflows(&pipeline.id);
        let selected_workflow_index = 0;

        // Load jobs for the first workflow
        let jobs = if !workflows.is_empty() {
            crate::api::models::mock_data::mock_jobs(&workflows[0].id)
        } else {
            Vec::new()
        };

        let selected_job_index = if jobs.is_empty() { None } else { Some(0) };

        let mut workflow_list_state = ListState::default();
        workflow_list_state.select(Some(0));

        let mut job_list_state = ListState::default();
        if !jobs.is_empty() {
            job_list_state.select(Some(0));
        }

        Self {
            pipeline,
            workflows,
            jobs,
            selected_workflow_index,
            selected_job_index,
            focus: PanelFocus::Workflows,
            job_filter: String::new(),
            show_only_failed: false,
            workflow_list_state,
            job_list_state,
        }
    }

    /// Select a workflow and reload its jobs
    pub fn select_workflow(&mut self, index: usize) {
        if index >= self.workflows.len() {
            return;
        }

        self.selected_workflow_index = index;
        self.workflow_list_state.select(Some(index));

        // Load jobs for the selected workflow
        self.jobs = crate::api::models::mock_data::mock_jobs(&self.workflows[index].id);

        // Reset job selection
        if !self.jobs.is_empty() {
            self.selected_job_index = Some(0);
            self.job_list_state.select(Some(0));
        } else {
            self.selected_job_index = None;
            self.job_list_state.select(None);
        }
    }

    /// Get the filtered list of jobs
    fn get_filtered_jobs(&self) -> Vec<&Job> {
        self.jobs
            .iter()
            .filter(|job| {
                // Apply text filter
                let matches_filter = if self.job_filter.is_empty() {
                    true
                } else {
                    job.name
                        .to_lowercase()
                        .contains(&self.job_filter.to_lowercase())
                };

                // Apply status filter
                let matches_status = if self.show_only_failed {
                    job.status == "failed"
                } else {
                    true
                };

                matches_filter && matches_status
            })
            .collect()
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> PipelineDetailAction {
        match key.code {
            KeyCode::Tab => {
                // Switch focus between panels
                self.focus = match self.focus {
                    PanelFocus::Workflows => PanelFocus::Jobs,
                    PanelFocus::Jobs => PanelFocus::Workflows,
                };
                PipelineDetailAction::None
            }
            KeyCode::Up => {
                match self.focus {
                    PanelFocus::Workflows => self.select_previous_workflow(),
                    PanelFocus::Jobs => self.select_previous_job(),
                }
                PipelineDetailAction::None
            }
            KeyCode::Down => {
                match self.focus {
                    PanelFocus::Workflows => self.select_next_workflow(),
                    PanelFocus::Jobs => self.select_next_job(),
                }
                PipelineDetailAction::None
            }
            KeyCode::Enter => {
                // Open job log for selected job
                if self.focus == PanelFocus::Jobs {
                    if let Some(idx) = self.selected_job_index {
                        let filtered_jobs = self.get_filtered_jobs();
                        if let Some(job) = filtered_jobs.get(idx) {
                            return PipelineDetailAction::OpenJobLog((*job).clone());
                        }
                    }
                }
                PipelineDetailAction::None
            }
            KeyCode::Char('f') => {
                // Toggle failed filter
                self.show_only_failed = !self.show_only_failed;
                // Reset job selection
                let filtered_jobs = self.get_filtered_jobs();
                if !filtered_jobs.is_empty() {
                    self.selected_job_index = Some(0);
                    self.job_list_state.select(Some(0));
                } else {
                    self.selected_job_index = None;
                    self.job_list_state.select(None);
                }
                PipelineDetailAction::None
            }
            KeyCode::Esc => PipelineDetailAction::Back,
            _ => PipelineDetailAction::None,
        }
    }

    /// Move workflow selection down
    fn select_next_workflow(&mut self) {
        if self.workflows.is_empty() {
            return;
        }

        let next = if self.selected_workflow_index >= self.workflows.len() - 1 {
            0
        } else {
            self.selected_workflow_index + 1
        };

        self.select_workflow(next);
    }

    /// Move workflow selection up
    fn select_previous_workflow(&mut self) {
        if self.workflows.is_empty() {
            return;
        }

        let prev = if self.selected_workflow_index == 0 {
            self.workflows.len() - 1
        } else {
            self.selected_workflow_index - 1
        };

        self.select_workflow(prev);
    }

    /// Move job selection down
    fn select_next_job(&mut self) {
        let filtered_jobs = self.get_filtered_jobs();
        if filtered_jobs.is_empty() {
            return;
        }

        let next = match self.selected_job_index {
            Some(idx) => {
                if idx >= filtered_jobs.len() - 1 {
                    0
                } else {
                    idx + 1
                }
            }
            None => 0,
        };

        self.selected_job_index = Some(next);
        self.job_list_state.select(Some(next));
    }

    /// Move job selection up
    fn select_previous_job(&mut self) {
        let filtered_jobs = self.get_filtered_jobs();
        if filtered_jobs.is_empty() {
            return;
        }

        let prev = match self.selected_job_index {
            Some(idx) => {
                if idx == 0 {
                    filtered_jobs.len() - 1
                } else {
                    idx - 1
                }
            }
            None => 0,
        };

        self.selected_job_index = Some(prev);
        self.job_list_state.select(Some(prev));
    }

    /// Render the pipeline detail screen
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Main layout: Header | Breadcrumb | Body | Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header (2 lines for pipeline info)
                Constraint::Length(1), // Breadcrumb
                Constraint::Min(0),    // Body
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header
        self.render_header(f, main_chunks[0]);

        // Render breadcrumb
        self.render_breadcrumb(f, main_chunks[1]);

        // Split body into left (workflows) and right (jobs) panels
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_chunks[2]);

        // Render left panel (workflow tree)
        self.render_workflows_panel(f, body_chunks[0]);

        // Render right panel (jobs list)
        self.render_jobs_panel(f, body_chunks[1]);

        // Render footer
        self.render_footer(f, main_chunks[3]);
    }

    /// Render the header with pipeline information
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let project = self
            .pipeline
            .project_slug
            .split('/')
            .last()
            .unwrap_or("project");

        let short_sha = if self.pipeline.vcs.revision.len() > 7 {
            &self.pipeline.vcs.revision[..7]
        } else {
            &self.pipeline.vcs.revision
        };

        // Line 1: project › pipeline #number › branch › sha
        let line1 = Line::from(vec![
            Span::styled(project, Style::default().fg(FG_PRIMARY)),
            Span::styled(" › ", Style::default().fg(FG_DIM)),
            Span::styled(
                format!("pipeline #{}", self.pipeline.number),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" › ", Style::default().fg(FG_DIM)),
            Span::styled(
                &self.pipeline.vcs.branch,
                Style::default().fg(FG_BRIGHT),
            ),
            Span::styled(" › ", Style::default().fg(FG_DIM)),
            Span::styled(short_sha, Style::default().fg(FG_DIM)),
        ]);

        // Line 2: commit message • author • time ago
        let line2 = Line::from(vec![
            Span::styled(
                truncate_string(&self.pipeline.vcs.commit_subject, 60),
                Style::default().fg(FG_PRIMARY),
            ),
            Span::styled(" • ", Style::default().fg(FG_DIM)),
            Span::styled(
                &self.pipeline.vcs.commit_author_name,
                Style::default().fg(FG_DIM),
            ),
            Span::styled(" • ", Style::default().fg(FG_DIM)),
            Span::styled("2h ago", Style::default().fg(FG_DIM)), // TODO: calculate from timestamp
        ]);

        let header = Paragraph::new(vec![line1, line2]);
        f.render_widget(header, area);
    }

    /// Render the breadcrumb
    fn render_breadcrumb(&self, f: &mut Frame, area: Rect) {
        let project = self
            .pipeline
            .project_slug
            .split('/')
            .last()
            .unwrap_or("project");
        let pipeline_num = format!("pipeline #{}", self.pipeline.number);

        let breadcrumb = render_breadcrumb(&[project, &pipeline_num]);
        f.render_widget(breadcrumb, area);
    }

    /// Render the workflows panel (left side)
    fn render_workflows_panel(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .workflows
            .iter()
            .enumerate()
            .map(|(idx, workflow)| {
                let icon = get_status_icon(&workflow.status);
                let status_col = get_status_color(&workflow.status);
                let is_selected = idx == self.selected_workflow_index;

                let arrow = if is_selected { "▶ " } else { "  " };
                let duration = workflow.duration_formatted();

                let line = if is_selected {
                    Line::from(vec![
                        Span::styled(arrow, Style::default().fg(ACCENT)),
                        Span::styled(
                            format!("{} ", icon),
                            Style::default()
                                .fg(status_col)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:<20} ", truncate_string(&workflow.name, 18)),
                            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(duration, Style::default().fg(FG_DIM)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(arrow, Style::default().fg(FG_DIM)),
                        Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
                        Span::styled(
                            format!("{:<20} ", truncate_string(&workflow.name, 18)),
                            Style::default().fg(FG_PRIMARY),
                        ),
                        Span::styled(duration, Style::default().fg(FG_DIM)),
                    ])
                };

                ListItem::new(line)
            })
            .collect();

        let block = Block::default()
            .title(" WORKFLOWS ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.focus == PanelFocus::Workflows {
                BORDER_FOCUSED
            } else {
                BORDER
            }))
            .style(Style::default().bg(BG_PANEL));

        let list = List::new(items).block(block);

        f.render_stateful_widget(list, area, &mut self.workflow_list_state);
    }

    /// Render the jobs panel (right side)
    fn render_jobs_panel(&mut self, f: &mut Frame, area: Rect) {
        // Split into filter bar and job list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Filter bar
                Constraint::Min(0),    // Job list
            ])
            .split(area);

        // Render filter bar
        self.render_filter_bar(f, chunks[0]);

        // Render job list
        self.render_job_list(f, chunks[1]);
    }

    /// Render the filter bar
    fn render_filter_bar(&self, f: &mut Frame, area: Rect) {
        let filter_display = if self.job_filter.is_empty() {
            "_____".to_string()
        } else {
            self.job_filter.clone()
        };

        let failed_checkbox = if self.show_only_failed { "☑" } else { "☐" };

        let line = Line::from(vec![
            Span::styled("Filter: [", Style::default().fg(FG_PRIMARY)),
            Span::styled(&filter_display, Style::default().fg(FG_DIM)),
            Span::styled("]  ", Style::default().fg(FG_PRIMARY)),
            Span::styled(
                format!("{}Failed ", failed_checkbox),
                Style::default().fg(FG_PRIMARY),
            ),
            Span::styled("☐All", Style::default().fg(FG_DIM)),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_INPUT));

        let paragraph = Paragraph::new(line).block(block);
        f.render_widget(paragraph, area);
    }

    /// Render the job list
    fn render_job_list(&mut self, f: &mut Frame, area: Rect) {
        let filtered_jobs = self.get_filtered_jobs();

        // Build multi-line list items in glim style
        let mut items: Vec<ListItem> = Vec::new();

        for job in filtered_jobs.iter() {
            let _icon = get_status_icon(&job.status);
            let status_col = get_status_color(&job.status);

            // Line 1: ● [time] [job_name] [duration] ●
            let time = if let Some(started) = job.started_at {
                format!("{}", started.format("%H:%M"))
            } else {
                "pending".to_string()
            };

            let duration = job.duration_formatted();

            let line1 = Line::from(vec![
                Span::styled("● ", Style::default().fg(status_col)),
                Span::styled(format!("{}  ", time), Style::default().fg(FG_DIM)),
                Span::styled(
                    format!("{:<20} ", truncate_string(&job.name, 18)),
                    Style::default().fg(FG_PRIMARY),
                ),
                Span::styled(format!("{:<8} ", duration), Style::default().fg(FG_DIM)),
                Span::styled("●", Style::default().fg(status_col)),
            ]);

            // Line 2: indented status message or step info
            let status_message = match job.status.as_str() {
                "success" => "Completed successfully",
                "failed" => "Connection timeout",
                "running" => "In progress...",
                "blocked" => "Waiting for dependencies",
                "pending" => "Queued",
                _ => &job.status,
            };

            let line2 = Line::from(vec![Span::styled(
                format!("     {}", status_message),
                Style::default().fg(FG_DIM),
            )]);

            items.push(ListItem::new(vec![line1, line2]));
        }

        let selected_workflow = &self.workflows[self.selected_workflow_index];
        let title = format!(" JOBS › From: {} ", selected_workflow.name);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(if self.focus == PanelFocus::Jobs {
                BORDER_FOCUSED
            } else {
                BORDER
            }))
            .style(Style::default().bg(BG_PANEL));

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(list, area, &mut self.job_list_state);
    }

    /// Render footer with keyboard shortcuts
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("[↑↓]", Style::default().fg(ACCENT)),
            Span::styled(" Nav  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[Tab]", Style::default().fg(ACCENT)),
            Span::styled(" Switch  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[⏎]", Style::default().fg(ACCENT)),
            Span::styled(" View Logs  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[f]", Style::default().fg(ACCENT)),
            Span::styled(" Filter Failed  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[Esc]", Style::default().fg(ACCENT)),
            Span::styled(" Back", Style::default().fg(FG_PRIMARY)),
        ]))
        .alignment(Alignment::Center);

        f.render_widget(footer, area);
    }
}

/// Truncate a string to a maximum length, adding "..." if truncated
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{TriggerInfo, VcsInfo};
    use chrono::Utc;

    fn create_test_pipeline() -> Pipeline {
        Pipeline {
            id: "test-pipe".to_string(),
            number: 1234,
            state: "success".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            vcs: VcsInfo {
                branch: "main".to_string(),
                revision: "a1b2c3d".to_string(),
                commit_subject: "feat: add webhook retry logic".to_string(),
                commit_author_name: "alice".to_string(),
                commit_timestamp: Utc::now(),
            },
            trigger: TriggerInfo {
                trigger_type: "webhook".to_string(),
            },
            project_slug: "gh/acme/api-service".to_string(),
        }
    }

    #[test]
    fn test_pipeline_detail_screen_new() {
        let pipeline = create_test_pipeline();
        let screen = PipelineDetailScreen::new(pipeline);

        assert!(!screen.workflows.is_empty());
        assert!(!screen.jobs.is_empty());
        assert_eq!(screen.selected_workflow_index, 0);
        assert_eq!(screen.focus, PanelFocus::Workflows);
    }

    #[test]
    fn test_select_workflow() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        let initial_jobs_count = screen.jobs.len();
        screen.select_workflow(1);

        assert_eq!(screen.selected_workflow_index, 1);
        // Jobs should be reloaded
        assert_eq!(screen.jobs.len(), initial_jobs_count); // Mock data returns same count
    }

    #[test]
    fn test_workflow_navigation() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        screen.select_next_workflow();
        assert_eq!(screen.selected_workflow_index, 1);

        screen.select_previous_workflow();
        assert_eq!(screen.selected_workflow_index, 0);
    }

    #[test]
    fn test_job_navigation() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        screen.focus = PanelFocus::Jobs;
        screen.select_next_job();
        assert_eq!(screen.selected_job_index, Some(1));

        screen.select_previous_job();
        assert_eq!(screen.selected_job_index, Some(0));
    }

    #[test]
    fn test_filter_failed_jobs() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        screen.show_only_failed = true;
        let filtered = screen.get_filtered_jobs();

        // Check that only failed jobs are returned
        assert!(filtered.iter().all(|job| job.status == "failed"));
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long string", 10), "this is...");
    }
}
