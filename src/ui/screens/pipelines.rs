/// Screen 1: Shows list of pipelines for the project
///
/// This is the first screen in the hierarchy: Pipeline → Workflow → Job
/// Users navigate from here to the workflows list screen by pressing Enter.
use crate::api::models::{mock_data, Pipeline, Workflow};
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_PANEL, BORDER_FOCUSED, FG_BRIGHT, FG_DIM,
    FG_PRIMARY,
};
use crate::ui::utils::{truncate_string, truncate_string_left};
use crate::ui::widgets::faceted_search::{Facet, FacetedSearchBar};
use crate::ui::widgets::spinner::Spinner;
use crate::ui::widgets::text_input::TextInput;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use std::collections::{HashMap, HashSet};

/// Pipeline screen with dense list and filters
pub struct PipelineScreen {
    /// All pipelines (from mock data)
    pub pipelines: Vec<Pipeline>,
    /// Filtered pipelines (after applying filters)
    pub filtered_pipelines: Vec<Pipeline>,
    /// Table selection state
    pub table_state: TableState,
    /// Currently selected index (in filtered list)
    pub selected_index: Option<usize>,
    /// Loading state
    pub loading: bool,
    /// Text input widget for search/filtering
    pub search_input: TextInput,
    /// Faceted search bar for filtering
    pub faceted_search: FacetedSearchBar,
    /// Whether filter bar is active (for keyboard input routing)
    pub filter_active: bool,
    /// Whether search input is focused
    pub search_focused: bool,
    /// Spinner for loading state
    pub spinner: Spinner,
    /// Refreshing indicator
    pub refreshing: bool,
    /// Authenticated user login (for "Mine" filter)
    pub authenticated_user: Option<String>,
    /// Authenticated user full name (for "Mine" filter)
    pub authenticated_user_name: Option<String>,
    /// Cached workflows by pipeline ID
    pub pipeline_workflows: HashMap<String, Vec<Workflow>>,
    /// Loading state for workflows
    pub loading_workflows: bool,
}

impl PipelineScreen {
    /// Create a new pipeline screen with saved preferences
    pub fn with_preferences(
        prefs: &crate::preferences::PipelineFilterPrefs,
        authenticated_user: Option<String>,
        authenticated_user_name: Option<String>,
    ) -> Self {
        let mut screen = Self::new();
        screen.authenticated_user = authenticated_user;
        screen.authenticated_user_name = authenticated_user_name;

        // Apply saved filter selections
        screen.faceted_search.set_facet_selection(0, prefs.owner_index);
        screen.faceted_search.set_facet_selection(2, prefs.date_index);
        screen.faceted_search.set_facet_selection(3, prefs.status_index);

        // Apply saved branch (add to options if doesn't exist)
        if let Some(ref branch) = prefs.branch {
            // Add branch to options and select it (creates if doesn't exist)
            screen.faceted_search.add_and_select_option(1, branch.clone());
        }

        // Apply saved search text
        screen.search_input.set_value(prefs.search_text.clone());

        // Apply filters
        screen.apply_filters();

        screen
    }

    /// Create a new pipeline screen with mock data
    pub fn new() -> Self {
        let pipelines = mock_data::mock_pipelines();
        let filtered_pipelines = pipelines.clone();
        let mut table_state = TableState::default();
        table_state.select(Some(0)); // Select first row (no header offset needed)

        // Extract unique branches from pipelines
        let mut branches: Vec<String> = pipelines
            .iter()
            .map(|p| p.vcs.branch.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        branches.sort();

        // Build branch options: "All" + unique branches
        let mut branch_options = vec!["All".to_string()];
        branch_options.extend(branches);

        // Create faceted search bar with 4 facets (matching Python implementation)
        let facets = vec![
            // Facet 0: Owner filter (All / Mine)
            Facet::new(
                "☍",
                vec!["All pipelines".to_string(), "Mine".to_string()],
                0, // Default: All pipelines
            ),
            // Facet 1: Branch filter (All / Main / specific branches)
            Facet::new("⎇", branch_options.clone(), 0),
            // Facet 2: Date filter (matching Python implementation)
            Facet::new(
                "📅",
                vec![
                    "Last 24 hours".to_string(),
                    "Last 7 days".to_string(),
                    "Last 30 days".to_string(),
                    "Last 90 days".to_string(),
                    "All time".to_string(),
                ],
                4, // Default: All time (index 4)
            ),
            // Facet 3: Status filter
            Facet::new(
                "●",
                vec![
                    "All".to_string(),
                    "success".to_string(),
                    "failed".to_string(),
                    "running".to_string(),
                    "pending".to_string(),
                ],
                0,
            ),
        ];

        let faceted_search = FacetedSearchBar::new(facets);
        let search_input = TextInput::new("Filter pipelines...")
            .with_borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

        Self {
            pipelines,
            filtered_pipelines,
            table_state,
            selected_index: Some(0),
            loading: false,
            search_input,
            faceted_search,
            filter_active: false,
            search_focused: false,
            spinner: Spinner::new("Loading pipelines..."),
            refreshing: false,
            authenticated_user: None,
            authenticated_user_name: None,
            pipeline_workflows: HashMap::new(),
            loading_workflows: false,
        }
    }

    /// Set pipelines from external source (e.g., API)
    pub fn set_pipelines(&mut self, pipelines: Vec<Pipeline>) {
        self.pipelines = pipelines;

        // Update branch filter options dynamically
        self.update_branch_filter();

        self.apply_filters();
    }

    /// Set workflows for pipelines (called by app.rs)
    pub fn set_pipeline_workflows(&mut self, workflows: HashMap<String, Vec<Workflow>>) {
        self.pipeline_workflows = workflows;
        self.loading_workflows = false;
    }

    /// Update the branch filter facet with current unique branches
    ///
    /// NOTE: When "Mine" filter is active, only shows branches from current (filtered) pipelines
    fn update_branch_filter(&mut self) {
        let branches = self.get_unique_branches();
        let mut branch_options = vec!["All".to_string()];
        branch_options.extend(branches);

        // Save current selection before updating options
        let current_selection = self.faceted_search.get_filter_value(1).map(|s| s.to_string());

        // Update facet 1 (branch filter) with new options
        self.faceted_search.update_facet_options(1, branch_options);

        // Restore selection if it still exists in new options
        if let Some(selected_branch) = current_selection {
            self.faceted_search.set_facet_selection_by_value(1, &selected_branch);
        }
    }

    /// Get current filter preferences for saving
    pub fn get_filter_preferences(&self) -> crate::preferences::PipelineFilterPrefs {
        use crate::preferences::PipelineFilterPrefs;

        let branch = self.faceted_search.get_filter_value(1);
        let branch_opt = if branch == Some("All") {
            None
        } else {
            branch.map(|s| s.to_string())
        };

        PipelineFilterPrefs {
            owner_index: self.faceted_search.get_facet_selection(0),
            branch: branch_opt,
            date_index: self.faceted_search.get_facet_selection(2),
            status_index: self.faceted_search.get_facet_selection(3),
            search_text: self.search_input.value().to_string(),
        }
    }

    /// Calculate status summary for pipelines (like "✓ 10 ● 2 ✗ 3")
    fn calculate_status_summary(&self) -> String {
        let mut success = 0;
        let mut failed = 0;
        let mut running = 0;

        for pipeline in &self.filtered_pipelines {
            match pipeline.state.as_str() {
                "success" => success += 1,
                "failed" | "error" => failed += 1,
                "running" => running += 1,
                _ => {}
            }
        }

        let mut parts = Vec::new();

        if success > 0 {
            parts.push(format!("✓ {}", success));
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
            parts.join(" ")
        }
    }

    /// Apply filters to the pipeline list (client-side only)
    ///
    /// NOTE: Owner ("Mine") and Branch filters are applied server-side by the API.
    /// This method only filters by text search, date range, and status.
    pub fn apply_filters(&mut self) {
        use chrono::Utc;

        // Get filter values from faceted search bar
        let date_filter = self
            .faceted_search
            .get_filter_value(2)
            .unwrap_or("All time");
        let status_filter = self.faceted_search.get_filter_value(3).unwrap_or("All");

        // Get text search value
        let search_text = self.search_input.value().to_lowercase();

        self.filtered_pipelines = self
            .pipelines
            .iter()
            .filter(|p| {
                // Text search filter - case-insensitive search in pipeline #, branch, commit message, and author
                let text_match = if search_text.is_empty() {
                    true
                } else {
                    let pipeline_num = format!("{}", p.number);
                    pipeline_num.contains(&search_text)
                        || p.vcs.branch.to_lowercase().contains(&search_text)
                        || p.vcs.commit_subject.to_lowercase().contains(&search_text)
                        || p.vcs.commit_author_name.to_lowercase().contains(&search_text)
                };

                // NOTE: Owner and Branch filters are applied server-side by API

                // Date filter - check pipeline created_at against cutoff
                let date_match = match date_filter {
                    "All time" => true,
                    "Last 24 hours" => {
                        let cutoff = Utc::now() - chrono::Duration::hours(24);
                        p.created_at >= cutoff
                    }
                    "Last 7 days" => {
                        let cutoff = Utc::now() - chrono::Duration::days(7);
                        p.created_at >= cutoff
                    }
                    "Last 30 days" => {
                        let cutoff = Utc::now() - chrono::Duration::days(30);
                        p.created_at >= cutoff
                    }
                    "Last 90 days" => {
                        let cutoff = Utc::now() - chrono::Duration::days(90);
                        p.created_at >= cutoff
                    }
                    _ => true,
                };

                // Status filter
                let status_match = if status_filter == "All" {
                    true
                } else {
                    p.state == status_filter
                };

                text_match && date_match && status_match
            })
            .cloned()
            .collect();

        // Reset selection if needed
        if self.filtered_pipelines.is_empty() {
            self.selected_index = None;
            self.table_state.select(None);
        } else if self.selected_index.is_some() {
            let idx = self
                .selected_index
                .unwrap()
                .min(self.filtered_pipelines.len() - 1);
            self.selected_index = Some(idx);
            self.table_state.select(Some(idx));
        }
    }

    /// Extract unique branches from pipelines
    pub fn get_unique_branches(&self) -> Vec<String> {
        let mut branches: Vec<String> = self
            .pipelines
            .iter()
            .map(|p| p.vcs.branch.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        branches.sort();
        branches
    }

    /// Render the pipeline screen
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Reset refreshing indicator after first render
        if self.refreshing {
            self.refreshing = false;
        }

        // Main layout: Header | Filters Panel | List | Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with title
                Constraint::Length(7), // Filters panel (search + spacing + filter buttons + borders)
                Constraint::Min(0),    // Pipeline list (full width)
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header with title
        self.render_header(f, main_chunks[0]);

        // Render unified filters panel
        self.render_filters_panel(f, main_chunks[1]);

        // Render pipeline list (full width, multi-line items)
        self.render_pipeline_list(f, main_chunks[2]);

        // Render footer with actions
        self.render_footer(f, main_chunks[3]);

        // Render dropdown LAST so it overlays everything
        if self.faceted_search.is_active() {
            self.faceted_search.render_dropdown_only(f, main_chunks[1]);
        }
    }

    /// Render the header with title
    fn render_header(&self, f: &mut Frame, area: Rect) {
        let project_slug = self
            .pipelines
            .first()
            .map(|p| p.project_slug.as_str())
            .unwrap_or("gh/acme/api-service");

        // Build title with filter count if filters are active
        let title = if self.faceted_search.is_filtered() {
            let filter_count = self.faceted_search.get_active_filter_count();
            format!(
                " CircleCI Pipelines - {} ({} filter{} active) ",
                project_slug,
                filter_count,
                if filter_count == 1 { "" } else { "s" }
            )
        } else {
            format!(" CircleCI Pipelines - {} ", project_slug)
        };

        let block = Block::default()
            .title(title)
            .title_style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .style(Style::default().bg(BG_PANEL));

        f.render_widget(block, area);
    }

    /// Render unified filters panel containing search input and filter buttons
    fn render_filters_panel(&mut self, f: &mut Frame, area: Rect) {
        // Determine border style based on focus
        let border_style = if self.search_focused || self.filter_active {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ACCENT)
        };

        // Create bordered block for entire filters panel
        let block = Block::default()
            .title(" FILTERS ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .style(Style::default().bg(BG_PANEL));

        let inner = block.inner(area);
        f.render_widget(block, area);

        // Split inner area into search and filter sections
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Search input (1 line, no borders)
                Constraint::Length(1), // Small spacing
                Constraint::Length(2), // Filter buttons (2 lines to ensure visibility)
            ])
            .split(inner);

        // Render search input without borders
        self.search_input.set_focused(self.search_focused);
        self.search_input.render_plain(f, chunks[0]);

        // Render filter buttons without borders
        self.faceted_search.render_filter_bar_only(f, chunks[2]);
    }

    /// Render pipeline list with multi-line items (glim-style dense layout)
    fn render_pipeline_list(&mut self, f: &mut Frame, area: Rect) {
        // Calculate status summary for title
        let status_summary = self.calculate_status_summary();
        let title = if status_summary.is_empty() {
            " PIPELINES ".to_string()
        } else {
            format!(" PIPELINES {} ", status_summary)
        };

        // Determine border style - bold when list is focused (not in search/filter mode)
        let border_style = if !self.search_focused && !self.filter_active {
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ACCENT)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style)
            .style(Style::default().bg(BG_PANEL));

        // Check if loading or empty
        if self.loading {
            // Show loading spinner with elapsed time and cancel hint
            self.spinner.tick();
            self.spinner
                .set_message("Loading pipelines from CircleCI...");
            let _inner = block.inner(area);

            let block_with_hint = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_FOCUSED))
                .style(Style::default().bg(BG_PANEL))
                .title(" Loading... ")
                .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));

            let inner = block_with_hint.inner(area);
            f.render_widget(block_with_hint, area);

            // Create spinner display with hint
            let spinner_lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled(
                        format!("{} ", self.spinner.current_frame()),
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "Loading pipelines from CircleCI...",
                        Style::default().fg(FG_PRIMARY),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Press Esc to cancel",
                    Style::default().fg(FG_DIM),
                )),
            ];

            let spinner_widget = Paragraph::new(spinner_lines).alignment(Alignment::Center);
            f.render_widget(spinner_widget, inner);
        } else if self.filtered_pipelines.is_empty() {
            // Show empty state message with ASCII art
            let inner = block.inner(area);
            f.render_widget(block, area);

            let empty_message = Paragraph::new(vec![
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(
                    "    (╯°□°)╯︵ ┻━┻",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "No pipelines found",
                    Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "This could mean:",
                    Style::default().fg(FG_DIM),
                )),
                Line::from(Span::styled(
                    "  • No pipelines match your filters",
                    Style::default().fg(FG_DIM),
                )),
                Line::from(Span::styled(
                    "  • Project hasn't run any pipelines yet",
                    Style::default().fg(FG_DIM),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default().fg(FG_DIM)),
                    Span::styled(
                        "'r'",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to refresh or ", Style::default().fg(FG_DIM)),
                    Span::styled(
                        "'Esc'",
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(" to clear filters", Style::default().fg(FG_DIM)),
                ]),
            ])
            .alignment(Alignment::Center);

            f.render_widget(empty_message, inner);
        } else if self.refreshing {
            // Show refreshing indicator briefly
            let title = format!(" {} Refreshing... ", self.spinner.current_frame());
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_FOCUSED))
                .style(Style::default().bg(BG_PANEL));

            // Define column widths: fixed sizes for STATUS/STAGES/DURATION, WORKFLOW fills remaining space
            let widths = [
                Constraint::Length(12),   // STATUS: icon + time (fixed)
                Constraint::Fill(1),      // WORKFLOW: expand to fill available space
                Constraint::Length(8),    // STAGES: stage icons (compact)
                Constraint::Length(10),   // DURATION: time display (compact)
            ];

            // Create header row
            let header = Row::new(vec![
                Cell::from(Span::styled(
                    "STATUS",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "WORKFLOW",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "STAGES",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "DURATION",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
            ])
            .height(1);

            // Map filtered pipelines to rows
            let selected_idx = self.selected_index;
            let rows: Vec<Row> = self
                .filtered_pipelines
                .iter()
                .enumerate()
                .map(|(idx, pipeline)| {
                    let is_selected = selected_idx == Some(idx);
                    let workflows = self.pipeline_workflows.get(&pipeline.id);
                    create_pipeline_row(pipeline, workflows, is_selected)
                })
                .collect();

            // Build and render table
            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .highlight_style(Style::default());

            f.render_stateful_widget(table, area, &mut self.table_state);
        } else {
            // Normal rendering
            // Define column widths: fixed sizes for STATUS/STAGES/DURATION, WORKFLOW fills remaining space
            let widths = [
                Constraint::Length(12),   // STATUS: icon + time (fixed)
                Constraint::Fill(1),      // WORKFLOW: expand to fill available space
                Constraint::Length(8),    // STAGES: stage icons (compact)
                Constraint::Length(10),   // DURATION: time display (compact)
            ];

            // Create header row
            let header = Row::new(vec![
                Cell::from(Span::styled(
                    "STATUS",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "WORKFLOW",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "STAGES",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "DURATION",
                    Style::default().fg(FG_DIM).add_modifier(Modifier::BOLD),
                )),
            ])
            .height(1);

            // Map filtered pipelines to rows
            let selected_idx = self.selected_index;
            let rows: Vec<Row> = self
                .filtered_pipelines
                .iter()
                .enumerate()
                .map(|(idx, pipeline)| {
                    let is_selected = selected_idx == Some(idx);
                    let workflows = self.pipeline_workflows.get(&pipeline.id);
                    create_pipeline_row(pipeline, workflows, is_selected)
                })
                .collect();

            // Build and render table
            let table = Table::new(rows, widths)
                .header(header)
                .block(block)
                .highlight_style(Style::default());

            f.render_stateful_widget(table, area, &mut self.table_state);
        }
    }

    /// Render footer with actions
    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer = if self.search_focused {
            // Show search-specific shortcuts when search input is focused
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "[Type]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Filter Text  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[Tab]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" To Filter Buttons  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[Esc]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Clear/Exit  ", Style::default().fg(FG_PRIMARY)),
            ]))
        } else if self.filter_active {
            // Show filter-specific shortcuts when filter is active
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "[←→/Tab]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Switch Filter  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[↑↓]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Navigate  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[⏎/Space]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Select  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[Shift+Tab]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" To Search  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[Esc]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Exit Filter", Style::default().fg(FG_PRIMARY)),
            ]))
        } else {
            // Show normal shortcuts
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "[↑↓]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Nav  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[⏎]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Open  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[/]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Search  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[f]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Filters  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[r]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Refresh  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[Esc]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Clear Filters  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[?]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Help  ", Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    "[q]",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" Quit", Style::default().fg(FG_PRIMARY)),
            ]))
        };

        f.render_widget(footer.alignment(Alignment::Center), area);
    }

    /// Handle keyboard input
    ///
    /// Returns true if the user wants to open the selected pipeline
    pub fn handle_input(&mut self, key: KeyEvent) -> bool {
        // If search input is focused, handle text input
        if self.search_focused {
            match key.code {
                KeyCode::Esc => {
                    // Exit search mode or clear if already empty
                    if self.search_input.is_empty() {
                        self.search_focused = false;
                    } else {
                        self.search_input.clear();
                        self.apply_filters();
                    }
                    false
                }
                KeyCode::Tab => {
                    // Move focus to faceted search buttons
                    self.search_focused = false;
                    self.filter_active = true;
                    false
                }
                _ => {
                    // Let TextInput handle the key
                    let handled = self.search_input.handle_key(key.code);
                    if handled {
                        // If search text changed, reapply filters
                        self.apply_filters();
                    }
                    false
                }
            }
        } else if self.filter_active {
            // If filter is active, delegate to faceted search widget
            match key.code {
                KeyCode::Esc => {
                    // Exit filter mode
                    self.filter_active = false;
                    // Apply filters when exiting
                    self.apply_filters();
                    false
                }
                KeyCode::BackTab => {
                    // Shift+Tab - return to text input
                    self.filter_active = false;
                    self.search_focused = true;
                    false
                }
                _ => {
                    // Let faceted search handle the key
                    let handled = self.faceted_search.handle_key(key.code);
                    if handled {
                        // If faceted search handled it, reapply filters
                        self.apply_filters();
                    }
                    false
                }
            }
        } else {
            // Normal navigation mode
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
                    self.refreshing = true;
                    self.spinner.set_message("Refreshing...");
                    self.pipelines = mock_data::mock_pipelines();
                    self.apply_filters();
                    // Note: In a real app, this would be async and we'd set refreshing to false
                    // after the API call completes. For now, it will be reset on next render.
                    false
                }
                KeyCode::Char('/') => {
                    // Activate search input focus
                    self.search_focused = true;
                    false
                }
                KeyCode::Char('f') => {
                    // Activate filter buttons focus
                    self.filter_active = true;
                    false
                }
                KeyCode::Esc => {
                    // Reset all filters including search input
                    self.faceted_search.reset_filters();
                    self.search_input.clear();
                    self.apply_filters();
                    false
                }
                _ => false,
            }
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
        self.table_state.select(Some(i));
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
        self.table_state.select(Some(i));
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

/// Create a pipeline row for Table widget (2 lines per row)
/// Returns a Row with 4 cells: STATUS | WORKFLOW | STAGES | DURATION
fn create_pipeline_row<'a>(
    pipeline: &'a Pipeline,
    workflows: Option<&'a Vec<Workflow>>,
    selected: bool,
) -> Row<'a> {
    let icon = get_status_icon(&pipeline.state);
    let status_col = get_status_color(&pipeline.state);

    // Use relative time only (like GitHub/GitLab)
    let time_str = format_time_ago(&pipeline.created_at);

    // Calculate duration (from created to updated)
    let duration = format_duration(pipeline.created_at, pipeline.updated_at);

    // Generate stage icons from workflow statuses
    let stage_icons = if let Some(wfs) = workflows {
        let icons: String = wfs
            .iter()
            .take(5) // First 5 workflows only
            .map(|w| get_status_icon(&w.status))
            .collect();

        // Add overflow indicator if more than 5 workflows
        if wfs.len() > 5 {
            format!("{}…", icons)
        } else if icons.is_empty() {
            "----".to_string() // No workflows
        } else {
            icons
        }
    } else {
        "····".to_string() // Loading state
    };

    // Extract first 7 chars of commit SHA
    let sha = if pipeline.vcs.revision.len() >= 7 {
        &pipeline.vcs.revision[..7]
    } else {
        &pipeline.vcs.revision
    };

    // Build tag string if trigger type is scheduled
    let tag_str = if pipeline.trigger.trigger_type == "scheduled" {
        " 🏷 scheduled"
    } else {
        ""
    };

    // Cell 1: STATUS (icon + time on line 1, empty on line 2)
    let status_line1 = if selected {
        Line::from(vec![
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(status_col).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                time_str,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
            Span::styled(time_str, Style::default().fg(FG_DIM)),
        ])
    };
    let status_line2 = Line::from("          "); // 10 spaces to match column width
    let status_cell = Cell::from(Text::from(vec![status_line1, status_line2]));

    // Cell 2: WORKFLOW (commit subject on line 1, metadata on line 2)
    let workflow_line1 = if selected {
        Line::from(Span::styled(
            &pipeline.vcs.commit_subject,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(
            &pipeline.vcs.commit_subject,
            Style::default().fg(FG_PRIMARY),
        ))
    };

    // Build metadata line with consistent separators: ⎇ branch  ∙  sha  ∙  @author  ∙  🏷 scheduled  ∙  #number
    let mut metadata_parts = vec![
        format!("⎇  {}", &pipeline.vcs.branch),
        sha.to_string(),
        format!("@{}", &pipeline.vcs.commit_author_name),
    ];

    if !tag_str.is_empty() {
        metadata_parts.push(tag_str.trim().to_string());
    }

    metadata_parts.push(format!("#{}", pipeline.number));

    let metadata_text = metadata_parts.join("  ∙  ");

    let workflow_line2 = if selected {
        Line::from(Span::styled(
            metadata_text,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(
            metadata_text,
            Style::default().fg(FG_DIM),
        ))
    };
    let workflow_cell = Cell::from(Text::from(vec![workflow_line1, workflow_line2]));

    // Cell 3: STAGES (stage icons on line 1, empty on line 2)
    let stages_line1 = if selected {
        Line::from(Span::styled(
            stage_icons,
            Style::default().fg(status_col).add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::styled(stage_icons, Style::default().fg(status_col)))
    };
    let stages_line2 = Line::from("       "); // 7 spaces to match column width
    let stages_cell = Cell::from(Text::from(vec![stages_line1, stages_line2]));

    // Cell 4: DURATION (duration on line 1, empty on line 2)
    let duration_line1 = Line::from(Span::styled(duration, Style::default().fg(FG_DIM)));
    let duration_line2 = Line::from("        "); // 8 spaces to match column width
    let duration_cell = Cell::from(Text::from(vec![duration_line1, duration_line2]));

    Row::new(vec![status_cell, workflow_cell, stages_cell, duration_cell]).height(2)
}

/// Format timestamp with date context for clarity
/// Examples: "Today 09:49", "Yesterday 17:49", "Mon 16:00", "Mar 1"
fn format_timestamp_with_date(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    use chrono::{Local, Timelike, Datelike};

    // Convert to local time for display
    let local_time = timestamp.with_timezone(&Local);
    let now = Local::now();

    // Calculate days difference
    let days_diff = (now.date_naive() - local_time.date_naive()).num_days();

    match days_diff {
        0 => {
            // Today: show "Today HH:MM"
            format!("Tdy {}", local_time.format("%H:%M"))
        }
        1 => {
            // Yesterday: show "Yesterday HH:MM"
            format!("Ydy {}", local_time.format("%H:%M"))
        }
        2..=6 => {
            // This week: show "Mon HH:MM"
            format!("{}", local_time.format("%a %H:%M"))
        }
        _ => {
            // Older: show "Mar 1" or "Jan 15"
            format!("{}", local_time.format("%b %-d"))
        }
    }
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
fn format_duration(
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> String {
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

}
