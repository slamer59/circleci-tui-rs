# Pipeline Screen Text Search Implementation

## Overview

Successfully added a TextInput widget to the pipelines screen for searching and filtering pipelines in real-time.

## Changes Made

### File Modified
- `/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipelines.rs`

### 1. Added Imports

```rust
use crate::ui::widgets::text_input::TextInput;
```

### 2. Updated PipelineScreen Struct

Added two new fields:

```rust
pub struct PipelineScreen {
    // ... existing fields ...

    /// Text input widget for search/filtering
    pub search_input: TextInput,

    /// Whether search input is focused
    pub search_focused: bool,

    // ... other fields ...
}
```

### 3. Initialized in Constructor

```rust
let search_input = TextInput::new("Filter pipelines...");

Self {
    // ... other fields ...
    search_input,
    search_focused: false,
    // ... other fields ...
}
```

### 4. Updated apply_filters() Method

Added text search filtering that searches across:
- Pipeline number (as string)
- Branch name
- Commit message
- Author name

All searches are case-insensitive and use substring matching:

```rust
// Get text search value
let search_text = self.search_input.value().to_lowercase();

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

// Combine with faceted filters using AND logic
text_match && owner_match && branch_match && date_match && status_match
```

### 5. Updated Layout

Changed from 4 sections to 5 sections to include the text input:

```rust
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
```

### 6. Added render_search_input() Method

```rust
fn render_search_input(&mut self, f: &mut Frame, area: Rect) {
    // Set focus state on the text input widget based on search focus
    self.search_input.set_focused(self.search_focused);

    // Render the text input widget
    self.search_input.render(f, area);
}
```

### 7. Enhanced Keyboard Handling

Updated `handle_input()` to support three modes:

#### Mode 1: Search Input Focused (`search_focused = true`)
- **Type any character**: Add to search text and filter in real-time
- **Backspace/Delete**: Edit text
- **Arrow keys**: Move cursor within text
- **Home/End**: Jump to start/end of text
- **Tab**: Move focus to faceted search buttons
- **Esc**: Clear text (if not empty) or unfocus (if empty)

#### Mode 2: Faceted Search Active (`filter_active = true`)
- **Arrow keys/Tab**: Navigate between filter buttons
- **Space/Enter**: Toggle filter option
- **Shift+Tab**: Return focus to search input
- **Esc**: Exit filter mode

#### Mode 3: Normal Navigation (neither focused)
- **Up/Down**: Navigate pipeline list
- **Enter**: Open selected pipeline
- **/**: Focus search input
- **r**: Refresh pipelines
- **Esc**: Clear all filters (text + faceted)

### 8. Updated Footer Shortcuts

Added contextual footer based on focus state:

#### When Search Input Focused
```
[Type] Filter Text  [Tab] To Filter Buttons  [Esc] Clear/Exit
```

#### When Faceted Search Active
```
[←→/Tab] Switch Filter  [↑↓] Navigate  [⏎/Space] Select  [Shift+Tab] To Search  [Esc] Exit Filter
```

#### When Nothing Focused (Normal Mode)
```
[↑↓] Nav  [⏎] Open  [/] Search  [r] Refresh  [Esc] Clear Filters  [?] Help  [q] Quit
```

## Features

### Real-time Filtering
- Filters update as you type (no need to press Enter)
- Performance is instant even with large pipeline lists

### Case-Insensitive Search
- "MAIN" matches "main"
- "alice" matches "Alice"

### Multi-field Search
Searches across 4 fields:
1. **Pipeline Number**: "1234" finds pipeline #1234
2. **Branch Name**: "main" finds all pipelines on main branch
3. **Commit Message**: "feat" finds all feature commits
4. **Author Name**: "alice" finds all pipelines by Alice

### Combined Filtering (AND Logic)
- Text search + Owner filter + Branch filter + Date filter + Status filter
- All filters combine with AND logic
- Example: Search "alice" + Filter "main" branch → only Alice's pipelines on main

### Keyboard-First Design
- **/** activates search from anywhere
- **Tab** moves between search input and filter buttons
- **Esc** clears all filters or exits current mode

### Visual Feedback
- Border highlights when focused (BORDER_FOCUSED color)
- Placeholder text when empty ("Filter pipelines...")
- Cursor shown when focused (accent colored `│`)
- Dimmed, italic placeholder text

## Architecture

### Flow
1. User presses `/` → `search_focused = true`
2. User types → `search_input.handle_key()` updates internal value
3. After each keystroke → `apply_filters()` re-filters pipelines
4. Filtered pipelines displayed in list
5. Tab → move to faceted search buttons
6. Esc → clear and return to normal mode

### State Management
- `search_input: TextInput` - Widget holding text value and cursor state
- `search_focused: bool` - Whether search input has keyboard focus
- `filter_active: bool` - Whether faceted search has keyboard focus

### Integration with Existing Filters
The text search integrates seamlessly with the existing 4 faceted filters:
- Owner (All / Mine)
- Branch (All / specific branches)
- Date (Last 24h, 7d, 30d, 90d, All time)
- Status (All / success / failed / running / pending)

All 5 filters combine using AND logic in `apply_filters()`.

## Testing

See `PIPELINE_SEARCH_TEST_CHECKLIST.md` for comprehensive test cases.

Key tests:
- Text input focus and rendering
- Real-time filtering across all 4 fields
- Keyboard navigation between modes
- Combined filtering (text + faceted)
- Filter clearing
- Empty states

## Implementation Notes

### Design Decision: Placement
Placed text input **above** the faceted search buttons because:
- Natural top-to-bottom flow
- Text search is often the first filter users apply
- Matches the pattern from pipeline_detail screen (job filters)

### Design Decision: Real-time Filtering
Applied filters on every keystroke (no debouncing) because:
- Filtering is extremely fast (substring matching in memory)
- Provides immediate feedback
- Better UX than requiring Enter key

### Design Decision: Combined Search Fields
Searches across all 4 fields (pipeline #, branch, commit, author) because:
- Users may not remember which field contains the text
- Reduces cognitive load (no field selection dropdown needed)
- Matches git log behavior (searches across all relevant fields)

## Future Enhancements (Not Implemented)

Possible improvements:
- Regex search support
- Fuzzy matching (typo tolerance)
- Search history (recent searches)
- Field-specific search syntax (e.g., "author:alice")
- Highlighting matching text in results
- Search result count display
- Saved filter presets

## Dependencies

Uses the existing `TextInput` widget from:
- `/home/pedot/projects/circleci-tui-rs/src/ui/widgets/text_input.rs`

No new dependencies added.

## Compatibility

- Works with existing API integration
- Compatible with mock data mode
- No breaking changes to public API
- Follows existing theme and color scheme

## Build Instructions

```bash
cd /home/pedot/projects/circleci-tui-rs
cargo build --release
cargo run
```

Then:
1. Navigate to pipelines screen (should be default)
2. Press `/` to activate text search
3. Start typing to filter pipelines
4. Use Tab to switch to faceted filters
5. Press Esc to clear all filters
