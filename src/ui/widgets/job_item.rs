/// Job item rendering
use crate::models::Job;
use crate::theme::{get_status_color, get_status_icon, ACCENT, FG_DIM, FG_PRIMARY};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Render a job item as a vector of Lines
///
/// Format: [icon] [name:30chars] [executor] [duration]
///
/// # Arguments
/// * `job` - The job data to render
/// * `selected` - Whether this item is currently selected
pub fn render_job_item<'a>(job: &'a Job, selected: bool) -> Vec<Line<'a>> {
    let icon = get_status_icon(&job.status);
    let status_col = get_status_color(&job.status);

    // Truncate job name to 30 chars
    let job_name = truncate_string(&job.name, 30);

    let line = if selected {
        Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(status_col).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<30} ", job_name), Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("[{}] ", job.executor), Style::default().fg(ACCENT)),
            Span::styled(&job.duration, Style::default().fg(ACCENT)),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
            Span::styled(format!("{:<30} ", job_name), Style::default().fg(FG_PRIMARY)),
            Span::styled(format!("[{}] ", job.executor), Style::default().fg(FG_DIM)),
            Span::styled(&job.duration, Style::default().fg(FG_DIM)),
        ])
    };

    vec![line]
}

/// Truncate a string to a maximum length, adding "..." if truncated
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long job name", 10), "this is...");
    }

    #[test]
    fn test_render_job_item() {
        let job = Job {
            id: "456".to_string(),
            name: "test-job".to_string(),
            status: "success".to_string(),
            duration: "2m 30s".to_string(),
            executor: "docker".to_string(),
            step: "test".to_string(),
            job_number: Some(123),
            ssh_enabled: false,
        };

        let lines = render_job_item(&job, false);
        assert_eq!(lines.len(), 1);
    }
}
