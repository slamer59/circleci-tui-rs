# Faceted Search - Visual Guide

## Filter Bar Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ ☍ All pipelines  ⎇ All  📅 All time  ● All                     │
└─────────────────────────────────────────────────────────────────┘
```

## Filter States

### Inactive Button (Not Focused, Default Selection)
```
┌─────────────────────────────────────────────────────────────────┐
│ ☍ All pipelines  ⎇ All  📅 All time  ● All                     │
│ ^^^^^^^^^^^^^^^                                                  │
│ dim gray text                                                    │
└─────────────────────────────────────────────────────────────────┘
```

### Focused Button (No Dropdown)
```
┌─────────────────────────────────────────────────────────────────┐
│ ☍ All pipelines  ⎇ All  📅 All time  ● All                     │
│ ┗━━━━━━━━━━━━━━━                                                │
│ bright white + bold                                              │
└─────────────────────────────────────────────────────────────────┘
```

### Pressed Button (Dropdown Open)
```
┌─────────────────────────────────────────────────────────────────┐
│ ☍ All pipelines  ⎇ All  📅 All time  ● All                     │
│ ┗━━━━━━━━━━━━━━━                                                │
│ dark background + bright white + bold                            │
│                                                                  │
│ ┌──────────────────┐                                            │
│ │ ✓ All pipelines  │ <- dropdown below button                  │
│ │   Mine           │                                            │
│ └──────────────────┘                                            │
└─────────────────────────────────────────────────────────────────┘
```

### Filtered Button (Non-Default Selection)
```
┌─────────────────────────────────────────────────────────────────┐
│ ☍ Mine  ⎇ main  📅 Last 7 days  ● failed                       │
│ ┗━━━━  ┗━━━━━  ┗━━━━━━━━━━━━━  ┗━━━━━━━                       │
│ cyan/accent color + bold                                         │
└─────────────────────────────────────────────────────────────────┘
```

## Dropdown Menu States

### Focused Item in Dropdown
```
┌──────────────────┐
│   All pipelines  │
│ ▸ ✓ Mine         │ <- dark background + bright text + bold
└──────────────────┘
```

### Selected Item (Not Focused)
```
┌──────────────────┐
│ ✓ All pipelines  │ <- cyan/accent text
│   Mine           │ <- normal text
└──────────────────┘
```

## Example Scenarios

### Scenario 1: Default State
```
┌───────────────────────────────────────────────────────────────────┐
│  CircleCI Pipelines - gh/acme/api-service                         │
├───────────────────────────────────────────────────────────────────┤
│ ☍ All pipelines  ⎇ All  📅 All time  ● All                       │
├───────────────────────────────────────────────────────────────────┤
│ ● 12:34 Pipeline #12345  2h ago  ● main                          │
│           Implement faceted search integration                     │
│           3 workflows • 24 jobs • success (45m)                   │
│                                                                    │
│ ● 11:22 Pipeline #12344  3h ago  ● dev                           │
│           Fix CI/CD pipeline configuration                         │
│           3 workflows • 24 jobs • failed (12m)                    │
└───────────────────────────────────────────────────────────────────┘
```

### Scenario 2: Active Filters (2 filters applied)
```
┌───────────────────────────────────────────────────────────────────┐
│  CircleCI Pipelines - gh/acme/api-service (2 filters active)     │
├───────────────────────────────────────────────────────────────────┤
│ ☍ Mine  ⎇ main  📅 Last 7 days  ● All                            │
│ ^^^^^   ^^^^^^   ^^^^^^^^^^^^^                                     │
│ cyan    cyan     cyan                                              │
├───────────────────────────────────────────────────────────────────┤
│ ● 12:34 Pipeline #12345  2h ago  ● main                          │
│           Implement faceted search integration                     │
│           3 workflows • 24 jobs • success (45m)                   │
│                                                                    │
│ ● 09:15 Pipeline #12340  1d ago  ● main                          │
│           Update README documentation                              │
│           3 workflows • 24 jobs • success (8m)                    │
└───────────────────────────────────────────────────────────────────┘
```

### Scenario 3: Filter Active with Dropdown Open
```
┌───────────────────────────────────────────────────────────────────┐
│  CircleCI Pipelines - gh/acme/api-service                         │
├───────────────────────────────────────────────────────────────────┤
│ ☍ All pipelines  ⎇ All  📅 All time  ● All                       │
│   ^^^^^^^^^^^^^                                                    │
│   dark bg + bright white + bold                                   │
│                                                                    │
│ ┌──────────────────┐                                              │
│ │ ▸ ✓ All pipelines│                                              │
│ │     Mine         │                                              │
│ └──────────────────┘                                              │
├───────────────────────────────────────────────────────────────────┤
│ (pipelines dimmed while dropdown is open)                         │
└───────────────────────────────────────────────────────────────────┘
```

### Scenario 4: Branch Dropdown with Many Options
```
┌───────────────────────────────────────────────────────────────────┐
│  CircleCI Pipelines - gh/acme/api-service                         │
├───────────────────────────────────────────────────────────────────┤
│ ☍ All pipelines  ⎇ All  📅 All time  ● All                       │
│                  ^^^^^                                             │
│                  dark bg + bright white                           │
│                                                                    │
│                  ┌────────────────┐                               │
│                  │ ▸ ✓ All        │                               │
│                  │     main       │                               │
│                  │     dev        │                               │
│                  │     feature-x  │                               │
│                  │     hotfix-123 │                               │
│                  └────────────────┘                               │
├───────────────────────────────────────────────────────────────────┤
│ (pipelines shown after selection)                                 │
└───────────────────────────────────────────────────────────────────┘
```

## Footer States

### Normal Mode (Filter Inactive)
```
┌───────────────────────────────────────────────────────────────────┐
│ [↑↓] Nav  [⏎] Open  [/] Filters  [r] Refresh  [Esc] Clear        │
│ Filters  [?] Help  [q] Quit                                       │
└───────────────────────────────────────────────────────────────────┘
```

### Filter Mode (Filter Active)
```
┌───────────────────────────────────────────────────────────────────┐
│ [←→/Tab] Switch Filter  [↑↓] Navigate  [⏎/Space] Select          │
│ [Esc] Exit Filter  [Esc] Clear Filters                            │
└───────────────────────────────────────────────────────────────────┘
```

## Color Scheme Reference

| State | Foreground | Background | Modifier |
|-------|-----------|-----------|----------|
| Inactive | FG_DIM (gray) | - | - |
| Focused | FG_BRIGHT (white) | - | BOLD |
| Pressed | FG_BRIGHT (white) | DarkGray | BOLD |
| Filtered | ACCENT (cyan) | - | BOLD |
| Dropdown Border | ACCENT (cyan) | - | - |
| Dropdown Item (focused) | FG_BRIGHT (white) | DarkGray | BOLD |
| Dropdown Item (selected) | ACCENT (cyan) | - | - |
| Dropdown Item (normal) | FG_PRIMARY (white) | - | - |
| Checkmark | - | - | - |

## Icon Reference

| Icon | Meaning | Unicode |
|------|---------|---------|
| ☍ | Owner/User Filter | U+260D |
| ⎇ | Branch Filter | U+2387 |
| 📅 | Date Filter | U+1F4C5 |
| ● | Status Filter | U+25CF |
| ✓ | Selected/Checked | U+2713 |
| ▸ | Focused Item | U+25B8 |

## Interaction Flow

### Opening a Filter
```
1. Press '/' to activate filter mode
   ┌─────────────────────────────────────┐
   │ ☍ All pipelines  ⎇ All  📅 All time│
   │ ┗━━━━━━━━━━━━━━━                   │
   │ (first button focused)              │
   └─────────────────────────────────────┘

2. Press Enter or Space to open dropdown
   ┌─────────────────────────────────────┐
   │ ☍ All pipelines  ⎇ All  📅 All time│
   │ ┗━━━━━━━━━━━━━━━                   │
   │ ┌──────────────────┐                │
   │ │ ▸ ✓ All pipelines│                │
   │ │     Mine         │                │
   │ └──────────────────┘                │
   └─────────────────────────────────────┘

3. Use ↑/↓ to navigate options
   ┌─────────────────────────────────────┐
   │ ☍ All pipelines  ⎇ All  📅 All time│
   │ ┗━━━━━━━━━━━━━━━                   │
   │ ┌──────────────────┐                │
   │ │   ✓ All pipelines│                │
   │ │ ▸   Mine         │                │
   │ └──────────────────┘                │
   └─────────────────────────────────────┘

4. Press Enter or Space to confirm
   ┌─────────────────────────────────────┐
   │ ☍ Mine  ⎇ All  📅 All time         │
   │ ┗━━━━                               │
   │ (cyan/accent color indicates filter)│
   │ (pipelines now filtered)            │
   └─────────────────────────────────────┘
```

### Switching Between Filters
```
1. Press → or Tab to move to next filter
   ┌─────────────────────────────────────┐
   │ ☍ Mine  ⎇ All  📅 All time         │
   │         ┗━━━━                       │
   │         (branch filter now focused) │
   └─────────────────────────────────────┘

2. Press ← to move to previous filter
   ┌─────────────────────────────────────┐
   │ ☍ Mine  ⎇ All  📅 All time         │
   │ ┗━━━━                               │
   │ (owner filter focused again)        │
   └─────────────────────────────────────┘
```

### Clearing Filters
```
1. Press Esc (when no dropdown is open)
   ┌─────────────────────────────────────┐
   │ ☍ All pipelines  ⎇ All  📅 All time│
   │ (all filters reset to defaults)     │
   │ (all pipelines shown)               │
   └─────────────────────────────────────┘
```

## Animation Notes

- Filter button state changes are instant (no animation)
- Dropdown appears/disappears instantly (no fade)
- Pipeline list updates instantly when filter changes
- Keyboard focus moves instantly between filters
- All interactions feel snappy and responsive
