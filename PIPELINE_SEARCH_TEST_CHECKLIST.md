# Pipeline Screen Text Search Test Checklist

## Implementation Summary

Added a TextInput widget to the pipelines screen for searching/filtering pipelines by:
- Pipeline number
- Branch name
- Commit message
- Author name

## Test Cases

### 1. Basic Text Input

- [ ] Press `/` from pipeline list → text input should be focused (border highlighted)
- [ ] Type characters → text should appear in the input field
- [ ] Input should show placeholder "Filter pipelines..." when empty and unfocused
- [ ] Cursor should be visible when focused (shown as `│`)

### 2. Text Filtering

#### Pipeline Number Search
- [ ] Type a pipeline number (e.g., "1234") → only pipelines with that number should show
- [ ] Type partial number (e.g., "123") → all pipelines containing "123" should show

#### Branch Name Search
- [ ] Type "main" → only pipelines on main branch should show
- [ ] Type "feature" → only pipelines on feature branches should show
- [ ] Case insensitive: "MAIN" should match "main"

#### Commit Message Search
- [ ] Type keyword from commit message (e.g., "feat") → matching pipelines should show
- [ ] Type partial text from commit → matching pipelines should show
- [ ] Case insensitive search should work

#### Author Name Search
- [ ] Type author name (e.g., "alice") → only pipelines by that author should show
- [ ] Type partial author name → matching pipelines should show
- [ ] Case insensitive search should work

### 3. Keyboard Navigation

#### From Pipeline List
- [ ] Press `/` → focus text input
- [ ] Press `↑` or `↓` → navigate pipelines (when not focused on input)

#### When Text Input Focused
- [ ] Type characters → filter updates in real-time
- [ ] Press `←` or `→` → move cursor within text
- [ ] Press `Home` → cursor moves to start
- [ ] Press `End` → cursor moves to end
- [ ] Press `Backspace` → delete character before cursor
- [ ] Press `Delete` → delete character after cursor
- [ ] Press `Tab` → move focus to faceted search buttons
- [ ] Press `Esc` when text present → clear text
- [ ] Press `Esc` when text empty → unfocus input

#### When Faceted Search Focused
- [ ] Press `Shift+Tab` → return focus to text input
- [ ] Press `Esc` → exit filter mode

### 4. Combined Filtering (Text + Faceted Filters)

- [ ] Enter text search + apply owner filter → both filters should apply (AND logic)
- [ ] Enter text search + apply branch filter → both filters should apply
- [ ] Enter text search + apply date filter → both filters should apply
- [ ] Enter text search + apply status filter → both filters should apply
- [ ] All 5 filters (text + 4 facets) → all should combine with AND logic

### 5. Filter Clearing

- [ ] Press `Esc` from main view → clear both text search and faceted filters
- [ ] Text input should clear
- [ ] Faceted filters should reset to defaults
- [ ] All pipelines should be visible again

### 6. Visual Design

- [ ] Text input border should be `BORDER` color when unfocused
- [ ] Text input border should be `BORDER_FOCUSED` color when focused
- [ ] Placeholder text should be dimmed and italic
- [ ] Input text should be bright and visible
- [ ] Cursor should be accent colored and bold

### 7. Footer Updates

#### Normal Mode (Nothing Focused)
- [ ] Should show: `[/] Search` (along with other shortcuts)

#### Search Input Focused
- [ ] Should show: `[Type] Filter Text  [Tab] To Filter Buttons  [Esc] Clear/Exit`

#### Faceted Search Focused
- [ ] Should show: `[Shift+Tab] To Search` (along with filter shortcuts)

### 8. Empty State

- [ ] When text filter results in no matches → show "No pipelines found" with helpful message
- [ ] Message should suggest clearing filters or refreshing

### 9. Real-time Filtering

- [ ] As user types → pipeline list updates immediately
- [ ] No need to press Enter to apply filter
- [ ] Debouncing not required (filtering is fast)

### 10. Layout

- [ ] Text input should appear above faceted search buttons
- [ ] Text input height: 3 lines (with border)
- [ ] Should not overlap with pipeline list
- [ ] Should be visible on screen

## Edge Cases

- [ ] Empty search text → show all pipelines (respecting other filters)
- [ ] Search text with no matches → empty state shown
- [ ] Very long search text → input should handle gracefully (no overflow)
- [ ] Special characters in search → should work correctly
- [ ] Unicode characters → should work correctly

## Performance

- [ ] Filtering should be instant (no noticeable lag)
- [ ] Real-time updates should not cause flickering
- [ ] Large pipeline lists (100+) → filtering should still be fast

## Integration

- [ ] Text search works alongside all 4 faceted filters
- [ ] Clearing filters (Esc) clears both text and faceted filters
- [ ] Refreshing (r) maintains filter state
- [ ] Navigation between text input and faceted filters is smooth

## Build and Run

To test:
```bash
cd /home/pedot/projects/circleci-tui-rs
cargo build --release
cargo run
```

Then:
1. Navigate to pipelines screen
2. Press `/` to activate text search
3. Type to filter pipelines
4. Use Tab to switch to faceted filters
5. Verify combined filtering works

## Known Limitations

- Search is substring match (not regex or fuzzy search)
- Case-insensitive only (no case-sensitive mode)
- No search history or autocomplete
