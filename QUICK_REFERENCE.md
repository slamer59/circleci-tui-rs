# CircleCI TUI - Quick Reference Card

## New Features Added

### 1. Status Messages (Top of Screen)
- **Green** ✓ - Success messages
- **Blue** ℹ - Info messages
- **Red** ✗ - Error messages
- Auto-hide after 5 seconds

### 2. Help Modal
Press **?** anywhere to show complete keyboard shortcuts

### 3. Better Empty States
- Friendly ASCII art emojis
- Clear explanations
- Actionable hints

### 4. Enhanced Loading
- Context: "Loading pipelines from CircleCI..."
- Elapsed time for long operations
- Cancel hint: "Press Esc to cancel"

### 5. Smart Text Filtering
- 300ms debounce delay
- Shows "(filtering...)" while typing
- Reduces lag on large lists

### 6. Better Error Messages
All errors now include:
- Clear explanation
- Suggestions to fix
- Links to documentation

## Keyboard Shortcuts

### Global Commands
| Key | Action |
|-----|--------|
| `q` | Quit application |
| `?` | Show help modal |
| `Esc` | Go back / Cancel |

### Pipeline List Screen
| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate pipelines |
| `Enter` | Open selected pipeline |
| `/` | Start text filter |
| `Tab` | Cycle through filters |
| `Space` | Toggle branch/status filter |
| `Backspace` | Delete filter character |
| `r` | Refresh pipelines |
| `Esc` | Clear all filters |

### Pipeline Detail Screen
| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate in current panel |
| `Tab` | Switch between workflows and jobs |
| `Enter` | View job logs |
| `f` | Toggle "failed jobs only" filter |
| `l` | Load more jobs (if available) |
| `R` | Rerun workflow (requires confirmation) |
| `Esc` | Go back to pipeline list |

### Modals
| Key | Action |
|-----|--------|
| `Esc` | Close modal |
| `?` | Close help modal |
| `↑` / `↓` | Scroll in log viewer |
| `y` / `Enter` | Confirm action |
| `n` | Cancel action |

## Empty State Messages

### No Pipelines
```
(╯°□°)╯︵ ┻━┻
No pipelines found
```
**Actions**: Press 'r' to refresh or 'Esc' to clear filters

### No Workflows
```
¯\_(ツ)_/¯
No workflows found
```
**Actions**: Press 'Esc' to go back

### No Jobs
```
¯\_(ツ)_/¯
No jobs found
```

### Filtered Jobs (none match)
```
(•_•)
No jobs match filters
```
**Actions**: Press 'f' to toggle filters or 'Tab' to switch panel

## Common Error Solutions

### Authentication Error (401/403)
1. Check `CIRCLECI_TOKEN` in `.env` file
2. Verify token has required permissions
3. Generate new token at: https://app.circleci.com/settings/user/tokens

### Not Found (404)
1. Verify `PROJECT_SLUG` in `.env` file (format: `gh/owner/repo`)
2. Check you have access to the project
3. Verify pipeline/workflow/job ID is correct

### Network Error
1. Check internet connection
2. Verify CircleCI service status
3. Check firewall settings

### Rate Limit (429)
1. Wait a few minutes before retrying
2. Consider reducing polling frequency
3. Check if multiple instances are running

### Request Timeout
1. Check internet connection speed
2. Try again (CircleCI may be slow)
3. Check if VPN is affecting connection

## Visual Indicators

### Border Colors
- **Bright Magenta** - Active/focused panel
- **Gray** - Inactive panel

### Status Colors
- **Green** ✓ - Success/passed
- **Red** ✗ - Failed/error
- **Magenta** ● - Running/in progress
- **Coral** ◆ - Blocked/waiting
- **Gray** ○ - Pending/queued
- **Gray** ◌ - Canceled

### Filter Indicators
- **Bold Magenta** - Active filter field
- **"(filtering...)"** - Debounce in progress
- **Underlined** - Selected filter checkbox

## Tips & Tricks

1. **Fast Navigation**: Use `Tab` to quickly switch between panels
2. **Quick Filter**: Press `/` to immediately start filtering
3. **Failed Jobs**: Press `f` to show only failed jobs
4. **Bulk Load**: Press `l` repeatedly to load all paginated jobs
5. **Fresh Start**: Press `Esc` to clear all filters at once
6. **Help Always**: Press `?` anytime you forget a shortcut
7. **Status Check**: Look at the top bar for operation status
8. **Cancel Operations**: Press `Esc` during loading to cancel

## Performance Notes

- Text filter has 300ms debounce (type freely!)
- Status messages auto-clear after 5 seconds
- Large lists (1000+ items) may take a moment to filter
- Pagination helps with workflows with many jobs

## File Locations

### Configuration
- `.env` - CircleCI token and project slug

### Generated Files
- `UI_IMPROVEMENTS.md` - Detailed implementation documentation
- `QUICK_REFERENCE.md` - This file

## Need More Help?

1. Press `?` in the app for full keyboard shortcuts
2. Check error messages for specific solutions
3. Review `.env.example` for configuration format
4. Verify CircleCI token permissions online
