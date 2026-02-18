# Filter UI - Keyboard Navigation Guide

## Quick Reference Card

### Pipelines Screen - Normal Mode

```
┌─────────────────────────────────────────────┐
│  CircleCI Pipelines                         │
├─────────────────────────────────────────────┤
│ Filter Bar: [All] [main] [Any time] [All]  │
│                                             │
│  Pipeline List                              │
│  ● Pipeline #123  - 2h ago                  │
│  ● Pipeline #122  - 4h ago                  │
│                                             │
├─────────────────────────────────────────────┤
│ [↑↓] Nav  [⏎] Open  [/] Filters  [r] Refresh │
│ [Esc] Clear Filters  [?] Help  [q] Quit    │
└─────────────────────────────────────────────┘

Keys:
  ↑/↓     - Navigate pipelines
  ⏎       - Open pipeline detail
  /       - Activate filter mode
  r       - Refresh pipeline list
  Esc     - Clear all filters
  ?       - Show help
  q       - Quit application
```

### Pipelines Screen - Filter Mode

```
┌─────────────────────────────────────────────┐
│  CircleCI Pipelines (2 filters active)     │
├─────────────────────────────────────────────┤
│ [All] [main] [Last week] [failed] (2 filters)│
│   ▔▔▔▔▔ (focused with underline)            │
│  ┌─────────────┐                            │
│  │ All         │← Dropdown open              │
│  │ main     ✓  │                             │
│  │ dev         │                             │
│  └─────────────┘                            │
├─────────────────────────────────────────────┤
│ [←→/Tab] Switch Filter  [↑↓] Navigate       │
│ [⏎/Space] Select  [Esc] Exit Filter        │
└─────────────────────────────────────────────┘

Keys:
  ←/→/Tab - Switch between filter buttons
  ↑/↓     - Navigate dropdown options
  ⏎/Space - Open dropdown or confirm selection
  Esc     - Exit filter mode / close dropdown
```

### Pipeline Detail Screen - Normal Mode

```
┌─────────────────────────────────────────────┐
│ project › pipeline #123 › main › a1b2c3d   │
│ feat: add webhook retry logic • alice • 2h │
├──────────────┬──────────────────────────────┤
│ WORKFLOWS    │ JOBS › build-and-test        │
│              │ ┌──────────────────────────┐ │
│ ▶ ● build... │ │ Filter: [___]            │ │
│   ● test...  │ ├──────────────────────────┤ │
│   ● deploy   │ │ ☑ Success  ☑ Running     │ │
│              │ │ ☑ Failed   ☑ Pending     │ │
│              │ ├──────────────────────────┤ │
│              │ │ ● build-backend          │ │
│              │ │ ● test-frontend          │ │
├──────────────┴──────────────────────────────┤
│ [↑↓] Nav  [Tab] Switch  [⏎] View Logs       │
│ [s] SSH  [f] Toggle Filters  [Esc] Back    │
└─────────────────────────────────────────────┘

Keys:
  ↑/↓     - Navigate workflows/jobs (depends on focus)
  Tab     - Switch between workflows and jobs panel
  ⏎       - View job logs
  s       - SSH to job
  f       - Toggle filter mode
  R       - Rerun workflow (when on workflows panel)
  l       - Load more jobs (if pagination available)
  Esc     - Go back to pipelines
```

### Pipeline Detail Screen - Filter Mode

```
┌─────────────────────────────────────────────┐
│ JOBS › build-and-test                       │
│ ┌──────────────────────────┐                │
│ │ Filter: [test]           │                │
│ ├──────────────────────────┤                │
│ │ ☑ Success  ☑ Running     │← Checkboxes   │
│ │   ▔▔▔▔▔▔▔  (underlined)                   │
│ │ ☑ Failed   ☑ Pending     │                │
│ │ ☑ Blocked                │                │
│ ├──────────────────────────┤                │
├─────────────────────────────────────────────┤
│ [←→/Tab] Navigate  [Space/⏎] Toggle/Open    │
│ [Esc/f] Exit Filter Mode                    │
└─────────────────────────────────────────────┘

Keys:
  ←/→/Tab  - Navigate between filter checkboxes
  Space/⏎  - Toggle checkbox or open duration dropdown
  ↑/↓      - Navigate duration dropdown (when open)
  Esc/f    - Exit filter mode
```

## Visual States Reference

### Filter Button States

#### 1. Inactive (Default)
```
 [All pipelines]
```
- Dim foreground color
- No special styling

#### 2. Focused
```
 [All pipelines]
  ▔▔▔▔▔▔▔▔▔▔▔▔▔
```
- Bright color
- **Bold text**
- **Underlined**

#### 3. Filtered (Active)
```
 [My pipelines]
```
- Accent color (cyan)
- **Bold text**
- Stands out from defaults

#### 4. Pressed (Dropdown Open)
```
 [All pipelines]
 ▔▔▔▔▔▔▔▔▔▔▔▔▔▔
```
- Dark background
- Bright foreground
- **Bold text**

### Filter Count Indicator

When filters are active:
```
┌────────────────────────────────────────┐
│ CircleCI Pipelines (2 filters active) │← Header
├────────────────────────────────────────┤
│ Filter Bar (2 filters)                 │← Filter bar
└────────────────────────────────────────┘
```

### Dropdown Menu

```
┌─────────────┐
│ All      ✓  │← Currently selected
│ Success     │← Available option
│ Failed      │← Available option
│ Running     │← Available option
└─────────────┘
```

- Checkmark (✓) shows current selection
- Highlighted item follows cursor position
- Border uses accent color when open

## Tips and Tricks

### Quick Filter Navigation
1. Press `/` to enter filter mode
2. Use `Tab` to quickly jump between filters
3. Press `Space` to open dropdown
4. Use `↑/↓` to select option
5. Press `⏎` to confirm and close

### Filter Combinations
- Combine multiple filters for precise results
- Filter count shows in both header and filter bar
- All filters persist when navigating between screens

### Keyboard Efficiency
- Use `Tab` for fast forward navigation
- Use `←→` for precise left/right movement
- `Space` and `⏎` both work for selection
- `Esc` quickly exits any mode

### Visual Feedback
- **Underline** = Currently focused item
- **Bold + Accent** = Active filter (non-default)
- **Dark background** = Dropdown open
- **Count badge** = Number of active filters

## Common Workflows

### Filter by Branch and Status
1. Press `/` to activate filters
2. `Tab` to navigate to branch filter
3. `Space` to open dropdown
4. `↓` to select your branch
5. `⏎` to confirm
6. `Tab` to status filter
7. Repeat steps 3-5 for status
8. `Esc` to exit filter mode

### Clear All Filters
- Simply press `Esc` when not in filter mode
- All filters reset to defaults instantly

### View Filtered Pipelines
1. Apply your filters
2. `⏎` to open a pipeline detail
3. Navigate and view jobs
4. `Esc` to return to pipelines
5. Filters are still active!

## Accessibility Notes

- High contrast colors for visibility
- Clear visual states (inactive, focused, filtered, pressed)
- Multiple ways to perform actions (keyboard alternatives)
- Consistent keyboard shortcuts across screens
- Visual feedback for all state changes
- Status information always visible

## Performance

- Filter state automatically persists in memory
- No network calls when changing filters
- Instant UI updates
- Efficient rendering with ratatui
- All tests pass (97/97)

---

For more information, see:
- `FILTER_UI_POLISH.md` - Implementation details
- `QUICK_REFERENCE.md` - General UI guide
- Help modal (`?` key) - In-app help
