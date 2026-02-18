/// Workflow screen implementation - displays workflow details, jobs list, and job logs
use crate::api::models::{Job, Pipeline, Workflow};
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_PANEL, BORDER, BORDER_FOCUSED, FG_BRIGHT,
    FG_DIM, FG_PRIMARY, RUNNING, SUCCESS, FAILED_TEXT,
};
use crate::ui::widgets::breadcrumb::render_breadcrumb;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Navigation action returned from input handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationAction {
    /// Go back to previous screen
    Back,
    /// Quit the application
    Quit,
    /// No navigation action
    None,
}

/// Workflow screen state and rendering
pub struct WorkflowScreen {
    /// Parent pipeline ID
    pub pipeline_id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Parent pipeline data
    pub pipeline: Pipeline,
    /// Selected workflow data
    pub workflow: Workflow,
    /// List of jobs in this workflow
    pub jobs: Vec<Job>,
    /// List state for job selection
    pub list_state: ListState,
    /// Selected job index
    pub selected_job_index: Option<usize>,
    /// Mock log lines for the selected job
    pub log_lines: Vec<String>,
    /// Log scroll position
    pub log_scroll: usize,
    /// Loading state
    pub loading: bool,
}

impl WorkflowScreen {
    /// Create a new workflow screen with mock data
    pub fn new(pipeline: Pipeline, workflow: Workflow) -> Self {
        let workflow_id = workflow.id.clone();
        let pipeline_id = pipeline.id.clone();

        // Load mock jobs
        let jobs = crate::api::models::mock_data::mock_jobs(&workflow_id);

        let mut screen = Self {
            pipeline_id,
            workflow_id,
            pipeline,
            workflow,
            jobs,
            list_state: ListState::default(),
            selected_job_index: None,
            log_lines: Vec::new(),
            log_scroll: 0,
            loading: false,
        };

        // Select first job by default
        if !screen.jobs.is_empty() {
            screen.list_state.select(Some(0));
            screen.selected_job_index = Some(0);
            screen.load_mock_logs();
        }

        screen
    }

    /// Render the workflow screen
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Main layout: header, breadcrumb, dag, body, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Length(1), // Breadcrumb + Status Summary
                Constraint::Length(3), // Workflow DAG
                Constraint::Min(10),   // Body (split into left/right)
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header
        self.render_header(f, chunks[0]);

        // Render breadcrumb and status summary
        self.render_breadcrumb_and_status(f, chunks[1]);

        // Render workflow DAG
        self.render_dag(f, chunks[2]);

        // Split body into left panel (jobs) and right panel (logs)
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(40), // Left panel (jobs list)
                Constraint::Min(40),    // Right panel (logs)
            ])
            .split(chunks[3]);

        // Render left panel (jobs)
        self.render_jobs_panel(f, body_chunks[0]);

        // Render right panel (logs)
        self.render_logs_panel(f, body_chunks[1]);

        // Render footer
        self.render_footer(f, chunks[4]);
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> NavigationAction {
        match (key.code, key.modifiers) {
            // Navigation
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                self.select_previous_job();
                NavigationAction::None
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.select_next_job();
                NavigationAction::None
            }
            // Log scrolling
            (KeyCode::PageUp, KeyModifiers::NONE) => {
                self.scroll_log_up();
                NavigationAction::None
            }
            (KeyCode::PageDown, KeyModifiers::NONE) => {
                self.scroll_log_down();
                NavigationAction::None
            }
            // Go back
            (KeyCode::Esc, KeyModifiers::NONE) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
                NavigationAction::Back
            }
            // Quit
            (KeyCode::Char('q'), KeyModifiers::NONE) => NavigationAction::Quit,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => NavigationAction::Quit,
            _ => NavigationAction::None,
        }
    }

    /// Move job selection down
    pub fn select_next_job(&mut self) {
        if self.jobs.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.jobs.len() - 1 {
                    i // Stay at last item
                } else {
                    i + 1
                }
            }
            None => 0,
        };

        self.list_state.select(Some(i));
        self.selected_job_index = Some(i);
        self.log_scroll = 0;
        self.load_mock_logs();
    }

    /// Move job selection up
    pub fn select_previous_job(&mut self) {
        if self.jobs.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };

        self.list_state.select(Some(i));
        self.selected_job_index = Some(i);
        self.log_scroll = 0;
        self.load_mock_logs();
    }

    /// Scroll logs down
    pub fn scroll_log_down(&mut self) {
        if self.log_lines.is_empty() {
            return;
        }

        let max_scroll = self.log_lines.len().saturating_sub(1);
        if self.log_scroll < max_scroll {
            self.log_scroll += 1;
        }
    }

    /// Scroll logs up
    pub fn scroll_log_up(&mut self) {
        if self.log_scroll > 0 {
            self.log_scroll -= 1;
        }
    }

    /// Load realistic mock log lines for the selected job
    pub fn load_mock_logs(&mut self) {
        let job = match self.selected_job_index {
            Some(idx) => &self.jobs[idx],
            None => return,
        };

        // Generate logs based on job status
        let mut logs = Vec::new();

        logs.push(format!("Job #{}: {}", job.job_number, job.name));
        logs.push(format!("Executor: {} ({})", job.executor.executor_type, "docker:latest"));
        logs.push("".to_string());
        logs.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        logs.push("".to_string());

        // Step 1: Spin up environment
        logs.push("$ Spinning up environment...".to_string());
        logs.push("✓ Pulling Docker image: cimg/base:2024.01".to_string());
        logs.push("✓ Starting container".to_string());
        logs.push("".to_string());

        // Step 2: Prepare environment
        logs.push("$ Preparing environment variables".to_string());
        logs.push("  CIRCLE_BRANCH=main".to_string());
        logs.push("  CIRCLE_BUILD_NUM=1234".to_string());
        logs.push("  CIRCLE_JOB={}".to_string().replace("{}", &job.name));
        logs.push("✓ Environment ready".to_string());
        logs.push("".to_string());

        // Step 3: Checkout code
        if job.name.contains("checkout") || job.job_number == 1 {
            logs.push("$ git clone --depth=1 https://github.com/acme/api-service".to_string());
            logs.push("  Cloning into 'api-service'...".to_string());
            logs.push("✓ Repository cloned (142 files)".to_string());
            logs.push("".to_string());
        }

        // Step 4: Job-specific operations
        match job.name.as_str() {
            name if name.contains("install") || name.contains("deps") => {
                logs.push("$ npm install".to_string());
                logs.push("  npm WARN deprecated request@2.88.2".to_string());
                logs.push("  added 847 packages, and audited 848 packages in 1m 24s".to_string());
                logs.push("✓ Dependencies installed (847 packages)".to_string());
                logs.push("".to_string());
            }
            name if name.contains("lint") => {
                logs.push("$ npm run lint".to_string());
                logs.push("  > eslint . --ext .js,.jsx,.ts,.tsx".to_string());
                logs.push("✓ No linting errors found".to_string());
                logs.push("".to_string());
            }
            name if name.contains("test") => {
                logs.push("$ npm run test".to_string());
                logs.push("  PASS  src/auth/login.test.ts".to_string());
                logs.push("    ✓ should authenticate valid user (142ms)".to_string());
                logs.push("    ✓ should reject invalid credentials (89ms)".to_string());
                logs.push("  PASS  src/api/webhooks.test.ts".to_string());
                logs.push("    ✓ should process webhook payload (67ms)".to_string());
                logs.push("    ✓ should validate signature (45ms)".to_string());
                logs.push("".to_string());
                logs.push("  Test Suites: 42 passed, 42 total".to_string());
                logs.push("  Tests:       287 passed, 287 total".to_string());
                logs.push("  Snapshots:   12 passed, 12 total".to_string());
                logs.push("  Time:        23.456s".to_string());

                // Add failure for failed tests
                if job.status == "failed" {
                    logs.push("".to_string());
                    logs.push("✗ Test suite failed".to_string());
                    logs.push("  FAIL  src/api/deploy.test.ts".to_string());
                    logs.push("    ✗ should deploy to production (1234ms)".to_string());
                    logs.push("      Error: Connection timeout".to_string());
                    logs.push("        at deploy.js:42:15".to_string());
                    logs.push("        at processTicksAndRejections (internal/process/task_queues.js:93:5)".to_string());
                    logs.push("".to_string());
                    logs.push("✗ Failed with exit code 1".to_string());
                } else {
                    logs.push("✓ All tests passed".to_string());
                }
                logs.push("".to_string());
            }
            name if name.contains("build") => {
                logs.push("$ npm run build".to_string());
                logs.push("  > webpack --mode production".to_string());
                logs.push("  asset main.js 287 KiB [emitted] [minimized] (name: main)".to_string());
                logs.push("  asset styles.css 42 KiB [emitted] [minimized]".to_string());
                logs.push("  webpack 5.89.0 compiled successfully in 12.3s".to_string());
                logs.push("✓ Build completed in 2m 15s".to_string());
                logs.push("".to_string());
            }
            name if name.contains("e2e") => {
                logs.push("$ npm run e2e".to_string());
                logs.push("  > cypress run".to_string());
                logs.push("  Running: login_spec.js".to_string());
                logs.push("    ✓ User can log in (2341ms)".to_string());
                logs.push("    ✓ User can log out (876ms)".to_string());
                logs.push("  Running: dashboard_spec.js".to_string());
                logs.push("    ✓ Dashboard loads correctly (1543ms)".to_string());
                logs.push("".to_string());
                logs.push("  (Tests passed: 12)".to_string());
                logs.push("  (Tests failed: 0)".to_string());
                logs.push("✓ E2E tests passed".to_string());
                logs.push("".to_string());
            }
            name if name.contains("security") => {
                logs.push("$ npm audit".to_string());
                logs.push("  found 0 vulnerabilities".to_string());
                logs.push("".to_string());
                logs.push("$ snyk test".to_string());
                logs.push("  ✓ Tested 847 dependencies for known issues".to_string());
                logs.push("  ✓ No vulnerable paths found".to_string());
                logs.push("".to_string());
            }
            name if name.contains("artifact") || name.contains("upload") => {
                logs.push("$ tar -czf artifacts.tar.gz dist/".to_string());
                logs.push("✓ Created artifacts.tar.gz (42.3 MB)".to_string());
                logs.push("".to_string());
                logs.push("$ aws s3 cp artifacts.tar.gz s3://circleci-artifacts/".to_string());
                logs.push("  upload: artifacts.tar.gz to s3://circleci-artifacts/build-1234/".to_string());
                logs.push("✓ Artifacts uploaded successfully".to_string());
                logs.push("".to_string());
            }
            _ => {
                logs.push("$ Running job steps...".to_string());
                logs.push("✓ Job completed successfully".to_string());
                logs.push("".to_string());
            }
        }

        // Step 5: Summary
        logs.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
        logs.push("".to_string());

        match job.status.as_str() {
            "success" => {
                logs.push("✓ Job completed successfully".to_string());
                logs.push(format!("  Duration: {}", job.duration_formatted()));
            }
            "failed" => {
                logs.push("✗ Job failed".to_string());
                logs.push("  See error details above".to_string());
                logs.push(format!("  Duration: {}", job.duration_formatted()));
            }
            "running" => {
                logs.push("● Job is currently running...".to_string());
                logs.push("  Please wait for completion".to_string());
            }
            _ => {
                logs.push(format!("○ Job status: {}", job.status));
            }
        }

        self.log_lines = logs;
    }

    /// Render header
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let title = format!(
            " Workflow: {} (Pipeline #{}) ",
            self.workflow.name, self.pipeline.number
        );
        let header = Paragraph::new(title)
            .style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD));
        f.render_widget(header, area);
    }

    /// Render breadcrumb and status summary
    fn render_breadcrumb_and_status(&self, f: &mut Frame, area: Rect) {
        let project = self
            .pipeline
            .project_slug
            .split('/')
            .last()
            .unwrap_or("project");
        let branch = &self.pipeline.vcs.branch;
        let workflow = &self.workflow.name;

        // Calculate status counts
        let mut passed = 0;
        let mut running = 0;
        let mut failed = 0;
        let mut pending = 0;

        for job in &self.jobs {
            match job.status.as_str() {
                "success" => passed += 1,
                "running" => running += 1,
                "failed" => failed += 1,
                _ => pending += 1,
            }
        }

        let breadcrumb_text = format!("{} › {} › {}", project, branch, workflow);
        let status_text = format!(
            "  ✓ {} passed  ● {} running  ✗ {} failed  ○ {} pending",
            passed, running, failed, pending
        );

        let line = Line::from(vec![
            Span::styled(breadcrumb_text, Style::default().fg(FG_DIM)),
            Span::styled(status_text, Style::default().fg(FG_PRIMARY)),
        ]);

        let paragraph = Paragraph::new(line);
        f.render_widget(paragraph, area);
    }

    /// Render workflow DAG
    fn render_dag(&self, f: &mut Frame, area: Rect) {
        // Simple text representation of workflow
        let dag_text = self.generate_dag_text();

        let block = Block::default()
            .title(" Workflow DAG ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER));

        let paragraph = Paragraph::new(dag_text).block(block);
        f.render_widget(paragraph, area);
    }

    /// Generate DAG text representation
    fn generate_dag_text(&self) -> String {
        // For now, create a simple linear flow from the first few jobs
        let mut dag_parts = Vec::new();

        for (i, job) in self.jobs.iter().take(5).enumerate() {
            let icon = get_status_icon(&job.status);
            let name = if job.name.len() > 12 {
                format!("{}...", &job.name[..9])
            } else {
                job.name.clone()
            };

            dag_parts.push(format!("{} {}", icon, name));

            if i < self.jobs.len().min(5) - 1 {
                dag_parts.push(" → ".to_string());
            }
        }

        if self.jobs.len() > 5 {
            dag_parts.push(format!(" ... (+{} more)", self.jobs.len() - 5));
        }

        dag_parts.join("")
    }

    /// Render jobs panel (left side)
    fn render_jobs_panel(&mut self, f: &mut Frame, area: Rect) {
        // Split into sections: search, filters, list, pagination, actions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Search box
                Constraint::Length(5), // Status filters
                Constraint::Min(8),    // Job list
                Constraint::Length(1), // Pagination info
                Constraint::Length(3), // Action buttons
            ])
            .split(area);

        // Render search box
        self.render_search_box(f, chunks[0]);

        // Render status filters
        self.render_status_filters(f, chunks[1]);

        // Render job list
        self.render_job_list(f, chunks[2]);

        // Render pagination info
        self.render_pagination_info(f, chunks[3]);

        // Render action buttons
        self.render_action_buttons(f, chunks[4]);
    }

    /// Render search box
    fn render_search_box(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Search Jobs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER));

        let text = Paragraph::new("Filter jobs...")
            .style(Style::default().fg(FG_DIM))
            .block(block);

        f.render_widget(text, area);
    }

    /// Render status filters
    fn render_status_filters(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Filters ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER));

        let filters = vec![
            "☑ Success",
            "☑ Running",
            "☑ Failed",
            "☑ Pending",
        ];

        let lines: Vec<Line> = filters
            .iter()
            .map(|f| Line::from(Span::styled(*f, Style::default().fg(FG_PRIMARY))))
            .collect();

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    /// Render job list
    fn render_job_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .jobs
            .iter()
            .map(|job| {
                let icon = get_status_icon(&job.status);
                let color = get_status_color(&job.status);
                let name = truncate_string(&job.name, 20);

                let line = Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::styled(name, Style::default().fg(FG_PRIMARY)),
                    Span::raw(" "),
                    Span::styled(
                        job.duration_formatted(),
                        Style::default().fg(FG_DIM),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect();

        let block = Block::default()
            .title(" Jobs ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED));

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(ACCENT)
                    .bg(BG_PANEL)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render pagination info
    fn render_pagination_info(&self, f: &mut Frame, area: Rect) {
        let info = format!("Showing {} jobs", self.jobs.len());
        let paragraph = Paragraph::new(info).style(Style::default().fg(FG_DIM));
        f.render_widget(paragraph, area);
    }

    /// Render action buttons
    fn render_action_buttons(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER));

        let buttons = Line::from(vec![
            Span::styled("[Rerun]", Style::default().fg(FG_DIM)),
            Span::raw("  "),
            Span::styled("[SSH]", Style::default().fg(FG_DIM)),
            Span::raw("  "),
            Span::styled("[Artifacts]", Style::default().fg(FG_DIM)),
        ]);

        let paragraph = Paragraph::new(buttons).block(block);
        f.render_widget(paragraph, area);
    }

    /// Render logs panel (right side)
    fn render_logs_panel(&mut self, f: &mut Frame, area: Rect) {
        let job_name = self.selected_job_index.map(|idx| self.jobs[idx].name.clone());

        let title = if let Some(name) = job_name {
            format!(" Job Logs: {} ", name)
        } else {
            " Job Logs ".to_string()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER));

        // Apply syntax highlighting to log lines
        let styled_lines: Vec<Line> = self
            .log_lines
            .iter()
            .skip(self.log_scroll)
            .map(|line| self.highlight_log_line(line))
            .collect();

        let paragraph = Paragraph::new(styled_lines).block(block);
        f.render_widget(paragraph, area);

        // Render scrollbar if needed
        if self.log_lines.len() > area.height.saturating_sub(2) as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(self.log_lines.len())
                .position(self.log_scroll);

            f.render_stateful_widget(
                scrollbar,
                area.inner(&ratatui::layout::Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut scrollbar_state,
            );
        }
    }

    /// Apply syntax highlighting to a log line
    fn highlight_log_line<'a>(&self, line: &'a str) -> Line<'a> {
        // Commands (lines starting with $)
        if line.trim_start().starts_with('$') {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
        }

        // Success lines
        if line.contains('✓') || line.to_lowercase().contains("success") {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(SUCCESS),
            ));
        }

        // Error lines
        if line.contains('✗') || line.to_lowercase().contains("error") || line.to_lowercase().contains("failed") {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(FAILED_TEXT),
            ));
        }

        // Running/in-progress lines
        if line.contains('●') || line.to_lowercase().contains("running") {
            return Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(RUNNING),
            ));
        }

        // Default
        Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(FG_PRIMARY),
        ))
    }

    /// Render footer with keyboard shortcuts
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer = Line::from(vec![
            Span::styled("↑↓", Style::default().fg(ACCENT)),
            Span::styled(" Navigate ", Style::default().fg(FG_DIM)),
            Span::styled("PgUp/PgDn", Style::default().fg(ACCENT)),
            Span::styled(" Scroll Logs ", Style::default().fg(FG_DIM)),
            Span::styled("Esc", Style::default().fg(ACCENT)),
            Span::styled(" Back ", Style::default().fg(FG_DIM)),
            Span::styled("q", Style::default().fg(ACCENT)),
            Span::styled(" Quit", Style::default().fg(FG_DIM)),
        ]);

        let paragraph = Paragraph::new(footer);
        f.render_widget(paragraph, area);
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
    use crate::api::models::{ExecutorInfo, TriggerInfo, VcsInfo};
    use chrono::Utc;

    #[test]
    fn test_workflow_screen_creation() {
        let pipeline = create_test_pipeline();
        let workflow = create_test_workflow();

        let screen = WorkflowScreen::new(pipeline, workflow);

        assert!(!screen.jobs.is_empty());
        assert_eq!(screen.selected_job_index, Some(0));
        assert!(!screen.log_lines.is_empty());
    }

    #[test]
    fn test_job_navigation() {
        let pipeline = create_test_pipeline();
        let workflow = create_test_workflow();
        let mut screen = WorkflowScreen::new(pipeline, workflow);

        let initial_index = screen.selected_job_index;

        screen.select_next_job();
        assert_eq!(screen.selected_job_index, Some(1));

        screen.select_previous_job();
        assert_eq!(screen.selected_job_index, initial_index);
    }

    #[test]
    fn test_log_scrolling() {
        let pipeline = create_test_pipeline();
        let workflow = create_test_workflow();
        let mut screen = WorkflowScreen::new(pipeline, workflow);

        assert_eq!(screen.log_scroll, 0);

        screen.scroll_log_down();
        assert_eq!(screen.log_scroll, 1);

        screen.scroll_log_up();
        assert_eq!(screen.log_scroll, 0);
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long string", 10), "this is...");
        assert_eq!(truncate_string("exact", 5), "exact");
    }

    fn create_test_pipeline() -> Pipeline {
        Pipeline {
            id: "test-pipe".to_string(),
            number: 123,
            state: "success".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            vcs: VcsInfo {
                branch: "main".to_string(),
                revision: "abc123".to_string(),
                commit_subject: "test commit".to_string(),
                commit_author_name: "tester".to_string(),
                commit_timestamp: Utc::now(),
            },
            trigger: TriggerInfo {
                trigger_type: "webhook".to_string(),
            },
            project_slug: "gh/test/project".to_string(),
        }
    }

    fn create_test_workflow() -> Workflow {
        Workflow {
            id: "test-wf".to_string(),
            name: "build-and-test".to_string(),
            status: "success".to_string(),
            created_at: Utc::now(),
            stopped_at: Some(Utc::now()),
            pipeline_id: "test-pipe".to_string(),
        }
    }
}
