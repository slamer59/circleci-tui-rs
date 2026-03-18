//! SSH modal widget
//!
//! This module provides a modal popup for displaying SSH commands for debugging jobs.

use crate::api::models::Job;
use crate::theme::{ACCENT, BG_PANEL, BORDER_FOCUSED, FG_BRIGHT, FG_DIM, FG_PRIMARY};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Actions that can be returned from the SSH modal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshAction {
    /// No action taken yet
    None,
    /// User closed the modal (pressed Esc/Enter/c)
    Close,
}

/// Modal popup for displaying SSH commands
pub struct SshModal {
    /// The job for SSH access
    job: Job,
    /// The SSH command to display
    ssh_command: String,
    /// Whether the modal is visible
    visible: bool,
}

impl SshModal {
    /// Create a new SSH modal for a job
    ///
    /// # Arguments
    ///
    /// * `job` - The job to SSH into
    ///
    /// # Examples
    ///
    /// ```
    /// let modal = SshModal::new(job);
    /// ```
    pub fn new(job: Job) -> Self {
        // Generate SSH command
        // Format: ssh -p PORT user@host
        // In CircleCI, the format is: ssh -p 64535 {job_number}-{hash}@{host}.circleci.com
        let ssh_command = format!(
            "ssh -p 64535 {}-90db2e@{}.circleci.com",
            job.job_number,
            &job.id[..8.min(job.id.len())]
        );

        Self {
            job,
            ssh_command,
            visible: true,
        }
    }

    /// Handle keyboard input
    ///
    /// # Arguments
    ///
    /// * `key` - The key event to handle
    ///
    /// # Returns
    ///
    /// An SshAction indicating what action should be taken
    pub fn handle_input(&mut self, key: KeyEvent) -> SshAction {
        match key.code {
            KeyCode::Char('c') | KeyCode::Enter | KeyCode::Esc => SshAction::Close,
            _ => SshAction::None,
        }
    }

    /// Render the modal to the frame
    ///
    /// # Arguments
    ///
    /// * `f` - The frame to render to
    /// * `area` - The area to render within
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate centered modal area (60% width, 40% height)
        let modal_area = centered_rect(60, 40, area);

        // Clear the background (dimmed effect)
        f.render_widget(Clear, modal_area);

        // Create the main block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .style(Style::default().bg(BG_PANEL))
            .title(" SSH INTO JOB ")
            .title_style(Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD));

        let inner_area = block.inner(modal_area);
        f.render_widget(block, modal_area);

        // Split into sections: title, command, hint, buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(4), // SSH command box
                Constraint::Length(3), // Hint
                Constraint::Length(3), // Buttons
            ])
            .split(inner_area);

        // Render title
        self.render_title(f, chunks[0]);

        // Render SSH command
        self.render_ssh_command(f, chunks[1]);

        // Render hint
        self.render_hint(f, chunks[2]);

        // Render buttons
        self.render_buttons(f, chunks[3]);
    }

    /// Render the title section
    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title_lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                &self.job.name,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
        ];

        let title_paragraph = Paragraph::new(title_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(title_paragraph, area);
    }

    /// Render the SSH command section
    fn render_ssh_command(&self, f: &mut Frame, area: Rect) {
        let command_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(BG_PANEL));

        let inner = command_block.inner(area);
        f.render_widget(command_block, area);

        let command_line = Line::from(vec![Span::styled(
            &self.ssh_command,
            Style::default().fg(FG_BRIGHT).add_modifier(Modifier::BOLD),
        )]);

        let command_paragraph =
            Paragraph::new(vec![Line::from(""), command_line]).alignment(Alignment::Center);

        f.render_widget(command_paragraph, inner);
    }

    /// Render the hint section
    fn render_hint(&self, f: &mut Frame, area: Rect) {
        let hint_lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Copy the command above and paste it in your terminal",
                Style::default().fg(FG_DIM),
            )]),
        ];

        let hint_paragraph = Paragraph::new(hint_lines).alignment(Alignment::Center);

        f.render_widget(hint_paragraph, area);
    }

    /// Render the buttons section
    fn render_buttons(&self, f: &mut Frame, area: Rect) {
        let buttons = Line::from(vec![
            Span::styled("[", Style::default().fg(FG_PRIMARY)),
            Span::styled(
                "C",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "lose]",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled("[", Style::default().fg(FG_PRIMARY)),
            Span::styled("Esc", Style::default().fg(FG_DIM)),
            Span::styled("]", Style::default().fg(FG_PRIMARY)),
        ]);

        let buttons_paragraph =
            Paragraph::new(vec![Line::from(""), buttons]).alignment(Alignment::Center);

        f.render_widget(buttons_paragraph, area);
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
    use crate::api::models::ExecutorInfo;
    use chrono::Utc;

    fn create_test_job() -> Job {
        Job {
            id: "test-job-id-12345".to_string(),
            name: "test-job".to_string(),
            status: "success".to_string(),
            job_number: 123,
            workflow_id: "test-workflow-id".to_string(),
            started_at: Some(Utc::now()),
            stopped_at: Some(Utc::now()),
            duration: Some(60),
            executor: ExecutorInfo {
                executor_type: "docker".to_string(),
            },
        }
    }

    #[test]
    fn test_ssh_modal_creation() {
        let job = create_test_job();
        let modal = SshModal::new(job);
        assert!(modal.is_visible());
        assert!(modal.ssh_command.contains("ssh -p 64535"));
        assert!(modal.ssh_command.contains("123-90db2e@"));
    }

    #[test]
    fn test_ssh_modal_input() {
        let job = create_test_job();
        let mut modal = SshModal::new(job);

        // Test Close action
        let action = modal.handle_input(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(action, SshAction::Close);

        // Test Enter (Close)
        let action = modal.handle_input(KeyEvent::from(KeyCode::Enter));
        assert_eq!(action, SshAction::Close);

        // Test Esc (Close)
        let action = modal.handle_input(KeyEvent::from(KeyCode::Esc));
        assert_eq!(action, SshAction::Close);
    }

    #[test]
    fn test_ssh_modal_hide() {
        let job = create_test_job();
        let mut modal = SshModal::new(job);
        assert!(modal.is_visible());

        modal.hide();
        assert!(!modal.is_visible());
    }

    #[test]
    fn test_ssh_command_format() {
        let job = create_test_job();
        let modal = SshModal::new(job);

        // Check that SSH command has correct format
        assert_eq!(
            modal.ssh_command,
            "ssh -p 64535 123-90db2e@test-job.circleci.com"
        );
    }
}
