# TextInput Filter Implementation for Pipeline Detail Screen

## Overview
Added TextInput widget to the pipeline_detail screen for filtering jobs by name, matching the Python implementation's functionality.

## Changes Made

### 1. Added TextInput Import
**File:** `/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipeline_detail.rs`
```rust
use crate::ui::widgets::text_input::TextInput;
```

### 2. Added FilterFocus Enum
Created a new enum to track focus within the filter panel:
```rust
pub enum FilterFocus {
    TextInput,    // Text input box for filtering by name
    Checkboxes,   // Status checkboxes and duration dropdown
}
```

### 3. Updated PipelineDetailScreen Struct
Added two new fields:
```rust
pub filter_focus: FilterFocus,           // Track focus within filter panel
pub job_filter_input: TextInput,         // The TextInput widget instance
```

### 4. Initialized TextInput in Constructor
```rust
filter_focus: FilterFocus::TextInput,
job_filter_input: TextInput::new("Filter jobs..."),
```

### 5. Updated get_filtered_jobs Method
Changed to use TextInput widget value instead of string:
```rust
let filter_text = self.job_filter_input.value();
let matches_filter = if filter_text.is_empty() {
    true
} else {
    job.name.to_lowercase().contains(&filter_text.to_lowercase())
};
```

### 6. Updated render_filter_bar Method
Simplified to directly render the TextInput widget:
```rust
fn render_filter_bar(&mut self, f: &mut Frame, area: Rect) {
    // Set focus state based on filter focus
    if self.focus == PanelFocus::Filters && self.filter_focus == FilterFocus::TextInput {
        self.job_filter_input.set_focused(true);
    } else {
        self.job_filter_input.set_focused(false);
    }

    // Render the text input widget
    self.job_filter_input.render(f, area);
}
```

### 7. Enhanced handle_filter_input Method
Added comprehensive keyboard handling for TextInput:

**When TextInput is focused:**
- Type keys → Add to filter text
- Backspace → Delete character
- Left/Right → Move cursor
- Home/End → Jump to start/end
- Tab/Down → Move focus to checkboxes
- Esc/'f' → Exit filter mode

**When Checkboxes are focused:**
- Up → Move focus to text input
- Left/Right/Tab → Navigate checkboxes
- Space/Enter → Toggle checkbox or open dropdown

### 8. Added '/' Keybinding
In main handle_input method:
```rust
KeyCode::Char('/') => {
    // Activate text input filter from anywhere
    self.focus = PanelFocus::Filters;
    self.filter_focus = FilterFocus::TextInput;
    PipelineDetailAction::None
}
```

### 9. Updated Footer Help Text
Enhanced footer to show context-sensitive keyboard shortcuts:
- Shows "[/] Search" in normal mode
- Shows "[Type] Filter Text [Tab/↓] To Checkboxes" when text input focused
- Shows "[←→/Tab] Navigate [↑] To Text [Space/⏎] Toggle/Open" when checkboxes focused

### 10. Updated get_pagination_info Method
Changed to use TextInput value:
```rust
let has_text_filter = !self.job_filter_input.value().is_empty();
```

## User Workflow

### Activating Filter
1. Press `/` from anywhere → Activates text input
2. Press `f` to toggle filter mode → Starts at text input

### Using Text Filter
1. Type characters → Filter jobs by name (case-insensitive)
2. Backspace/Delete → Edit filter text
3. Left/Right arrows → Move cursor
4. Tab/Down → Move to checkboxes

### Navigating Checkboxes
1. Left/Right/Tab → Navigate between filters
2. Up → Return to text input
3. Space/Enter → Toggle checkbox or open dropdown

### Exiting Filter Mode
1. Press `Esc` or `f` to return to jobs panel

## Features

### Text Filtering
- **Case-insensitive matching**: Filters job names regardless of case
- **Real-time filtering**: Jobs update immediately as you type
- **Visual feedback**: Cursor shown when focused
- **Placeholder text**: "Filter jobs..." shown when empty

### Focus Management
- Clear visual indicators for focused elements
- Text input shows focused border when active
- Checkboxes show underline when focused
- Smooth focus transitions between elements

### Integration with Other Filters
- Text filter works alongside status filters (Success, Running, Failed, etc.)
- Works with duration filters (Quick, Short, Medium, etc.)
- All filters combine using AND logic
- Job list updates in real-time

## Testing Recommendations

1. **Text Input Functionality**
   - Type various characters (alphanumeric, special chars)
   - Test cursor movement (Left, Right, Home, End)
   - Test text deletion (Backspace, Delete)
   - Test with empty filter vs. populated filter

2. **Focus Management**
   - Press '/' to activate from different panels
   - Tab between text input and checkboxes
   - Up arrow from checkboxes to text input
   - Verify visual focus indicators

3. **Filter Combinations**
   - Text filter alone
   - Text filter + status filters
   - Text filter + duration filter
   - All filters combined

4. **Edge Cases**
   - Filter with no matching jobs
   - Filter with all jobs matching
   - Very long filter text
   - Special characters in filter

## Files Modified
- `/home/pedot/projects/circleci-tui-rs/src/ui/screens/pipeline_detail.rs`

## Dependencies
- `TextInput` widget from `src/ui/widgets/text_input.rs`
- No external crate dependencies added

## Future Enhancements
- Add regex support for advanced filtering
- Add filter history (up/down arrows)
- Add filter suggestions/autocomplete
- Add ability to filter by job status in text input
