/// Screen 1: Shows list of pipelines for the project
///
/// This is the first screen in the hierarchy: Pipeline → Workflow → Job
/// Users navigate from here to the workflows list screen by pressing Enter.
use crate::api::models::{mock_data, Pipeline};
use crate::theme::{
    get_status_color, get_status_icon, ACCENT, BG_PANEL, BORDER, BORDER_FOCUSED, FG_BRIGHT, FG_DIM,
    FG_PRIMARY,
};
use crate::ui::widgets::faceted_search::{Facet, FacetedSearchBar};
use crate::ui::widgets::spinner::Spinner;
use crate::ui::widgets::text_input::TextInput;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::collections::HashSet;

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
}

impl PipelineScreen {
    /// Create a new pipeline screen with mock data
    pub fn new() -> Self {
        let pipelines = mock_data::mock_pipelines();
        let filtered_pipelines = pipelines.clone();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

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
        let search_input = TextInput::new("Filter pipelines...");

        Self {
            pipelines,
            filtered_pipelines,
            list_state,
            selected_index: Some(0),
            loading: false,
            search_input,
            faceted_search,
            filter_active: false,
            search_focused: false,
            spinner: Spinner::new("Loading pipelines..."),
            refreshing: false,
        }
    }

    /// Set pipelines from external source (e.g., API)
    pub fn set_pipelines(&mut self, pipelines: Vec<Pipeline>) {
        self.pipelines = pipelines;

        // Update branch filter options dynamically
        self.update_branch_filter();

        self.apply_filters();
    }

    /// Update the branch filter facet with current unique branches
    fn update_branch_filter(&mut self) {
        let branches = self.get_unique_branches();
        let mut branch_options = vec!["All".to_string()];
        branch_options.extend(branches);

        // Update facet 1 (branch filter) with new options
        self.faceted_search.update_facet_options(1, branch_options);
    }

    /// Apply filters to the pipeline list
    pub fn apply_filters(&mut self) {
        use chrono::Utc;

        // Get filter values from faceted search bar
        let owner_filter = self
            .faceted_search
            .get_filter_value(0)
            .unwrap_or("All pipelines");
        let branch_filter = self.faceted_search.get_filter_value(1).unwrap_or("All");
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

                // Owner filter (All / Mine)
                let owner_match = match owner_filter {
                    "All pipelines" => true,
                    "Mine" => {
                        // Mock: filter by current user (placeholder logic)
                        // In a real app, this would check against the authenticated user
                        p.vcs.commit_author_name.contains("Alice")
                            || p.vcs.commit_author_name.contains("Bob")
                    }
                    _ => true,
                };

                // Branch filter - match exact branch
                let branch_match = if branch_filter == "All" {
                    true
                } else {
                    &p.vcs.branch == branch_filter
                };

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
                    &p.state == status_filter
                };

                text_match && owner_match && branch_match && date_match && status_match
            })
            .cloned()
            .collect();

        // Reset selection if needed
        if self.filtered_pipelines.is_empty() {
            self.selected_index = None;
            self.list_state.select(None);
        } else if self.selected_index.is_some() {
            let idx = self
                .selected_index
                .unwrap()
                .min(self.filtered_pipelines.len() - 1);
            self.selected_index = Some(idx);
            self.list_state.select(Some(idx));
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

        // Main layout: Header | Search Input | Filter Bar | List | Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with title
                Constraint::Length(3), // Search input
                Constraint::Length(3), // Filter bar
                Constraint::Min(0),    // Pipeline list (full width)
                Constraint::Length(1), // Footer
            ])
            .split(area);

        // Render header with title
        self.render_header(f, main_chunks[0]);

        // Render search input
        self.render_search_input(f, main_chunks[1]);

        // Render filter bar
        self.render_filter_bar(f, main_chunks[2]);

        // Render pipeline list (full width, multi-line items)
        self.render_pipeline_list(f, main_chunks[3]);

        // Render footer with actions
        self.render_footer(f, main_chunks[4]);
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
            .border_style(Style::default().fg(BORDER))
            .style(Style::default().bg(BG_PANEL));

        f.render_widget(block, area);
    }

    /// Render search input widget
    fn render_search_input(&mut self, f: &mut Frame, area: Rect) {
        // Set focus state on the text input widget based on search focus
        self.search_input.set_focused(self.search_focused);

        // Render the text input widget
        self.search_input.render(f, area);
    }

    /// Render filter bar using faceted search widget
    fn render_filter_bar(&mut self, f: &mut Frame, area: Rect) {
        self.faceted_search.render(f, area);
    }

    /// Render pipeline list with multi-line items (glim-style dense layout)
    fn render_pipeline_list(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER_FOCUSED))
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

            let selected_idx = self.selected_index;
            let items: Vec<ListItem> = self
                .filtered_pipelines
                .iter()
                .enumerate()
                .map(|(idx, pipeline)| {
                    let is_selected = selected_idx == Some(idx);
                    render_pipeline_multiline(pipeline, is_selected)
                })
                .collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(Style::default());

            f.render_stateful_widget(list, area, &mut self.list_state);
        } else {
            // Normal rendering
            let selected_idx = self.selected_index;
            let items: Vec<ListItem> = self
                .filtered_pipelines
                .iter()
                .enumerate()
                .map(|(idx, pipeline)| {
                    let is_selected = selected_idx == Some(idx);
                    render_pipeline_multiline(pipeline, is_selected)
                })
                .collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(Style::default());

            f.render_stateful_widget(list, area, &mut self.list_state);
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
            Span::styled(format!("{:<10} ", time_ago), Style::default().fg(FG_DIM)),
            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
            Span::styled(
                format!(" {}", pipeline.vcs.branch),
                Style::default().fg(ACCENT),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
            Span::styled(format!("{:<5} ", time_str), Style::default().fg(FG_DIM)),
            Span::styled(
                format!("Pipeline #{:<6} ", pipeline.number),
                Style::default().fg(FG_PRIMARY),
            ),
            Span::styled(format!("{:<10} ", time_ago), Style::default().fg(FG_DIM)),
            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
            Span::styled(
                format!(" {}", pipeline.vcs.branch),
                Style::default().fg(FG_DIM),
            ),
        ])
    };

    // Line 2: Indented commit message
    let line2 = if selected {
        Line::from(vec![Span::styled(
            format!("          {}", pipeline.vcs.commit_subject),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )])
    } else {
        Line::from(vec![Span::styled(
            format!("          {}", pipeline.vcs.commit_subject),
            Style::default().fg(FG_PRIMARY),
        )])
    };

    // Line 3: Indented summary (mock: 3 workflows • 24 jobs • 2 failed)
    let summary = format!(
        "          3 workflows • 24 jobs • {} ({})",
        pipeline.state, duration
    );
    let line3 = Line::from(vec![Span::styled(summary, Style::default().fg(FG_DIM))]);

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

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(
            truncate_string("this is a very long string", 10),
            "this is..."
        );
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
