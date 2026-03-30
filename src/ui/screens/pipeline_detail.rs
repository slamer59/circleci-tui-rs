/// Screen 2: Pipeline Detail with Workflow Tree + Jobs List
///
/// This screen shows:
/// - Left Panel (30%): Workflow tree with arrows indicating selection
/// - Right Panel (70%): Filtered jobs list for the selected workflow
use crate::api::models::{Job, Pipeline, Workflow};
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_PANEL, FG_BRIGHT, FG_DIM, FG_PRIMARY, RUNNING,
    SECONDARY,
};
use crate::ui::utils::truncate_string;
use crate::ui::widgets::breadcrumb::render_breadcrumb;
use crate::ui::widgets::faceted_search::{Facet, FacetedSearchBar};
use crate::ui::widgets::line_range_modal::{LineRangeAction, LineRangeModal};
use crate::ui::widgets::powerline::PowerlineBar;
use crate::ui::widgets::spinner::Spinner;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table,
        TableState,
    },
    Frame,
};
use std::collections::HashMap;

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
    /// Copy logs for the specified job number
    CopyLogs(u32),
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
    /// Faceted search bar for status and duration filtering
    pub faceted_search: FacetedSearchBar,
    /// List state for workflow selection
    pub workflow_list_state: ListState,
    /// Table state for job selection
    pub job_table_state: TableState,
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
    /// Workflow ID to rerun (if confirmation is shown)
    pub confirm_workflow_id: Option<String>,
    /// Spinner for loading state
    pub spinner: Spinner,
    /// Cache of fetched logs by job number (populated by prefetch system for yank operations)
    pub log_cache: HashMap<u32, Vec<String>>,
    /// Pending log fetch request (job number)
    pub pending_log_fetch: Option<u32>,
    /// Previous visible job numbers for prefetch tracking
    pub previous_visible_jobs: Vec<u32>,
    /// Line range modal for yank operations
    pub line_range_modal: LineRangeModal,
    /// Job number pending copy operation (waiting for modal confirmation)
    pub pending_copy_job: Option<u32>,
    /// Powerline status bar
    pub powerline: PowerlineBar,
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

        let mut job_table_state = TableState::default();
        if !jobs.is_empty() {
            job_table_state.select(Some(0));
        }

        // Create faceted search bar with Status and Duration facets
        let facets = vec![
            // Facet 0: Status filter
            Facet::new(
                "●",
                vec![
                    "All".to_string(),
                    "success".to_string(),
                    "running".to_string(),
                    "failed".to_string(),
                    "pending".to_string(),
                    "blocked".to_string(),
                ],
                0, // Default: All
            ),
            // Facet 1: Duration filter
            Facet::new(
                "⏱",
                vec![
                    "All durations".to_string(),
                    "Quick (< 1min)".to_string(),
                    "Short (1-5min)".to_string(),
                    "Medium (5-15min)".to_string(),
                    "Long (15-30min)".to_string(),
                    "Very Long (>30min)".to_string(),
                ],
                0, // Default: All durations
            ),
        ];

        let faceted_search = FacetedSearchBar::new(facets);

        Self {
            pipeline,
            workflows,
            jobs,
            selected_workflow_index,
            selected_job_index,
            focus: PanelFocus::Workflows,
            faceted_search,
            workflow_list_state,
            job_table_state,
            loading_workflows: false,
            loading_jobs: false,
            loading_more_jobs: false,
            next_page_token: None,
            confirm_workflow_id: None,
            total_jobs_count: None,
            spinner: Spinner::new("Loading..."),
            log_cache: HashMap::new(),
            pending_log_fetch: None,
            line_range_modal: LineRangeModal::new(),
            pending_copy_job: None,
            powerline: PowerlineBar::new(),
            previous_visible_jobs: Vec::new(),
        }
    }

    /// Get current filter preferences for saving
    pub fn get_filter_preferences(&self) -> crate::preferences::PipelineDetailFilterPrefs {
        use crate::preferences::PipelineDetailFilterPrefs;

        PipelineDetailFilterPrefs {
            status_index: self.faceted_search.get_facet_selection(0),
            duration_index: self.faceted_search.get_facet_selection(1),
        }
    }

    /// Apply saved filter preferences
    pub fn apply_filter_preferences(
        &mut self,
        prefs: &crate::preferences::PipelineDetailFilterPrefs,
    ) {
        self.faceted_search
            .set_facet_selection(0, prefs.status_index);
        self.faceted_search
            .set_facet_selection(1, prefs.duration_index);
    }

    /// Set workflows from external source (e.g., API)
    pub fn set_workflows(&mut self, workflows: Vec<Workflow>) {
        self.workflows = workflows;
        if !self.workflows.is_empty() {
            self.selected_workflow_index = 0;
            self.workflow_list_state.select(Some(0));
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
            self.job_table_state.select(Some(0));
        } else {
            self.selected_job_index = None;
            self.job_table_state.select(None);
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
            self.job_table_state.select(Some(idx));
        }
    }

    /// Check if we can load more jobs
    pub fn can_load_more(&self) -> bool {
        self.next_page_token.is_some() && !self.loading_more_jobs
    }

    /// Check if logs need to be loaded for the currently selected job
    /// Returns Some(job_number) if logs should be loaded, None otherwise
    /// Get pagination info for display
    fn get_pagination_info(&self) -> String {
        let current_count = self.jobs.len();
        let filtered_count = self.get_filtered_jobs().len();
        let has_filter = self.faceted_search.is_filtered();

        if let Some(total) = self.total_jobs_count {
            // We know the exact total
            if has_filter {
                format!("(Showing {} of {} total jobs)", filtered_count, total)
            } else {
                format!("(Showing {} of {})", current_count, total)
            }
        } else if self.next_page_token.is_some() {
            // More pages exist, but we don't know the total
            if has_filter {
                format!(
                    "(Showing {} of {}+ total jobs)",
                    filtered_count, current_count
                )
            } else {
                format!("(Showing {} of {}+)", current_count, current_count)
            }
        } else {
            // All jobs loaded
            if has_filter {
                format!(
                    "(Showing {} of {} total jobs)",
                    filtered_count, current_count
                )
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
        self.job_table_state.select(None);
    }

    /// Get the filtered list of jobs
    fn get_filtered_jobs(&self) -> Vec<&Job> {
        // Get filter values from faceted search bar
        let status_filter = self.faceted_search.get_filter_value(0).unwrap_or("All");
        let duration_filter = self
            .faceted_search
            .get_filter_value(1)
            .unwrap_or("All durations");

        self.jobs
            .iter()
            .filter(|job| {
                // Apply status filter
                let matches_status = if status_filter == "All" {
                    true
                } else {
                    match job.status.as_str() {
                        "success" | "passed" | "fixed" | "successful" => status_filter == "success",
                        "running" | "in_progress" | "in-progress" => status_filter == "running",
                        "failed" | "error" | "failure" => status_filter == "failed",
                        "pending" | "queued" => status_filter == "pending",
                        "blocked" | "waiting" => status_filter == "blocked",
                        _ => true, // Show unknown statuses by default
                    }
                };

                // Apply duration filter
                let matches_duration = match duration_filter {
                    "All durations" => true,
                    "Quick (< 1min)" => job.duration.map(|d| d < 60).unwrap_or(false),
                    "Short (1-5min)" => job
                        .duration
                        .map(|d| (60..300).contains(&d))
                        .unwrap_or(false),
                    "Medium (5-15min)" => job
                        .duration
                        .map(|d| (300..900).contains(&d))
                        .unwrap_or(false),
                    "Long (15-30min)" => job
                        .duration
                        .map(|d| (900..1800).contains(&d))
                        .unwrap_or(false),
                    "Very Long (>30min)" => job.duration.map(|d| d >= 1800).unwrap_or(false),
                    _ => true,
                };

                matches_status && matches_duration
            })
            .collect()
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> PipelineDetailAction {
        // Check if line range modal is open first
        if self.line_range_modal.is_visible() {
            match self.line_range_modal.handle_input(key) {
                LineRangeAction::Confirm(range_str) => {
                    if let Some(job_number) = self.pending_copy_job.take() {
                        self.handle_copy_logs_range(job_number, &range_str);
                    }
                    return PipelineDetailAction::None;
                }
                LineRangeAction::Cancel => {
                    self.pending_copy_job = None;
                    return PipelineDetailAction::None;
                }
                LineRangeAction::None => return PipelineDetailAction::None,
            }
        }

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
                            let workflow_id =
                                self.workflows[self.selected_workflow_index].id.clone();
                            return PipelineDetailAction::LoadJobs(workflow_id);
                        }
                    }
                    PanelFocus::Jobs => self.select_previous_job(),
                    PanelFocus::Filters => {}
                }
                PipelineDetailAction::None
            }
            KeyCode::Down => {
                match self.focus {
                    PanelFocus::Workflows => {
                        self.select_next_workflow();
                        // Return LoadJobs action to trigger job loading for the selected workflow
                        if !self.workflows.is_empty() {
                            let workflow_id =
                                self.workflows[self.selected_workflow_index].id.clone();
                            return PipelineDetailAction::LoadJobs(workflow_id);
                        }
                    }
                    PanelFocus::Jobs => self.select_next_job(),
                    PanelFocus::Filters => {}
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
            KeyCode::Char('c') => {
                // Show line range input modal
                if self.focus == PanelFocus::Jobs {
                    if let Some(job) = self.get_selected_job() {
                        let job_number = job.job_number;

                        // Check if logs are cached
                        if let Some(logs) = self.log_cache.get(&job_number) {
                            // Show modal with total line count
                            self.line_range_modal.show(logs.len());
                            self.pending_copy_job = Some(job_number);
                        } else {
                            // Trigger log fetch first
                            self.powerline.set_loading("Loading logs...".to_string());
                            self.pending_log_fetch = Some(job_number);
                            self.pending_copy_job = Some(job_number);
                            return PipelineDetailAction::CopyLogs(job_number);
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
            KeyCode::Esc | KeyCode::Char('f') => {
                // Exit filter mode (or close dropdown if open)
                if self.faceted_search.is_active() {
                    self.faceted_search.handle_key(key.code);
                } else {
                    self.focus = PanelFocus::Jobs;
                }
                PipelineDetailAction::None
            }
            _ => {
                // Let faceted search handle all other keys
                let handled = self.faceted_search.handle_key(key.code);
                if handled {
                    // Reset job selection when filter changes
                    let filtered_jobs = self.get_filtered_jobs();
                    if !filtered_jobs.is_empty() {
                        self.selected_job_index = Some(0);
                        self.job_table_state.select(Some(0));
                    } else {
                        self.selected_job_index = None;
                        self.job_table_state.select(None);
                    }
                }
                PipelineDetailAction::None
            }
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
        self.job_table_state.select(Some(next));
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
        self.job_table_state.select(Some(prev));
    }

    /// Render the pipeline detail screen
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Check if powerline should be visible (only when showing notification)
        let show_powerline = !matches!(
            self.powerline.content,
            crate::ui::widgets::powerline::PowerlineContent::Empty
        );

        // Main layout: Header Panel (includes breadcrumb) | Body | Powerline (optional) | Footer
        let main_chunks = if show_powerline {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5), // Header panel (2 header + 1 breadcrumb + 2 borders)
                    Constraint::Min(0),    // Body
                    Constraint::Length(1), // Powerline (only when visible)
                    Constraint::Length(1), // Footer
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5), // Header panel (2 header + 1 breadcrumb + 2 borders)
                    Constraint::Min(0),    // Body
                    Constraint::Length(1), // Footer
                ])
                .split(area)
        };

        // Render combined header panel
        self.render_header_panel(f, main_chunks[0]);

        // Split body into left (workflows) and right (jobs) panels
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_chunks[1]);

        // Render left panel (workflow tree)
        self.render_workflows_panel(f, body_chunks[0]);

        // Render right panel (jobs list)
        self.render_jobs_panel(f, body_chunks[1]);

        // Render powerline if visible
        if show_powerline {
            self.powerline.render(f, main_chunks[2]);
            // Render footer at index 3
            self.render_footer(f, main_chunks[3]);
        } else {
            // Render footer at index 2 (no powerline)
            self.render_footer(f, main_chunks[2]);
        }

        // Render line range modal last (on top)
        self.line_range_modal.render(f, area);
    }

    /// Render the combined header panel with pipeline information and breadcrumb
    fn render_header_panel(&self, f: &mut Frame, area: Rect) {
        // Create bordered block for header panel
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(BG_PANEL));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Split inner area into header (2 lines) and breadcrumb (1 line)
        let inner_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header (2 lines)
                Constraint::Length(1), // Breadcrumb (1 line)
            ])
            .split(inner);

        // Render header content
        let project = self
            .pipeline
            .project_slug
            .split('/')
            .next_back()
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
            Span::styled(&self.pipeline.vcs.branch, Style::default().fg(FG_BRIGHT)),
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
        f.render_widget(header, inner_chunks[0]);

        // Render breadcrumb
        let pipeline_num = format!("pipeline #{}", self.pipeline.number);
        let breadcrumb = render_breadcrumb(&[project, &pipeline_num]);
        f.render_widget(breadcrumb, inner_chunks[1]);
    }

    /// Render the workflows panel (left side)
    fn render_workflows_panel(&mut self, f: &mut Frame, area: Rect) {
        let (border_style, border_type, title_style) = if self.focus == PanelFocus::Workflows {
            (
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                BorderType::Double,
                Style::default().fg(SECONDARY).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(ACCENT),
                BorderType::Rounded,
                Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
            )
        };

        let block = Block::default()
            .title(" WORKFLOWS ")
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
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
                Line::from(Span::styled("found", Style::default().fg(FG_DIM))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(FG_DIM)),
                    Span::styled(
                        "'Esc'",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
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
                                Style::default().fg(status_col).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!("{:<20} ", truncate_string(&workflow.name, 18)),
                                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(duration, Style::default().fg(ACCENT)),
                        ])
                    } else {
                        Line::from(vec![
                            Span::styled(arrow, Style::default().fg(FG_DIM)),
                            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
                            Span::styled(
                                format!("{:<20} ", truncate_string(&workflow.name, 18)),
                                Style::default().fg(FG_DIM),
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
        // Split into faceted search bar and job list
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4), // Faceted search bar (3 content + 1 top border, no bottom)
                Constraint::Min(0),    // Job list
            ])
            .split(area);

        // Render faceted search bar (buttons only, not dropdown)
        self.render_faceted_search_bar(f, chunks[0]);

        // Render job list
        self.render_job_list(f, chunks[1]);

        // Render dropdown LAST for proper z-ordering
        if self.faceted_search.is_active() {
            self.faceted_search.render_dropdown_only(f, chunks[0]);
        }
    }

    /// Render the faceted search bar (status and duration filters)
    fn render_faceted_search_bar(&mut self, f: &mut Frame, area: Rect) {
        // Determine border style based on focus - match pipelines.rs styling
        let (border_style, border_type, title_style) =
            if self.focus == PanelFocus::Filters || self.faceted_search.is_active() {
                (
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    BorderType::Double,
                    Style::default().fg(SECONDARY).add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    Style::default().fg(ACCENT),
                    BorderType::Rounded,
                    Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
                )
            };

        // Create bordered block for filter bar (complete borders like pipelines.rs)
        let block = Block::default()
            .title(" FILTERS ")
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
            .style(Style::default().bg(BG_PANEL));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Render only the filter buttons inside the panel (dropdown is rendered separately for z-ordering)
        self.faceted_search.render_filter_bar_only(f, inner);
    }

    /// Create a table row for a job
    fn create_job_row(job: &Job) -> Row<'static> {
        let status_col = get_status_color(&job.status);

        // Format time
        let time = if let Some(started) = job.started_at {
            format!("{}", started.format("%H:%M"))
        } else {
            "pending".to_string()
        };

        // Format duration
        let duration = job.duration_formatted();

        // Get status message (owned string)
        let status_message = match job.status.as_str() {
            "success" => "Completed successfully".to_string(),
            "failed" => "Connection timeout".to_string(),
            "running" => "In progress...".to_string(),
            "blocked" => "Waiting for dependencies".to_string(),
            "pending" => "Queued".to_string(),
            _ => job.status.clone(),
        };

        // Clone job name to make it owned
        let job_name = job.name.clone();

        // Create cells with 2 lines each
        let status_cell = Cell::from(Text::from(vec![
            Line::from(Span::styled("● ", Style::default().fg(status_col))),
            Line::from("  "),
        ]));

        let time_cell = Cell::from(Text::from(vec![
            Line::from(Span::styled(time, Style::default().fg(FG_DIM))),
            Line::from("      "),
        ]));

        let job_name_cell = Cell::from(Text::from(vec![
            Line::from(Span::styled(job_name, Style::default().fg(FG_PRIMARY))),
            Line::from(Span::styled(
                format!("  {}", status_message),
                Style::default().fg(FG_DIM),
            )),
        ]));

        let duration_cell = Cell::from(Text::from(vec![
            Line::from(Span::styled(duration, Style::default().fg(FG_DIM))),
            Line::from("        "),
        ]));

        Row::new(vec![status_cell, time_cell, job_name_cell, duration_cell]).height(2)
    }

    /// Render the job list
    fn render_job_list(&mut self, f: &mut Frame, area: Rect) {
        let filtered_jobs = self.get_filtered_jobs();

        // Calculate status summary
        let status_summary = self.calculate_status_summary();

        // Get workflow name (default if workflows not loaded yet)
        let workflow_name =
            if !self.workflows.is_empty() && self.selected_workflow_index < self.workflows.len() {
                &self.workflows[self.selected_workflow_index].name
            } else {
                "Loading..."
            };

        let title = format!(
            " JOBS › {} {} ",
            truncate_string(workflow_name, 20),
            status_summary
        );

        let (border_style, border_type, title_style) = if self.focus == PanelFocus::Jobs {
            (
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                BorderType::Double,
                Style::default().fg(SECONDARY).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(ACCENT),
                BorderType::Rounded,
                Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
            )
        };

        let block = Block::default()
            .title(title)
            .title_style(title_style)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style)
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
                        Span::styled(
                            "'f'",
                            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" to toggle filters or ", Style::default().fg(FG_DIM)),
                        Span::styled(
                            "'Tab'",
                            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" to switch panel", Style::default().fg(FG_DIM)),
                    ]),
                ])
            }
            .alignment(Alignment::Center);

            f.render_widget(empty_message, inner);
        } else {
            // Normal rendering with Table widget - compact fixed columns, JOB NAME fills remaining
            let widths = [
                Constraint::Length(3),  // STATUS: icon (compact)
                Constraint::Length(8),  // TIME: HH:MM format (compact)
                Constraint::Fill(1),    // JOB NAME: expand to fill available space
                Constraint::Length(10), // DURATION: time display (compact)
            ];

            // Create header row
            let header = Row::new(vec![
                Cell::from(""),
                Cell::from(Span::styled(
                    "TIME",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "JOB NAME",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "DURATION",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
            ])
            .height(1);

            // Map filtered jobs to rows
            let mut rows: Vec<Row> = filtered_jobs
                .iter()
                .map(|job| Self::create_job_row(job))
                .collect();

            // Add "Load More" button if pagination is available
            if self.can_load_more() {
                let load_more_text = if self.loading_more_jobs {
                    "Loading more jobs...".to_string()
                } else {
                    let count_info = self.get_pagination_info();
                    format!("[Load More Jobs] {}", count_info)
                };

                let load_more_row = Row::new(vec![
                    Cell::from(""),
                    Cell::from(""),
                    Cell::from(Text::from(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            load_more_text,
                            Style::default()
                                .fg(if self.loading_more_jobs {
                                    RUNNING
                                } else {
                                    ACCENT
                                })
                                .add_modifier(Modifier::BOLD),
                        )),
                    ])),
                    Cell::from(""),
                ])
                .height(2);

                rows.push(load_more_row);
            }

            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .highlight_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

            f.render_stateful_widget(table, area, &mut self.job_table_state);
        }
    }

    /// Calculate status summary for jobs
    /// Calculate status summary for jobs with fixed-width formatting
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

        // Use fixed-width formatting (2 digits) so single-digit numbers don't shift
        if passed > 0 {
            parts.push(format!("✓ {:>2}", passed));
        }
        if running > 0 {
            parts.push(format!("● {:>2}", running));
        }
        if failed > 0 {
            parts.push(format!("✗ {:>2}", failed));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("│ {}", parts.join("  "))
        }
    }
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
        ];

        // Add [y]Copy when Jobs panel is focused and a job is selected
        if self.focus == PanelFocus::Jobs && self.selected_job_index.is_some() {
            footer_items.push(Span::styled("[c]", Style::default().fg(ACCENT)));
            footer_items.push(Span::styled(" Copy  ", Style::default().fg(FG_PRIMARY)));
        }

        footer_items.push(Span::styled("[f]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Filters  ", Style::default().fg(FG_PRIMARY)));

        // Add filter mode shortcuts if in filter mode
        if self.focus == PanelFocus::Filters {
            if self.faceted_search.is_active() {
                // Dropdown is open
                footer_items.push(Span::styled("[↑↓]", Style::default().fg(ACCENT)));
                footer_items.push(Span::styled(" Navigate  ", Style::default().fg(FG_PRIMARY)));
                footer_items.push(Span::styled("[⏎]", Style::default().fg(ACCENT)));
                footer_items.push(Span::styled(" Select  ", Style::default().fg(FG_PRIMARY)));
                footer_items.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                footer_items.push(Span::styled(" Close  ", Style::default().fg(FG_PRIMARY)));
            } else {
                // Navigating filter buttons
                footer_items.push(Span::styled("[←→/Tab]", Style::default().fg(ACCENT)));
                footer_items.push(Span::styled(
                    " Switch Filter  ",
                    Style::default().fg(FG_PRIMARY),
                ));
                footer_items.push(Span::styled("[⏎/Space]", Style::default().fg(ACCENT)));
                footer_items.push(Span::styled(" Open  ", Style::default().fg(FG_PRIMARY)));
                footer_items.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
                footer_items.push(Span::styled(" Exit  ", Style::default().fg(FG_PRIMARY)));
            }
        }

        // Add "Load More" to footer if pagination is available
        if self.can_load_more() && self.focus == PanelFocus::Jobs {
            footer_items.push(Span::styled("[l]", Style::default().fg(ACCENT)));
            footer_items.push(Span::styled(
                " Load More  ",
                Style::default().fg(FG_PRIMARY),
            ));
        }
        // Add "Rerun Workflow" to footer if focused on workflows panel
        if self.focus == PanelFocus::Workflows {
            footer_items.push(Span::styled("[R]", Style::default().fg(ACCENT)));
            footer_items.push(Span::styled(
                " Rerun Workflow  ",
                Style::default().fg(FG_PRIMARY),
            ));
        }

        footer_items.push(Span::styled("[?]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Help  ", Style::default().fg(FG_PRIMARY)));
        footer_items.push(Span::styled("[Esc]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Back", Style::default().fg(FG_PRIMARY)));

        let footer = Paragraph::new(Line::from(footer_items)).alignment(Alignment::Center);

        f.render_widget(footer, area);
    }

    /// Get the currently selected job
    pub fn get_selected_job(&self) -> Option<&Job> {
        self.selected_job_index
            .and_then(|idx| self.get_filtered_jobs().get(idx).copied())
    }

    /// Calculate visible job numbers for prefetching based on viewport and selection
    pub fn get_visible_job_numbers(&self, _viewport_height: u16) -> Vec<u32> {
        const PREFETCH_AHEAD: usize = 3;
        const PREFETCH_BEHIND: usize = 3;

        let filtered_jobs = self.get_filtered_jobs();

        if filtered_jobs.is_empty() {
            return Vec::new();
        }

        // Get selected index (default to 0)
        let selected = self.selected_job_index.unwrap_or(0);

        // Calculate prefetch range with N jobs ahead/behind
        let start = selected.saturating_sub(PREFETCH_BEHIND);
        let end = (selected + PREFETCH_AHEAD + 1).min(filtered_jobs.len());

        // Extract job numbers from filtered list
        filtered_jobs[start..end]
            .iter()
            .map(|job| job.job_number)
            .collect()
    }

    /// Store fetched logs and show modal if copy was pending
    pub fn set_logs_for_job(&mut self, job_number: u32, logs: Vec<String>) {
        // Store in cache
        self.log_cache.insert(job_number, logs.clone());

        // Clear pending fetch
        self.pending_log_fetch = None;

        // If this was triggered by 'y', show the modal now
        if self.pending_copy_job == Some(job_number) {
            self.line_range_modal.show(logs.len());
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
    pub fn tick_powerline(&mut self) {
        self.powerline.tick();
    }

    /// Handle copying logs with a specified line range
    fn handle_copy_logs_range(&mut self, job_number: u32, range_str: &str) {
        if let Some(logs) = self.log_cache.get(&job_number) {
            match Self::parse_line_range(range_str, logs.len()) {
                Ok((start, end)) => {
                    let lines_to_copy = Self::extract_line_range(logs, start, end);

                    if lines_to_copy.is_empty() {
                        self.powerline.set_notification(
                            "No lines to copy".to_string(),
                            crate::ui::widgets::powerline::NotificationLevel::Error,
                            std::time::Duration::from_secs(2),
                        );
                        return;
                    }

                    match Self::copy_to_clipboard(&lines_to_copy.join("\n")) {
                        Ok(_) => {
                            let message = format!(
                                "Copied lines {:>5}-{:<5} ({:>4} lines)",
                                start,
                                end,
                                lines_to_copy.len()
                            );
                            self.powerline.set_notification(
                                message,
                                crate::ui::widgets::powerline::NotificationLevel::Success,
                                std::time::Duration::from_secs(2),
                            );
                        }
                        Err(err) => {
                            self.powerline.set_notification(
                                err,
                                crate::ui::widgets::powerline::NotificationLevel::Error,
                                std::time::Duration::from_secs(3),
                            );
                        }
                    }
                }
                Err(err) => {
                    self.powerline.set_notification(
                        format!("Invalid range: {}", err),
                        crate::ui::widgets::powerline::NotificationLevel::Error,
                        std::time::Duration::from_secs(3),
                    );
                }
            }
        }
    }

    /// Parse line range input with Vim-style syntax
    /// Supports: "1,1000", "1:1000", "100,$", "%", "1000"
    /// Returns (start_line, end_line) - both 1-indexed
    fn parse_line_range(input: &str, max_lines: usize) -> Result<(usize, usize), String> {
        let input = input.trim();

        if input.is_empty() {
            return Err("Empty input".to_string());
        }

        // Special case: "%" means all lines (1 to end)
        if input == "%" {
            return Ok((1, max_lines));
        }

        // Helper to parse a single position (number or "$")
        let parse_position = |s: &str| -> Result<usize, String> {
            let s = s.trim();
            if s == "$" {
                Ok(max_lines)
            } else {
                s.parse::<usize>()
                    .map_err(|_| format!("Invalid line number: {}", s))
            }
        };

        // Check for range format "start,end" or "start:end"
        if let Some(sep_pos) = input.find([',', ':']) {
            let (start_str, end_str) = input.split_at(sep_pos);
            let end_str = &end_str[1..]; // Skip separator

            let start = parse_position(start_str)?;
            let end = parse_position(end_str)?;

            if start < 1 {
                return Err("Start line must be >= 1".to_string());
            }
            if end < start {
                return Err("End line must be >= start line".to_string());
            }
            if start > max_lines {
                return Err(format!(
                    "Start line {} exceeds total lines {}",
                    start, max_lines
                ));
            }

            let end_clamped = end.min(max_lines);
            Ok((start, end_clamped))
        } else {
            // Single number means "1:N"
            let end = parse_position(input)?;

            if end < 1 {
                return Err("Line number must be >= 1".to_string());
            }

            let end_clamped = end.min(max_lines);
            Ok((1, end_clamped))
        }
    }

    /// Extract line range from logs (1-indexed input)
    fn extract_line_range(logs: &[String], start: usize, end: usize) -> Vec<String> {
        if logs.is_empty() || start < 1 || end < start {
            return Vec::new();
        }

        // Convert to 0-indexed
        let start_idx = start.saturating_sub(1);
        let end_idx = end.min(logs.len());

        if start_idx >= logs.len() {
            return Vec::new();
        }

        logs[start_idx..end_idx].to_vec()
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

        // Set status filter to "failed" via faceted search
        // Facet 0 is status: [All, success, running, failed, pending, blocked]
        // Index 3 is "failed", Facet 1 stays at index 0 (All durations)
        screen.faceted_search.restore_filter_state(&[3, 0]);

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

        // Test filtering by "running" status
        // Facet 0 status indices: [0=All, 1=success, 2=running, 3=failed, 4=pending, 5=blocked]
        screen.faceted_search.restore_filter_state(&[2, 0]);
        let filtered = screen.get_filtered_jobs();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status, "running");

        // Test filtering only failed jobs
        screen.faceted_search.restore_filter_state(&[3, 0]);
        let filtered = screen.get_filtered_jobs();
        assert_eq!(filtered.len(), 1);
        assert!(filtered
            .iter()
            .all(|job| job.status == "failed" || job.status == "error" || job.status == "failure"));
    }

    #[test]
    fn test_duration_filter() {
        let pipeline = create_test_pipeline();
        let mut screen = PipelineDetailScreen::new(pipeline);

        // Add test jobs with different durations
        use crate::api::models::Job;
        use chrono::Utc;
        let jobs = vec![
            Job {
                id: "job1".to_string(),
                name: "quick".to_string(),
                status: "success".to_string(),
                job_number: 1,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(30), // 30 seconds (quick)
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: "job2".to_string(),
                name: "short".to_string(),
                status: "success".to_string(),
                job_number: 2,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(180), // 3 minutes (short)
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
            Job {
                id: "job3".to_string(),
                name: "long".to_string(),
                status: "success".to_string(),
                job_number: 3,
                workflow_id: "wf1".to_string(),
                started_at: Some(Utc::now()),
                stopped_at: Some(Utc::now()),
                duration: Some(1200), // 20 minutes (long)
                executor: crate::api::models::ExecutorInfo {
                    executor_type: "docker".to_string(),
                },
            },
        ];
        screen.set_jobs(jobs);

        // Test "All durations" - should show all jobs
        // Facet 1 duration indices: [0=All, 1=Quick, 2=Short, 3=Medium, 4=Long, 5=Very Long]
        screen.faceted_search.restore_filter_state(&[0, 0]);
        let filtered = screen.get_filtered_jobs();
        assert_eq!(filtered.len(), 3);

        // Test "Quick (< 1min)" - should show only job1
        screen.faceted_search.restore_filter_state(&[0, 1]);
        let filtered = screen.get_filtered_jobs();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "quick");

        // Test "Short (1-5min)" - should show only job2
        screen.faceted_search.restore_filter_state(&[0, 2]);
        let filtered = screen.get_filtered_jobs();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "short");

        // Test "Long (15-30min)" - should show only job3
        screen.faceted_search.restore_filter_state(&[0, 4]);
        let filtered = screen.get_filtered_jobs();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "long");
    }
}
