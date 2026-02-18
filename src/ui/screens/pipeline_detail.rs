/// Screen 2: Pipeline Detail with Workflow Tree + Jobs List
///
/// This screen shows:
/// - Left Panel (30%): Workflow tree with arrows indicating selection
/// - Right Panel (70%): Filtered jobs list for the selected workflow
use crate::api::models::{Job, Pipeline, Workflow};
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_INPUT, BG_PANEL, BLOCKED, BORDER,
    BORDER_FOCUSED, FAILED_TEXT, FG_BRIGHT, FG_DIM, FG_PRIMARY, PENDING, RUNNING, SUCCESS,
};
use crate::ui::widgets::breadcrumb::render_breadcrumb;
use crate::ui::widgets::spinner::Spinner;
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
    /// Status filters (checkboxes)
    Filters,
}

/// Action returned from input handling
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineDetailAction {
    /// No action
    None,
    /// Open job log modal for the specified job
    OpenJobLog(Job),
    /// Open SSH modal for the specified job
    OpenSsh(Job),
    /// Go back to pipelines screen
    Back,
    /// Load more jobs (trigger pagination)
    LoadMoreJobs,
    /// Rerun a workflow
    RerunWorkflow(String),
    /// Load jobs for a workflow
    LoadJobs(String),
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
    /// Show only failed jobs (legacy)
    pub show_only_failed: bool,
    /// Status filter: show success jobs
    pub filter_success: bool,
    /// Status filter: show running jobs
    pub filter_running: bool,
    /// Status filter: show failed jobs
    pub filter_failed: bool,
    /// Status filter: show pending jobs
    pub filter_pending: bool,
    /// Status filter: show blocked jobs
    pub filter_blocked: bool,
    /// Currently selected filter checkbox index (0-4)
    pub selected_filter_index: usize,
    /// List state for workflow selection
    pub workflow_list_state: ListState,
    /// List state for job selection
    pub job_list_state: ListState,
    /// Loading workflows
    pub loading_workflows: bool,
    /// Loading jobs
    pub loading_jobs: bool,
    /// Loading more jobs (pagination)
    pub loading_more_jobs: bool,
    /// Next page token for job pagination
    pub next_page_token: Option<String>,
    /// Total jobs count (estimated if more pages exist)
    pub total_jobs_count: Option<usize>,
    /// Show rerun confirmation modal
    pub show_rerun_confirm: bool,
    /// Workflow ID to rerun (if confirmation is shown)
    pub confirm_workflow_id: Option<String>,
    /// Spinner for loading state
    pub spinner: Spinner,
}

impl PipelineDetailScreen {
    /// Create a new pipeline detail screen
    pub fn new(pipeline: Pipeline) -> Self {
        let workflows = Vec::new();
        let selected_workflow_index = 0;

        // Initialize with empty jobs - app will trigger real API loading
        let jobs = Vec::new();

        let selected_job_index = None;

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
            filter_success: true,
            filter_running: true,
            filter_failed: true,
            filter_pending: true,
            filter_blocked: true,
            selected_filter_index: 0,
            workflow_list_state,
            job_list_state,
            loading_workflows: false,
            loading_jobs: false,
            loading_more_jobs: false,
            next_page_token: None,
            show_rerun_confirm: false,
            confirm_workflow_id: None,
            total_jobs_count: None,
            spinner: Spinner::new("Loading..."),
        }
    }

    /// Set workflows from external source (e.g., API)
    pub fn set_workflows(&mut self, workflows: Vec<Workflow>) {
        self.workflows = workflows;
        if !self.workflows.is_empty() {
            self.selected_workflow_index = 0;
            self.workflow_list_state.select(Some(0));
        }
    }

    /// Set jobs from external source (e.g., API)
    pub fn set_jobs(&mut self, jobs: Vec<Job>) {
        self.jobs = jobs;
        if !self.jobs.is_empty() {
            self.selected_job_index = Some(0);
            self.job_list_state.select(Some(0));
        } else {
            self.selected_job_index = None;
            self.job_list_state.select(None);
        }
    }

    /// Set jobs with pagination information
    pub fn set_jobs_with_pagination(
        &mut self,
        jobs: Vec<Job>,
        next_page_token: Option<String>,
        total_count: Option<usize>,
    ) {
        self.jobs = jobs;
        self.next_page_token = next_page_token;
        self.total_jobs_count = total_count;

        if !self.jobs.is_empty() {
            self.selected_job_index = Some(0);
            self.job_list_state.select(Some(0));
        } else {
            self.selected_job_index = None;
            self.job_list_state.select(None);
        }
    }

    /// Append more jobs from pagination
    pub fn append_jobs(&mut self, jobs: Vec<Job>, next_page_token: Option<String>) {
        let current_selected = self.selected_job_index;

        self.jobs.extend(jobs);
        self.next_page_token = next_page_token;

        // Update total count if we know there are no more pages
        if self.next_page_token.is_none() {
            self.total_jobs_count = Some(self.jobs.len());
        }

        // Restore selection
        if let Some(idx) = current_selected {
            self.selected_job_index = Some(idx);
            self.job_list_state.select(Some(idx));
        }
    }

    /// Check if we can load more jobs
    pub fn can_load_more(&self) -> bool {
        self.next_page_token.is_some() && !self.loading_more_jobs
    }

    /// Get pagination info for display
    fn get_pagination_info(&self) -> String {
        let current_count = self.jobs.len();
        let filtered_count = self.get_filtered_jobs().len();

        if let Some(total) = self.total_jobs_count {
            // We know the exact total
            if self.show_only_failed || !self.job_filter.is_empty() {
                format!("(Showing {} of {} total jobs)", filtered_count, total)
            } else {
                format!("(Showing {} of {})", current_count, total)
            }
        } else if self.next_page_token.is_some() {
            // More pages exist, but we don't know the total
            if self.show_only_failed || !self.job_filter.is_empty() {
                format!("(Showing {} of {}+ total jobs)", filtered_count, current_count)
            } else {
                format!("(Showing {} of {}+)", current_count, current_count)
            }
        } else {
            // All jobs loaded
            if self.show_only_failed || !self.job_filter.is_empty() {
                format!("(Showing {} of {} total jobs)", filtered_count, current_count)
            } else {
                format!("(All {} jobs loaded)", current_count)
            }
        }
    }

    /// Select a workflow and reload its jobs
    pub fn select_workflow(&mut self, index: usize) {
        if index >= self.workflows.len() {
            return;
        }

        self.selected_workflow_index = index;
        self.workflow_list_state.select(Some(index));

        // Clear jobs - app will trigger real API loading
        self.jobs.clear();

        // Reset job selection
        self.selected_job_index = None;
        self.job_list_state.select(None);
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

                // Apply status filters
                let matches_status = match job.status.as_str() {
                    "success" | "passed" | "fixed" | "successful" => self.filter_success,
                    "running" | "in_progress" | "in-progress" => self.filter_running,
                    "failed" | "error" | "failure" => self.filter_failed,
                    "pending" | "queued" => self.filter_pending,
                    "blocked" | "waiting" => self.filter_blocked,
                    _ => true, // Show unknown statuses by default
                };

                matches_filter && matches_status
            })
            .collect()
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> PipelineDetailAction {
        // Handle filter mode input
        if self.focus == PanelFocus::Filters {
            return self.handle_filter_input(key);
        }

        match key.code {
            KeyCode::Tab => {
                // Switch focus between panels
                self.focus = match self.focus {
                    PanelFocus::Workflows => PanelFocus::Jobs,
                    PanelFocus::Jobs => PanelFocus::Workflows,
                    PanelFocus::Filters => PanelFocus::Jobs,
                };
                PipelineDetailAction::None
            }
            KeyCode::Up => {
                match self.focus {
                    PanelFocus::Workflows => {
                        self.select_previous_workflow();
                        // Return LoadJobs action to trigger job loading for the selected workflow
                        if !self.workflows.is_empty() {
                            let workflow_id = self.workflows[self.selected_workflow_index].id.clone();
                            return PipelineDetailAction::LoadJobs(workflow_id);
                        }
                    },
                    PanelFocus::Jobs => self.select_previous_job(),
                    PanelFocus::Filters => {},
                }
                PipelineDetailAction::None
            }
            KeyCode::Down => {
                match self.focus {
                    PanelFocus::Workflows => {
                        self.select_next_workflow();
                        // Return LoadJobs action to trigger job loading for the selected workflow
                        if !self.workflows.is_empty() {
                            let workflow_id = self.workflows[self.selected_workflow_index].id.clone();
                            return PipelineDetailAction::LoadJobs(workflow_id);
                        }
                    },
                    PanelFocus::Jobs => self.select_next_job(),
                    PanelFocus::Filters => {},
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
                // Toggle filter focus mode
                if self.focus == PanelFocus::Filters {
                    self.focus = PanelFocus::Jobs;
                } else {
                    self.focus = PanelFocus::Filters;
                    self.selected_filter_index = 0;
                }
                PipelineDetailAction::None
            }
            KeyCode::Char('l') => {
                // Load more jobs if pagination is available
                if self.focus == PanelFocus::Jobs && self.can_load_more() {
                    PipelineDetailAction::LoadMoreJobs
                } else {
                    PipelineDetailAction::None
                }
            }
            KeyCode::Char('R') => {
                // Trigger rerun workflow confirmation
                if self.focus == PanelFocus::Workflows && !self.workflows.is_empty() {
                    let workflow = &self.workflows[self.selected_workflow_index];
                    return PipelineDetailAction::RerunWorkflow(workflow.id.clone());
                }
                PipelineDetailAction::None
            }
            KeyCode::Char('s') => {
                // Open SSH modal for selected job
                if self.focus == PanelFocus::Jobs {
                    if let Some(idx) = self.selected_job_index {
                        let filtered_jobs = self.get_filtered_jobs();
                        if let Some(job) = filtered_jobs.get(idx) {
                            return PipelineDetailAction::OpenSsh((*job).clone());
                        }
                    }
                }
                PipelineDetailAction::None
            }
            KeyCode::Esc => PipelineDetailAction::Back,
            _ => PipelineDetailAction::None,
        }
    }

    /// Handle input when in filter mode
    fn handle_filter_input(&mut self, key: KeyEvent) -> PipelineDetailAction {
        match key.code {
            KeyCode::Left => {
                if self.selected_filter_index > 0 {
                    self.selected_filter_index -= 1;
                }
                PipelineDetailAction::None
            }
            KeyCode::Right | KeyCode::Tab => {
                if self.selected_filter_index < 4 {
                    self.selected_filter_index += 1;
                } else {
                    self.selected_filter_index = 0;
                }
                PipelineDetailAction::None
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                // Toggle the selected filter
                self.toggle_selected_filter();
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
            KeyCode::Esc | KeyCode::Char('f') => {
                // Exit filter mode
                self.focus = PanelFocus::Jobs;
                PipelineDetailAction::None
            }
            _ => PipelineDetailAction::None,
        }
    }

    /// Toggle the currently selected filter checkbox
    fn toggle_selected_filter(&mut self) {
        match self.selected_filter_index {
            0 => self.filter_success = !self.filter_success,
            1 => self.filter_running = !self.filter_running,
            2 => self.filter_failed = !self.filter_failed,
            3 => self.filter_pending = !self.filter_pending,
            4 => self.filter_blocked = !self.filter_blocked,
            _ => {}
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

        if self.loading_workflows {
            // Show loading spinner
            self.spinner.tick();
            self.spinner.set_message("Loading workflows...");
            let inner = block.inner(area);
            f.render_widget(block, area);

            let spinner_widget = self.spinner.render();
            f.render_widget(spinner_widget, inner);
        } else if self.workflows.is_empty() {
            // Show empty state with ASCII art
            let inner = block.inner(area);
            f.render_widget(block, area);

            let empty_message = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  ¯\\_(ツ)_/¯",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "No workflows",
                    Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "found",
                    Style::default().fg(FG_DIM),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(FG_DIM)),
                    Span::styled("'Esc'", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                    Span::styled(" to go back", Style::default().fg(FG_DIM)),
                ]),
            ])
            .alignment(Alignment::Center);

            f.render_widget(empty_message, inner);
        } else {
            // Normal rendering
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

            let list = List::new(items).block(block);

            f.render_stateful_widget(list, area, &mut self.workflow_list_state);
        }
    }

    /// Render the jobs panel (right side)
    fn render_jobs_panel(&mut self, f: &mut Frame, area: Rect) {
        // Split into filter bar, status filters, and job list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Filter bar (text filter)
                Constraint::Length(1), // Status filter checkboxes
                Constraint::Min(0),    // Job list
            ])
            .split(area);

        // Render filter bar
        self.render_filter_bar(f, chunks[0]);

        // Render status filters
        self.render_status_filters(f, chunks[1]);

        // Render job list
        self.render_job_list(f, chunks[2]);
    }

    /// Render the filter bar
    fn render_filter_bar(&self, f: &mut Frame, area: Rect) {
        let filter_display = if self.job_filter.is_empty() {
            "_____".to_string()
        } else {
            self.job_filter.clone()
        };

        let filter_info = self.get_filter_info();

        let line = Line::from(vec![
            Span::styled("Filter: [", Style::default().fg(FG_PRIMARY)),
            Span::styled(&filter_display, Style::default().fg(FG_DIM)),
            Span::styled("]  ", Style::default().fg(FG_PRIMARY)),
            Span::styled(filter_info, Style::default().fg(FG_DIM)),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_INPUT));

        let paragraph = Paragraph::new(line).block(block);
        f.render_widget(paragraph, area);
    }

    /// Get filter info display string
    fn get_filter_info(&self) -> String {
        let enabled_count = [
            self.filter_success,
            self.filter_running,
            self.filter_failed,
            self.filter_pending,
            self.filter_blocked,
        ]
        .iter()
        .filter(|&&enabled| enabled)
        .count();

        if enabled_count == 5 {
            "All statuses".to_string()
        } else {
            format!("Filters: {}/5 statuses", enabled_count)
        }
    }

    /// Render status filter checkboxes
    fn render_status_filters(&self, f: &mut Frame, area: Rect) {
        let success_checkbox = if self.filter_success { "☑" } else { "☐" };
        let running_checkbox = if self.filter_running { "☑" } else { "☐" };
        let failed_checkbox = if self.filter_failed { "☑" } else { "☐" };
        let pending_checkbox = if self.filter_pending { "☑" } else { "☐" };
        let blocked_checkbox = if self.filter_blocked { "☑" } else { "☐" };

        let in_filter_mode = self.focus == PanelFocus::Filters;

        let spans = vec![
            // Success checkbox
            Span::styled(
                format!("{} Success  ", success_checkbox),
                Style::default().fg(SUCCESS).add_modifier(
                    if in_filter_mode && self.selected_filter_index == 0 {
                        Modifier::BOLD | Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    }
                ),
            ),
            // Running checkbox
            Span::styled(
                format!("{} Running  ", running_checkbox),
                Style::default().fg(RUNNING).add_modifier(
                    if in_filter_mode && self.selected_filter_index == 1 {
                        Modifier::BOLD | Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    }
                ),
            ),
            // Failed checkbox
            Span::styled(
                format!("{} Failed  ", failed_checkbox),
                Style::default().fg(FAILED_TEXT).add_modifier(
                    if in_filter_mode && self.selected_filter_index == 2 {
                        Modifier::BOLD | Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    }
                ),
            ),
            // Pending checkbox
            Span::styled(
                format!("{} Pending  ", pending_checkbox),
                Style::default().fg(PENDING).add_modifier(
                    if in_filter_mode && self.selected_filter_index == 3 {
                        Modifier::BOLD | Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    }
                ),
            ),
            // Blocked checkbox
            Span::styled(
                format!("{} Blocked", blocked_checkbox),
                Style::default().fg(BLOCKED).add_modifier(
                    if in_filter_mode && self.selected_filter_index == 4 {
                        Modifier::BOLD | Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    }
                ),
            ),
        ];

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, area);
    }

    /// Render the job list
    fn render_job_list(&mut self, f: &mut Frame, area: Rect) {
        let filtered_jobs = self.get_filtered_jobs();

        // Calculate status summary
        let status_summary = self.calculate_status_summary();

        // Get workflow name (default if workflows not loaded yet)
        let workflow_name = if !self.workflows.is_empty() && self.selected_workflow_index < self.workflows.len() {
            &self.workflows[self.selected_workflow_index].name
        } else {
            "Loading..."
        };

        let title = format!(
            " JOBS › {} {} ",
            truncate_string(workflow_name, 20),
            status_summary
        );

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

        if self.loading_jobs {
            // Show loading spinner
            self.spinner.tick();
            self.spinner.set_message("Loading jobs...");
            let inner = block.inner(area);
            f.render_widget(block, area);

            let spinner_widget = self.spinner.render();
            f.render_widget(spinner_widget, inner);
        } else if filtered_jobs.is_empty() {
            // Show empty state
            let inner = block.inner(area);
            f.render_widget(block, area);

            let empty_message = if self.jobs.is_empty() {
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ¯\\_(ツ)_/¯",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "No jobs found",
                        Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        "for this workflow",
                        Style::default().fg(FG_DIM),
                    )),
                ])
            } else {
                // Jobs exist but filtered out
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  (•_•)",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "No jobs match filters",
                        Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Press ", Style::default().fg(FG_DIM)),
                        Span::styled("'f'", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                        Span::styled(" to toggle filters or ", Style::default().fg(FG_DIM)),
                        Span::styled("'Tab'", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
                        Span::styled(" to switch panel", Style::default().fg(FG_DIM)),
                    ]),
                ])
            }
            .alignment(Alignment::Center);

            f.render_widget(empty_message, inner);
        } else {
            // Normal rendering
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

            // Add "Load More" button if pagination is available
            if self.can_load_more() {
                let load_more_text = if self.loading_more_jobs {
                    "● Loading more jobs...".to_string()
                } else {
                    let count_info = self.get_pagination_info();
                    format!("[Load More Jobs] {}", count_info)
                };

                let load_more_line = Line::from(vec![
                    Span::raw("     "),
                    Span::styled(
                        load_more_text,
                        Style::default()
                            .fg(if self.loading_more_jobs {
                                RUNNING
                            } else {
                                ACCENT
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);

                items.push(ListItem::new(vec![Line::from(""), load_more_line]));
            }

            let list = List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .fg(ACCENT)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(list, area, &mut self.job_list_state);
        }
    }

    /// Calculate status summary for jobs
    fn calculate_status_summary(&self) -> String {
        let mut passed = 0;
        let mut failed = 0;
        let mut running = 0;

        for job in &self.jobs {
            match job.status.as_str() {
                "success" | "passed" | "fixed" | "successful" => passed += 1,
                "failed" | "error" | "failure" => failed += 1,
                "running" | "in_progress" | "in-progress" => running += 1,
                _ => {}
            }
        }

        let mut parts = Vec::new();

        if passed > 0 {
            parts.push(format!("✓ {}", passed));
        }
        if running > 0 {
            parts.push(format!("● {}", running));
        }
        if failed > 0 {
            parts.push(format!("✗ {}", failed));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("│ {}", parts.join("  "))
        }
    }

    /// Render footer with keyboard shortcuts
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let mut footer_items = vec![
            Span::styled("[↑↓]", Style::default().fg(ACCENT)),
            Span::styled(" Nav  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[Tab]", Style::default().fg(ACCENT)),
            Span::styled(" Switch  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[⏎]", Style::default().fg(ACCENT)),
            Span::styled(" View Logs  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[s]", Style::default().fg(ACCENT)),
            Span::styled(" SSH  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[f]", Style::default().fg(ACCENT)),
            Span::styled(" Toggle Filters  ", Style::default().fg(FG_PRIMARY)),
        ];

        // Add filter mode shortcuts if in filter mode
        if self.focus == PanelFocus::Filters {
            footer_items.push(Span::styled("[←→]", Style::default().fg(ACCENT)));
            footer_items.push(Span::styled(" Navigate  ", Style::default().fg(FG_PRIMARY)));
            footer_items.push(Span::styled("[Space]", Style::default().fg(ACCENT)));
            footer_items.push(Span::styled(" Toggle  ", Style::default().fg(FG_PRIMARY)));
        }

        // Add "Load More" to footer if pagination is available
        if self.can_load_more() && self.focus == PanelFocus::Jobs {
            footer_items.push(Span::styled("[l]", Style::default().fg(ACCENT)));
            footer_items.push(Span::styled(" Load More  ", Style::default().fg(FG_PRIMARY)));
        }
        // Add "Rerun Workflow" to footer if focused on workflows panel
        if self.focus == PanelFocus::Workflows {
            footer_items.push(Span::styled("[R]", Style::default().fg(ACCENT)));
            footer_items.push(Span::styled(" Rerun Workflow  ", Style::default().fg(FG_PRIMARY)));
        }

        footer_items.push(Span::styled("[?]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Help  ", Style::default().fg(FG_PRIMARY)));
        footer_items.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Back", Style::default().fg(FG_PRIMARY)));

        let footer = Paragraph::new(Line::from(footer_items))
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

        // Should start with empty workflows and jobs - app will load from API
        assert!(screen.workflows.is_empty());
        assert!(screen.jobs.is_empty());
        assert_eq!(screen.selected_workflow_index, 0);
        assert_eq!(screen.focus, PanelFocus::Workflows);
    }

    #[test]
    fn test_select_workflow() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        // Add some test workflows using setter
        use crate::api::models::Workflow;
        use chrono::Utc;
        let workflows = vec![
            Workflow {
                id: "wf1".to_string(),
                name: "build".to_string(),
                status: "success".to_string(),
                created_at: Utc::now(),
                stopped_at: Some(Utc::now()),
                pipeline_id: "test-pipeline".to_string(),
            },
            Workflow {
                id: "wf2".to_string(),
                name: "test".to_string(),
                status: "success".to_string(),
                created_at: Utc::now(),
                stopped_at: Some(Utc::now()),
                pipeline_id: "test-pipeline".to_string(),
            },
        ];
        screen.set_workflows(workflows);

        screen.select_workflow(1);
        assert_eq!(screen.selected_workflow_index, 1);
        // Jobs should be cleared when switching workflows
        assert_eq!(screen.jobs.len(), 0);
    }

    #[test]
    fn test_workflow_navigation() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        // Add test workflows
        use crate::api::models::Workflow;
        use chrono::Utc;
        let workflows = vec![
            Workflow {
                id: "wf1".to_string(),
                name: "build".to_string(),
                status: "success".to_string(),
                created_at: Utc::now(),
                stopped_at: Some(Utc::now()),
                pipeline_id: "test-pipeline".to_string(),
            },
            Workflow {
                id: "wf2".to_string(),
                name: "test".to_string(),
                status: "success".to_string(),
                created_at: Utc::now(),
                stopped_at: Some(Utc::now()),
                pipeline_id: "test-pipeline".to_string(),
            },
        ];
        screen.set_workflows(workflows);

        screen.select_next_workflow();
        assert_eq!(screen.selected_workflow_index, 1);

        screen.select_previous_workflow();
        assert_eq!(screen.selected_workflow_index, 0);
    }

    #[test]
    fn test_job_navigation() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        // Add test jobs
        use crate::api::models::Job;
        use chrono::Utc;
        let jobs = vec![
            Job {
                id: "job1".to_string(),
                name: "build".to_string(),
                status: "success".to_string(),
                job_number: 1,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(60),
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: "job2".to_string(),
                name: "test".to_string(),
                status: "success".to_string(),
                job_number: 2,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(60),
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
        ];
        screen.set_jobs(jobs);

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

        // Add test jobs with mixed statuses
        use crate::api::models::Job;
        use chrono::Utc;
        let jobs = vec![
            Job {
                id: "job1".to_string(),
                name: "build".to_string(),
                status: "success".to_string(),
                job_number: 1,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(60),
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: "job2".to_string(),
                name: "test".to_string(),
                status: "failed".to_string(),
                job_number: 2,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(60),
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
        ];
        screen.set_jobs(jobs);

        // Enable only failed filter
        screen.filter_success = false;
        screen.filter_running = false;
        screen.filter_failed = true;
        screen.filter_pending = false;
        screen.filter_blocked = false;

        let filtered = screen.get_filtered_jobs();

        // Check that only failed jobs are returned
        assert!(filtered.iter().all(|job| job.status == "failed"));
    }

    #[test]
    fn test_status_filters() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        // Add test jobs with various statuses
        use crate::api::models::Job;
        use chrono::Utc;
        let jobs = vec![
            Job {
                id: "job1".to_string(),
                name: "build".to_string(),
                status: "success".to_string(),
                job_number: 1,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(60),
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: "job2".to_string(),
                name: "test".to_string(),
                status: "failed".to_string(),
                job_number: 2,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(60),
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: "job3".to_string(),
                name: "deploy".to_string(),
                status: "running".to_string(),
                job_number: 3,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: None,
                duration: None,
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
        ];
        screen.set_jobs(jobs);

        // Test filtering by specific status
        screen.filter_success = false;
        screen.filter_running = true;
        screen.filter_failed = true;
        screen.filter_pending = true;
        screen.filter_blocked = true;

        let filtered = screen.get_filtered_jobs();
        // No success jobs should be included
        assert!(filtered.iter().all(|job| job.status != "success"));

        // Test filtering only failed jobs
        screen.filter_success = false;
        screen.filter_running = false;
        screen.filter_failed = true;
        screen.filter_pending = false;
        screen.filter_blocked = false;

        let filtered = screen.get_filtered_jobs();
        // Only failed jobs should be included
        assert!(filtered
            .iter()
            .all(|job| job.status == "failed" || job.status == "error" || job.status == "failure"));
    }

    #[test]
    fn test_filter_toggle() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        // Test toggling filters
        let initial = screen.filter_success;
        screen.selected_filter_index = 0;
        screen.toggle_selected_filter();
        assert_eq!(screen.filter_success, !initial);

        let initial = screen.filter_running;
        screen.selected_filter_index = 1;
        screen.toggle_selected_filter();
        assert_eq!(screen.filter_running, !initial);

        let initial = screen.filter_failed;
        screen.selected_filter_index = 2;
        screen.toggle_selected_filter();
        assert_eq!(screen.filter_failed, !initial);
    }

    #[test]
    fn test_get_filter_info() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        // All filters enabled
        assert_eq!(screen.get_filter_info(), "All statuses");

        // Some filters disabled
        screen.filter_success = false;
        screen.filter_running = false;
        assert_eq!(screen.get_filter_info(), "Filters: 3/5 statuses");

        // Only one filter enabled
        screen.filter_failed = false;
        screen.filter_pending = false;
        assert_eq!(screen.get_filter_info(), "Filters: 1/5 statuses");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long string", 10), "this is...");
    }
}
