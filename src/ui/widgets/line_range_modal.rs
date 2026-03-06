use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::ui::widgets::text_input::TextInput;
use crate::theme::{BORDER_FOCUSED, BG_PANEL, FG_BRIGHT};

pub enum LineRangeAction {
    None,
    Confirm(String), // Returns the input string (e.g., "1,1000")
    Cancel,
}

pub struct LineRangeModal {
    visible: bool,
    input: TextInput,
    total_lines: usize,
}

impl LineRangeModal {
    pub fn new() -> Self {
        let mut input = TextInput::new("1,1000");
        input.set_focused(true);

        Self {
            visible: false,
            input,
            total_lines: 0,
        }
    }

    pub fn show(&mut self, total_lines: usize) {
        self.visible = true;
        self.total_lines = total_lines;
        self.input.set_focused(true);

        // Set default to last 1000 lines (or all if fewer than 1000)
        let default_value = if total_lines <= 1000 {
            "%".to_string()  // All lines
        } else {
            let start = total_lines - 999;  // Last 1000 lines
            format!("{},$", start)
        };
        self.input.set_value(default_value);
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.input.set_focused(false);
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> LineRangeAction {
        match key.code {
            KeyCode::Esc => {
                self.hide();
                LineRangeAction::Cancel
            }
            KeyCode::Enter => {
                let value = self.input.value().to_string();
                self.hide();
                LineRangeAction::Confirm(value)
            }
            _ => {
                // Pass to text input
                self.input.handle_key(key.code);
                LineRangeAction::None
            }
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let modal_area = centered_rect(50, 30, area);

        // Clear background
        f.render_widget(Clear, modal_area);

        // Create block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .style(Style::default().bg(BG_PANEL))
            .title(" Copy Log Lines ")
            .title_style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD));

        let inner_area = block.inner(modal_area);
        f.render_widget(block, modal_area);

        // Layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Instructions
                Constraint::Length(1), // Spacing
                Constraint::Length(3), // Input
                Constraint::Length(1), // Spacing
                Constraint::Length(2), // Help text
            ])
            .split(inner_area);

        // Instructions (Vim-style)
        let instructions = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("Enter range: "),
                Span::styled("1,1000", Style::default().fg(Color::Cyan)),
                Span::raw(" or "),
                Span::styled("100,$", Style::default().fg(Color::Cyan)),
                Span::raw(" ($ = end)"),
            ]),
            Line::from(vec![
                Span::styled("%", Style::default().fg(Color::Cyan)),
                Span::raw(" for all lines, or just "),
                Span::styled("1000", Style::default().fg(Color::Cyan)),
                Span::raw(" for lines 1-1000"),
            ]),
        ]);
        f.render_widget(instructions, chunks[0]);

        // Input field
        self.input.render(f, chunks[2]);

        // Help text
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" to copy | "),
            Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" to cancel | Total lines: "),
            Span::styled(
                self.total_lines.to_string(),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ),
        ]));
        f.render_widget(help, chunks[4]);
    }
}

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
