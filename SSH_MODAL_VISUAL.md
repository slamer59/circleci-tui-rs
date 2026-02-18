# SSH Modal Visual Reference

## Modal Appearance

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                         ╔═══════════════════════════════════╗       │
│                         ║     SSH INTO JOB                 ║       │
│                         ╠═══════════════════════════════════╣       │
│                         ║                                   ║       │
│                         ║         test-job-name             ║       │
│                         ║                                   ║       │
│                         ║ ┌───────────────────────────────┐ ║       │
│                         ║ │                               │ ║       │
│                         ║ │ ssh -p 64535 123-90db2e@      │ ║       │
│                         ║ │   test-job.circleci.com       │ ║       │
│                         ║ │                               │ ║       │
│                         ║ └───────────────────────────────┘ ║       │
│                         ║                                   ║       │
│                         ║  Copy the command above and       ║       │
│                         ║  paste it in your terminal        ║       │
│                         ║                                   ║       │
│                         ║       [Close]  [Esc]             ║       │
│                         ║                                   ║       │
│                         ╚═══════════════════════════════════╝       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Color Scheme (Cyberpunk Theme)

- **Title Bar**: Bright foreground text with bold modifier
- **Job Name**: Accent color (#FF2E97) with bold modifier
- **SSH Command Box**:
  - Border: Accent color (#FF2E97)
  - Background: Panel background (#0D0D19)
  - Text: Bright foreground (#E0E0FF) with bold modifier
- **Hint Text**: Dim foreground color (#666680)
- **Close Button**:
  - 'C' in accent color with bold
  - 'Esc' in dim foreground
- **Modal Border**: Focused border color (#FF2E97)
- **Modal Background**: Panel background (#0D0D19)

## Layout Specifications

### Modal Dimensions
- **Width**: 60% of screen width
- **Height**: 40% of screen height
- **Position**: Centered on screen

### Internal Layout (Vertical)
1. **Title Section**: 3 lines
   - Empty line
   - Job name (centered, accent color)
   - Empty line

2. **SSH Command Box**: 4 lines
   - Bordered box with accent color
   - Centered SSH command text
   - Bold and bright styling

3. **Hint Section**: 3 lines
   - Empty line
   - Hint text (centered, dim color)
   - Empty line

4. **Buttons Section**: 3 lines
   - Empty line
   - Buttons (centered)
   - Empty line

## User Interaction Flow

```
Pipeline Detail Screen
         │
         │ User selects job
         │ User presses 's'
         ↓
   SSH Modal Opens
         │
         │ Shows SSH command
         │ User copies command
         │
         ↓
   User presses Esc/Enter/'c'
         │
         ↓
   Modal Closes
         │
         ↓
   Returns to Pipeline Detail
```

## Example SSH Commands

For different jobs, the SSH command format is:

```bash
# Job #123 with ID "abc12345..."
ssh -p 64535 123-90db2e@abc12345.circleci.com

# Job #456 with ID "def67890..."
ssh -p 64535 456-90db2e@def67890.circleci.com

# Job #789 with ID "ghi13579..."
ssh -p 64535 789-90db2e@ghi13579.circleci.com
```

The format is always: `ssh -p 64535 {job_number}-90db2e@{first_8_chars_of_job_id}.circleci.com`

## Context in Application

### When SSH Modal is Available
- User must be on the Pipeline Detail Screen (Screen 2)
- User must have focus on the Jobs panel (right panel)
- User must have a job selected

### Keyboard Shortcuts
- **Open**: Press 's' (when job is selected)
- **Close**: Press 'Esc', 'Enter', or 'c'

### Footer Display
When on Pipeline Detail Screen with Jobs panel focused:
```
[↑↓] Nav  [Tab] Switch  [⏎] View Logs  [s] SSH  [f] Toggle Filters  [?] Help  [Esc] Back
```

## Implementation Details

### Modal Priority
The SSH modal is rendered with the following priority order (lowest to highest):
1. Log modal
2. **SSH modal** ← Current modal
3. Confirmation modal
4. Error modal
5. Help modal

This ensures proper layering when multiple modals could be shown.

### State Management
- Modal state is stored in `App.ssh_modal: Option<SshModal>`
- Modal is created when 's' key is pressed on a selected job
- Modal is destroyed when closed or when navigating away from detail screen
- No clipboard integration - users manually copy from terminal

## Testing Checklist

- [ ] Open modal from Pipeline Detail screen
- [ ] SSH command displays correctly for different jobs
- [ ] Job name displays in title
- [ ] Close modal with 'Esc' key
- [ ] Close modal with 'Enter' key
- [ ] Close modal with 'c' key
- [ ] Modal centers properly on different screen sizes
- [ ] Modal styling matches cyberpunk theme
- [ ] Modal closes when navigating back to Pipeline screen
- [ ] Footer shows 's' key hint when on Jobs panel
- [ ] Modal blocks interaction with underlying screen
