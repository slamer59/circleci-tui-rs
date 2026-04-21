//! Help modal widget showing keyboard shortcuts
//!
//! This module provides a modal overlay that displays all available keyboard shortcuts
//! organized by screen and context.

use crate::theme::{ACCENT, BG_PANEL, BORDER_FOCUSED, FG_BRIGHT, FG_DIM, FG_PRIMARY};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// ASCII logo lines - easy to modify
const LOGO_LINES: &[&str] = &[
    "                                     ,,,,,                                      ",
    "                                   ,,;~~;; ,,                                   ",
    "                                 ,'';+~~~~~~'',                                 ",
    "                                ,'',~~~;;;~~,'',                                ",
    "                                ,''',;;;;;;,''',                                ",
    "                      ;;        ,'''',    ,'''',     ;;;        ;               ",
    "              ;     ;;;;~; ;;;   ', ,,'''',,,,',  ;;~ ;    ;;;;                 ",
    "                       ;;    ~;    ;~;,,,,;~;;   ;~; , ;                        ",
    "                      ;    ;;    ;,. ~    ~ .,;   ,;;  ;                        ",
    "           ;;;;;     ;,,  ~ '    ,            ,    ' +         ;;;;;;;;         ",
    "        ;;;;;;;;;;; ;~,,  ;;,,,,,,     ;;     ,,,,,,;~       ;;;     ;;;        ",
    "       ~~;      ;;;  ;;,,  ;;;  ;  ;;  ~ ; ,   ;; ;~~    ;;;;;        ; ;       ",
];

/// Help modal action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpAction {
    /// No action
    None,
    /// Close the help modal
    Close,
}

/// Help modal showing keyboard shortcuts
pub struct HelpModal {
    /// Whether the modal is visible
    visible: bool,
}

impl HelpModal {
    /// Create a new help modal
    pub fn new() -> Self {
        Self { visible: true }
    }

    /// Handle keyboard input
    pub fn handle_input(&mut self, key: KeyEvent) -> HelpAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => HelpAction::Close,
            _ => HelpAction::None,
        }
    }

    /// Render the help modal
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate centered modal area (80% width, 80% height)
        let modal_area = centered_rect(80, 80, area);

        // Clear the background
        f.render_widget(Clear, modal_area);

        // Create the main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .style(Style::default().bg(BG_PANEL))
            .title(" KEYBOARD SHORTCUTS ")
            .title_style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD));

        let inner_area = block.inner(modal_area);
        f.render_widget(block, modal_area);

        // Create help content
        let mut help_lines = vec![];

        // === LOGO ASCII (easy to modify - see LOGO_LINES constant at top) ===
        for line in LOGO_LINES.iter() {
            help_lines.push(Line::from(*line).alignment(Alignment::Center));
        }
        help_lines.push(Line::from(""));
        help_lines.push(
            Line::from(Span::styled(
                "CircleCI TUI",
                Style::default()
                    .fg(FG_PRIMARY)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            ))
            .alignment(Alignment::Center),
        );
        help_lines.push(Line::from(""));
        // === FIN LOGO ===

        help_lines.extend(vec![
            Line::from(Span::styled(
                "GLOBAL SHORTCUTS",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "  q",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Quit application",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  ?",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Show/hide this help",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Esc",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("         Go back / Cancel", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(""),
            // Navigation
            Line::from(Span::styled(
                "NAVIGATION",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "  ↑/↓",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "         Move selection up/down",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Enter",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("       Select / Open", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Tab",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "         Switch focus / Cycle filters",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(""),
            // Pipeline Screen
            Line::from(Span::styled(
                "PIPELINE SCREEN",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "  /",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Activate text filter",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  r",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Refresh pipelines",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Space",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "       Toggle branch/status filter",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Backspace",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "   Delete filter character",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(""),
            // Pipeline Detail Screen
            Line::from(Span::styled(
                "PIPELINE DETAIL SCREEN",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "  w",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Focus Workflows panel",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  j",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Focus Jobs panel",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  f",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Focus Filter panel",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Tab",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "         Cycle between panels",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Enter",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("       View job logs", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  l",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Load more jobs (pagination)",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  R",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("           Rerun workflow", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  c",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Copy job logs (opens range selector)",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  s",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("           SSH into job", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  e",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "           Export failing job logs",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(""),
            // Modals
            Line::from(Span::styled(
                "MODALS",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled(
                    "  Esc",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled("         Close modal", Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  ↑/↓",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "         Scroll log viewer",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  y/n",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "         Confirm/Cancel in confirmation dialog",
                    Style::default().fg(FG_PRIMARY),
                ),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "Press ? or Esc to close",
                Style::default().fg(FG_DIM).add_modifier(Modifier::ITALIC),
            ))
            .alignment(Alignment::Center),
        ]);

        let help_paragraph = Paragraph::new(help_lines)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left);

        f.render_widget(help_paragraph, inner_area);
    }
}

impl Default for HelpModal {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_modal_creation() {
        let modal = HelpModal::new();
        assert!(modal.is_visible());
    }

    #[test]
    fn test_help_modal_input() {
        let mut modal = HelpModal::new();

        // Test close actions
        let action = modal.handle_input(KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, HelpAction::Close);

        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('?')));
        assert_eq!(action, HelpAction::Close);

        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('q')));
        assert_eq!(action, HelpAction::Close);

        // Test other keys
        let action = modal.handle_input(KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, HelpAction::None);
    }
}
