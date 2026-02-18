# TextInput Filter Testing Checklist

## Build and Compilation
- [ ] Project compiles without errors: `cargo build`
- [ ] No warnings related to TextInput changes: `cargo clippy`
- [ ] Tests pass: `cargo test`

## Basic Functionality
- [ ] Press `/` from workflows panel → Text input activates
- [ ] Press `/` from jobs panel → Text input activates
- [ ] Press `f` → Text input activates (or toggles if already in filter mode)
- [ ] Text input shows "Filter jobs..." placeholder when empty
- [ ] Text input shows border highlight when focused

## Text Input Operations
- [ ] Type letters → Text appears in input
- [ ] Type numbers → Text appears in input
- [ ] Type special characters → Text appears in input
- [ ] Backspace → Deletes character before cursor
- [ ] Delete → Deletes character after cursor
- [ ] Left arrow → Moves cursor left
- [ ] Right arrow → Moves cursor right
- [ ] Home key → Moves cursor to start
- [ ] End key → Moves cursor to end
- [ ] Cursor is visible ("|" character) when input is focused

## Focus Navigation
- [ ] Tab from text input → Moves to first checkbox (Success)
- [ ] Down arrow from text input → Moves to first checkbox
- [ ] Up arrow from checkboxes → Moves back to text input
- [ ] Text input border changes when focus changes
- [ ] Checkboxes show underline when focused

## Filtering Behavior
- [ ] Type "build" → Only jobs with "build" in name are shown
- [ ] Type "BUILD" → Case-insensitive: same jobs shown
- [ ] Type partial name → Matches jobs containing that substring
- [ ] Clear filter → All jobs shown again
- [ ] No matching jobs → Shows empty state with "(•_•)" emoji
- [ ] Filter with 1 match → Job list shows only that job

## Integration with Status Filters
- [ ] Text filter + Success checkbox → Shows only success jobs matching text
- [ ] Text filter + Failed checkbox → Shows only failed jobs matching text
- [ ] Text filter + Multiple checkboxes → Shows jobs matching any status AND text
- [ ] Uncheck all statuses → Shows no jobs (even with text filter)
- [ ] Text filter works when statuses are partially selected

## Integration with Duration Filter
- [ ] Text filter + "Quick (< 1min)" → Shows only quick jobs matching text
- [ ] Text filter + "Long (15-30min)" → Shows only long jobs matching text
- [ ] Text filter + "All durations" → Shows all jobs matching text

## Exit and Reset
- [ ] Press Esc from text input → Returns to jobs panel
- [ ] Press 'f' from text input → Returns to jobs panel
- [ ] Press Esc from checkboxes → Returns to jobs panel
- [ ] Filter text persists when exiting and re-entering filter mode
- [ ] Filter continues to apply when not in filter mode

## Visual Feedback
- [ ] Focused text input shows highlighted border (BORDER_FOCUSED color)
- [ ] Unfocused text input shows normal border (BORDER color)
- [ ] Cursor position is correct when typing
- [ ] Cursor position is correct after arrow key movements
- [ ] Placeholder text is dimmed and italic

## Footer Updates
- [ ] Footer shows "[/] Search" when not in filter mode
- [ ] Footer shows "[Type] Filter Text" when text input focused
- [ ] Footer shows "[Tab/↓] To Checkboxes" when text input focused
- [ ] Footer shows "[←→/Tab] Navigate [↑] To Text" when checkboxes focused
- [ ] Footer updates correctly when switching focus

## Pagination Info
- [ ] Shows correct count: "Showing X of Y" with text filter active
- [ ] Count updates in real-time as filter text changes
- [ ] Count shows filtered vs total correctly

## Edge Cases
- [ ] Empty job list → No crash, shows appropriate message
- [ ] Very long filter text (50+ chars) → Handles gracefully
- [ ] Filter text with special regex chars (., *, ?, etc.) → Treats as literal
- [ ] Rapid typing → All characters captured correctly
- [ ] Filter job that doesn't exist → Shows empty state

## Performance
- [ ] Filtering 100+ jobs → Smooth, no lag
- [ ] Real-time updates while typing → Responsive
- [ ] Switching between focus states → Instant

## Integration Testing
- [ ] Load pipeline → Select workflow → Open filter → Type filter → View logs
- [ ] Filter jobs → Select filtered job → Open logs → Logs open correctly
- [ ] Filter jobs → Press SSH → SSH modal opens with correct job
- [ ] Load more jobs → Filter applies to newly loaded jobs
- [ ] Rerun workflow → Filter state persists

## Regression Testing
- [ ] Status checkboxes still work independently
- [ ] Duration dropdown still works
- [ ] Job selection still works
- [ ] Log viewing still works
- [ ] SSH functionality still works
- [ ] Workflow navigation still works
- [ ] Pagination still works

## Comparison with Python Version
- [ ] Text input appears in same location as Python version
- [ ] Keyboard shortcuts match Python version
- [ ] Filter behavior matches Python version (case-insensitive substring)
- [ ] Visual appearance is consistent with overall theme
