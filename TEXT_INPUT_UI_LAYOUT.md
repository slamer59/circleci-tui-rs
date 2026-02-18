# TextInput Filter UI Layout

## Overview Layout

```
┌────────────────────────────────────────────────────────────────────────────┐
│ project › pipeline #123 › main › a1b2c3d                                  │
│ feat: add webhook retry logic • alice • 2h ago                            │
├────────────────────────────────────────────────────────────────────────────┤
│ project > pipeline #123                                                    │
├──────────────────────┬─────────────────────────────────────────────────────┤
│ WORKFLOWS (30%)      │ JOBS › build_and_test (70%)                        │
│                      │                                                     │
│ ▶ ✓ build_and_test   │ ┌─────────────────────────────────────────────┐   │
│   ● deploy_to_prod   │ │ Filter jobs...                              │   │ ⬅ TextInput
│                      │ └─────────────────────────────────────────────┘   │
│                      │ ☑ Success  ☑ Running  ☑ Failed  ☑ Pending  ...   │ ⬅ Checkboxes
│                      │                                                     │
│                      │ ● 14:23  build_backend        2m 30s    ●          │
│                      │      Completed successfully                        │
│                      │ ● 14:25  test_frontend        1m 15s    ●          │
│                      │      In progress...                                │
│                      │ ● 14:27  deploy_staging       45s       ●          │
│                      │      Completed successfully                        │
│                      │                                                     │
├──────────────────────┴─────────────────────────────────────────────────────┤
│ [↑↓] Nav  [Tab] Switch  [⏎] View Logs  [s] SSH  [/] Search  [f] Filters  │
└────────────────────────────────────────────────────────────────────────────┘
```

## Filter Panel Detail - Normal State (Not Focused)

```
┌─────────────────────────────────────────────┐
│ Filter jobs...                              │  ⬅ Placeholder shown (dim + italic)
└─────────────────────────────────────────────┘
☑ Success  ☑ Running  ☑ Failed  ☑ Pending  ☑ Blocked  │  ⏱ All durations
```

## Filter Panel Detail - Text Input Focused (Press '/')

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓  ⬅ Focused border (bright)
┃ build│                                     ┃  ⬅ Cursor shown, text visible
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
☑ Success  ☑ Running  ☑ Failed  ☑ Pending  ☑ Blocked  │  ⏱ All durations

Footer: [Type] Filter Text  [Tab/↓] To Checkboxes  [Esc] Exit
```

## Filter Panel Detail - Checkboxes Focused (Press Tab from TextInput)

```
┌─────────────────────────────────────────────┐  ⬅ Normal border
│ build                                       │  ⬅ Filter text persists
└─────────────────────────────────────────────┘
☑̲ S̲u̲c̲c̲e̲s̲s̲  ☑ Running  ☑ Failed  ☑ Pending  ☑ Blocked  │  ⏱ All durations
 ⬆ Underlined = focused

Footer: [←→/Tab] Navigate  [↑] To Text  [Space/⏎] Toggle/Open  [Esc] Exit
```

## State Transitions

### From Jobs Panel
```
Jobs Panel (viewing list)
    ↓ Press '/'
TextInput Focused (filter mode)
```

### From Workflows Panel
```
Workflows Panel (selecting workflow)
    ↓ Press '/'
TextInput Focused (filter mode)
```

### Within Filter Mode
```
TextInput Focused
    ↓ Press Tab or Down
Checkboxes Focused
    ↓ Press Up
TextInput Focused (cycle)

Either State
    ↓ Press Esc or 'f'
Return to Jobs Panel
```

## Focus Indicators

### TextInput Focus States

**Unfocused:**
```
┌─────────────────────┐  Border: BORDER (dim cyan)
│ Filter jobs...      │  Text: FG_DIM + italic (placeholder)
└─────────────────────┘
```

**Focused (Empty):**
```
┏━━━━━━━━━━━━━━━━━━━━━┓  Border: BORDER_FOCUSED (bright cyan)
┃ │Filter jobs...     ┃  Cursor: ACCENT (purple) + bold
┗━━━━━━━━━━━━━━━━━━━━━┛  Text: FG_DIM + italic
```

**Focused (With Text):**
```
┏━━━━━━━━━━━━━━━━━━━━━┓  Border: BORDER_FOCUSED (bright cyan)
┃ build│              ┃  Cursor: ACCENT (purple) + bold
┗━━━━━━━━━━━━━━━━━━━━━┛  Text: FG_BRIGHT
```

### Checkbox Focus States

**Unfocused:**
```
☑ Success  ☑ Running  ☑ Failed
```

**Success Focused:**
```
☑̲ S̲u̲c̲c̲e̲s̲s̲  ☑ Running  ☑ Failed
^--- Bold + Underlined
```

**Duration Focused:**
```
☑ Success  ☑ Running  ☑ Failed  ☑ Pending  ☑ Blocked  │  ⏱̲ A̲l̲l̲ d̲u̲r̲a̲t̲i̲o̲n̲s̲
                                                          ^--- Bold + Underlined
```

## Filtering Behavior Examples

### Example 1: Text Filter Only
```
Input: "build"

Visible Jobs:
✓ build_backend     (matches)
✓ build_frontend    (matches)
✗ test_integration  (hidden)
✗ deploy_staging    (hidden)
```

### Example 2: Text + Status Filter
```
Input: "build"
Status: ☑ Success, ☐ Running, ☐ Failed

Visible Jobs:
✓ build_backend     (success, matches text)
✗ build_frontend    (running, wrong status)
✗ test_integration  (doesn't match text)
```

### Example 3: Text + Status + Duration
```
Input: "build"
Status: ☑ Success, ☐ Running, ☐ Failed
Duration: Quick (< 1min)

Visible Jobs:
✗ build_backend     (success, matches text, but 2m30s > 1min)
✓ build_css         (success, matches text, 45s < 1min)
```

## Keyboard Navigation Map

```
                    ┌─────────────┐
                    │ Jobs Panel  │
                    └──────┬──────┘
                           │
                    Press '/' or 'f'
                           │
                           ↓
                ┌──────────────────┐
                │ TextInput Focus  │◄───┐
                └────────┬─────────┘    │
                         │              │
                  Press Tab/Down    Press Up
                         │              │
                         ↓              │
                ┌──────────────────┐    │
                │ Checkboxes Focus │────┘
                └────────┬─────────┘
                         │
                   Press Esc/'f'
                         │
                         ↓
                ┌──────────────────┐
                │ Jobs Panel       │
                └──────────────────┘
```

## Color Scheme

### Border Colors
- Normal: `BORDER` (rgb(0, 180, 180) - dim cyan)
- Focused: `BORDER_FOCUSED` (rgb(0, 255, 255) - bright cyan)

### Text Colors
- Placeholder: `FG_DIM` (rgb(100, 100, 100) - gray) + italic
- Input Text: `FG_BRIGHT` (rgb(200, 200, 200) - light gray)
- Cursor: `ACCENT` (rgb(200, 50, 255) - purple) + bold

### Status Colors
- Success: `SUCCESS` (green)
- Running: `RUNNING` (cyan/blue)
- Failed: `FAILED_TEXT` (red)
- Pending: `PENDING` (yellow)
- Blocked: `BLOCKED` (orange)

## Space Allocation

```
Total Right Panel Height: 100%
├─ Filter Bar (TextInput):  3 lines (fixed)
├─ Status Filters:          1 line  (fixed)
├─ Job List:               remaining (flexible)
└─ (Pagination info shown at bottom of job list)
```

## Responsive Behavior

- TextInput takes full width of right panel
- Minimum width: 20 characters (terminal width permitting)
- Text scrolls if longer than widget width
- Cursor always visible within widget bounds
