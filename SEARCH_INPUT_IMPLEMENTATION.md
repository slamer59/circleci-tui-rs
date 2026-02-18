# Search/Filter Text Input Implementation Guide

## Overview

This document provides a complete implementation plan for adding text search input to both the PipelineScreen and PipelineDetailScreen using the new `TextInput` widget.

## Files Created

### 1. `/src/ui/widgets/text_input.rs` ✅ Created

A simple, focused text input widget with:
- Text entry and editing (char, backspace, delete)
- Cursor movement (left, right, home, end)
- Focus management
- Placeholder text support
- Visual feedback (focused border, cursor indicator)

## Required Modifications

### 1. PipelineScreen (`src/ui/screens/pipelines.rs`)

#### Add text_input to imports:
```rust
use crate::ui::widgets::text_input::TextInput;
```

#### Add field to PipelineScreen struct:
```rust
pub struct PipelineScreen {
    // ... existing fields ...

    /// Text input for searching pipelines
    pub search_input: TextInput,

    /// Whether search input is focused
    pub search_input_focused: bool,
}
```

#### Update `new()` method:
```rust
pub fn new() -> Self {
    // ... existing code ...

    Self {
        // ... existing fields ...
        search_input: TextInput::new("Filter pipelines..."),
        search_input_focused: false,
    }
}
```

#### Update `apply_filters()` method to use text input:
```rust
pub fn apply_filters(&mut self) {
    // Get text search from input widget
    let text_search = self.search_input.value();

    // Get other filter values from faceted search bar
    let branch_filter = self.faceted_search.get_filter_value(0).unwrap_or("All");
    let date_filter = self.faceted_search.get_filter_value(1).unwrap_or("Any time");
    let status_filter = self.faceted_search.get_filter_value(2).unwrap_or("All");

    self.filtered_pipelines = self.pipelines
        .iter()
        .filter(|p| {
            // Text search - search in pipeline number, branch, commit message, and author
            let text_match = if text_search.is_empty() {
                true
            } else {
                let search_lower = text_search.to_lowercase();
                p.number.to_string().contains(&search_lower)
                    || p.vcs.branch.to_lowercase().contains(&search_lower)
                    || p.vcs.commit_subject.to_lowercase().contains(&search_lower)
                    || p.vcs.commit_author_name.to_lowercase().contains(&search_lower)
            };

            // ... rest of filter logic ...

            text_match && branch_match && date_match && status_match
        })
        .cloned()
        .collect();

    // ... rest of method ...
}
```

#### Update faceted search creation (remove text input facet, keep only dropdowns):
```rust
// Create faceted search bar with 3 dropdown facets
let facets = vec![
    // Facet 0: Branch filter
    Facet::new("⎇", branch_options.clone(), 0),
    // Facet 1: Date filter
    Facet::new(
        "📅",
        vec![
            "Any time".to_string(),
            "Last 24 hours".to_string(),
            "Last week".to_string(),
        ],
        0,
    ),
    // Facet 2: Status filter
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
```

#### Update `render()` method layout:
```rust
let main_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3), // Header with title
        Constraint::Length(3), // Search input
        Constraint::Length(3), // Filter bar
        Constraint::Min(0),    // Pipeline list
        Constraint::Length(1), // Footer
    ])
    .split(area);

// Render header
self.render_header(f, main_chunks[0]);

// Render search input
self.search_input.render(f, main_chunks[1]);

// Render filter bar
self.faceted_search.render(f, main_chunks[2]);

// Render pipeline list
self.render_pipeline_list(f, main_chunks[3]);

// Render footer
self.render_footer(f, main_chunks[4]);
```

#### Update `handle_input()` method:
```rust
pub fn handle_input(&mut self, key: KeyEvent) -> bool {
    // Handle '/' key to focus search input (when not already focused)
    if !self.search_input_focused && !self.filter_active && key.code == KeyCode::Char('/') {
        self.search_input_focused = true;
        self.search_input.set_focused(true);
        return false;
    }

    // If search input is focused, delegate to it
    if self.search_input_focused {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                // Exit search input mode
                self.search_input_focused = false;
                self.search_input.set_focused(false);
                // Apply filters on exit
                self.apply_filters();
                return false;
            }
            _ => {
                // Let search input handle the key
                let handled = self.search_input.handle_key(key.code);
                if handled {
                    // Reapply filters in real-time as user types
                    self.apply_filters();
                }
                return false;
            }
        }
    }

    // If filter is active, delegate to faceted search widget
    if self.filter_active {
        // ... existing filter handling code ...
    } else {
        // Normal navigation mode
        match key.code {
            // ... existing navigation code ...
            KeyCode::Char('/') => {
                // Note: This is already handled above, but kept for completeness
                false
            }
            KeyCode::Char('f') => {
                // Activate filter mode
                self.filter_active = true;
                false
            }
            // ... rest of navigation code ...
        }
    }
}
```

#### Update footer to show search shortcut:
```rust
fn render_footer(&self, f: &mut Frame, area: Rect) {
    let footer = if self.search_input_focused {
        // Show search-specific shortcuts when search is active
        Paragraph::new(Line::from(vec![
            Span::styled("[Type]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Search  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[⏎/Esc]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Exit Search", Style::default().fg(FG_PRIMARY)),
        ]))
    } else if self.filter_active {
        // Show filter-specific shortcuts when filter is active
        Paragraph::new(Line::from(vec![
            Span::styled("[←→]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Switch Filter  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[↑↓]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[⏎]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Select  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[Esc]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Exit Filter", Style::default().fg(FG_PRIMARY)),
        ]))
    } else {
        // Show normal shortcuts
        Paragraph::new(Line::from(vec![
            Span::styled("[↑↓]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Nav  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[⏎]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Open  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[/]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Search  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[f]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Filter  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[r]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Refresh  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[?]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Help  ", Style::default().fg(FG_PRIMARY)),
            Span::styled("[q]", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" Quit", Style::default().fg(FG_PRIMARY)),
        ]))
    };

    f.render_widget(footer.alignment(Alignment::Center), area);
}
```

### 2. PipelineDetailScreen (`src/ui/screens/pipeline_detail.rs`)

#### Add text_input to imports:
```rust
use crate::ui::widgets::text_input::TextInput;
```

#### Add field to PipelineDetailScreen struct:
```rust
pub struct PipelineDetailScreen {
    // ... existing fields ...

    /// Text input for searching jobs
    pub search_input: TextInput,

    /// Whether search input is focused
    pub search_input_focused: bool,

    // Remove: pub job_filter: String,  // No longer needed
}
```

#### Update `new()` method:
```rust
pub fn new(pipeline: Pipeline) -> Self {
    // ... existing code ...

    Self {
        // ... existing fields ...
        search_input: TextInput::new("Filter jobs..."),
        search_input_focused: false,
        // Remove: job_filter: String::new(),
    }
}
```

#### Update `get_filtered_jobs()` method:
```rust
fn get_filtered_jobs(&self) -> Vec<&Job> {
    self.jobs
        .iter()
        .filter(|job| {
            // Apply text filter using search input
            let matches_filter = if self.search_input.is_empty() {
                true
            } else {
                job.name
                    .to_lowercase()
                    .contains(&self.search_input.value().to_lowercase())
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
```

#### Update `handle_input()` method:
```rust
pub fn handle_input(&mut self, key: KeyEvent) -> PipelineDetailAction {
    // Handle '/' key to focus search input (when not in filter mode)
    if !self.search_input_focused && self.focus != PanelFocus::Filters && key.code == KeyCode::Char('/') {
        self.search_input_focused = true;
        self.search_input.set_focused(true);
        return PipelineDetailAction::None;
    }

    // If search input is focused, delegate to it
    if self.search_input_focused {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                // Exit search input mode
                self.search_input_focused = false;
                self.search_input.set_focused(false);
                // Reset job selection after filtering
                let filtered_jobs = self.get_filtered_jobs();
                if !filtered_jobs.is_empty() {
                    self.selected_job_index = Some(0);
                    self.job_list_state.select(Some(0));
                } else {
                    self.selected_job_index = None;
                    self.job_list_state.select(None);
                }
                return PipelineDetailAction::None;
            }
            _ => {
                // Let search input handle the key
                self.search_input.handle_key(key.code);
                // Real-time filtering as user types
                let filtered_jobs = self.get_filtered_jobs();
                if !filtered_jobs.is_empty() {
                    self.selected_job_index = Some(0);
                    self.job_list_state.select(Some(0));
                } else {
                    self.selected_job_index = None;
                    self.job_list_state.select(None);
                }
                return PipelineDetailAction::None;
            }
        }
    }

    // Handle filter mode input
    if self.focus == PanelFocus::Filters {
        return self.handle_filter_input(key);
    }

    // ... rest of existing input handling ...
}
```

#### Update `render_jobs_panel()` layout:
```rust
fn render_jobs_panel(&mut self, f: &mut Frame, area: Rect) {
    // Split into search input, status filters, and job list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input
            Constraint::Length(1), // Status filter checkboxes
            Constraint::Min(0),    // Job list
        ])
        .split(area);

    // Render search input
    self.search_input.render(f, chunks[0]);

    // Render status filters
    self.render_status_filters(f, chunks[1]);

    // Render job list
    self.render_job_list(f, chunks[2]);
}
```

#### Remove `render_filter_bar()` method (no longer needed)

#### Update footer to show search shortcut:
```rust
fn render_footer(&self, f: &mut Frame, area: Rect) {
    let mut footer_items = vec![];

    if self.search_input_focused {
        // Show search-specific shortcuts when search is active
        footer_items.push(Span::styled("[Type]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Search  ", Style::default().fg(FG_PRIMARY)));
        footer_items.push(Span::styled("[⏎/Esc]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Exit Search  ", Style::default().fg(FG_PRIMARY)));
    } else {
        footer_items.push(Span::styled("[↑↓]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Nav  ", Style::default().fg(FG_PRIMARY)));
        footer_items.push(Span::styled("[Tab]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Switch  ", Style::default().fg(FG_PRIMARY)));
        footer_items.push(Span::styled("[⏎]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" View Logs  ", Style::default().fg(FG_PRIMARY)));
        footer_items.push(Span::styled("[/]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Search  ", Style::default().fg(FG_PRIMARY)));
        footer_items.push(Span::styled("[f]", Style::default().fg(ACCENT)));
        footer_items.push(Span::styled(" Toggle Filters  ", Style::default().fg(FG_PRIMARY)));

        // ... rest of footer items ...
    }

    let footer = Paragraph::new(Line::from(footer_items))
        .alignment(Alignment::Center);

    f.render_widget(footer, area);
}
```

## Testing Checklist

After implementing the above changes:

### PipelineScreen Tests:
- [ ] Press '/' to focus search input
- [ ] Type text to filter pipelines in real-time
- [ ] Search filters by: pipeline number, branch, commit message, author
- [ ] Search works case-insensitively
- [ ] Press Enter/Esc to exit search input
- [ ] Search works alongside other filters (branch, date, status)
- [ ] Cursor movement (left, right, home, end) works
- [ ] Backspace/Delete keys work
- [ ] Visual feedback: focused border, cursor indicator
- [ ] Empty state message appears when no matches
- [ ] Selection resets properly after filtering

### PipelineDetailScreen Tests:
- [ ] Press '/' to focus search input (when not in filter mode)
- [ ] Type text to filter jobs in real-time
- [ ] Search filters by job name only
- [ ] Search works case-insensitively
- [ ] Press Enter/Esc to exit search input
- [ ] Search works alongside status filters
- [ ] Job selection resets to first match
- [ ] Empty state message appears when no matches
- [ ] All keyboard shortcuts work as expected

## Key Features

1. **'/' Key Focus**: Press '/' from anywhere to quickly focus the search input
2. **Real-time Filtering**: Results update as you type
3. **Case-Insensitive Search**: Searches are case-insensitive for better UX
4. **Multiple Field Search**: Pipeline screen searches across number, branch, commit message, and author
5. **Clear Visual Feedback**: Focused border and cursor indicator
6. **Works Alongside Filters**: Text search combines with other filter facets
7. **Easy Exit**: Press Enter or Esc to exit search and return to navigation

## Notes

- The `TextInput` widget is simple and focused - no complex state management
- Real-time filtering provides immediate feedback
- The implementation matches the Python version's UX
- The '/' key shortcut is a common pattern (like vim, Gmail, etc.)
- All filtering happens client-side for instant results
