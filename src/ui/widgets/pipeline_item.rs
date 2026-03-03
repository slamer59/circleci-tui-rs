/// Pipeline item rendering
use crate::models::Pipeline;
use crate::theme::{get_status_color, get_status_icon, ACCENT, FG_DIM, FG_PRIMARY};
use crate::ui::utils::truncate_string;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

/// Render a pipeline item as a vector of Lines
///
/// Format: [icon] [commit_message:50chars] [branch] [sha:7] [author]
///
/// # Arguments
/// * `pipeline` - The pipeline data to render
/// * `selected` - Whether this item is currently selected
pub fn render_pipeline_item<'a>(pipeline: &'a Pipeline, selected: bool) -> Vec<Line<'a>> {
    let icon = get_status_icon(&pipeline.status);
    let status_col = get_status_color(&pipeline.status);

    // Truncate commit message to 50 chars
    let commit_msg = truncate_string(&pipeline.commit_msg, 50);

    // Truncate SHA to 7 chars
    let short_sha = if pipeline.sha.len() > 7 {
        &pipeline.sha[..7]
    } else {
        &pipeline.sha
    };

    let line = if selected {
        Line::from(vec![
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(status_col).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{:<50} ", commit_msg),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}] ", pipeline.branch),
                Style::default().fg(ACCENT),
            ),
            Span::styled(format!("{} ", short_sha), Style::default().fg(FG_DIM)),
            Span::styled(&pipeline.author, Style::default().fg(FG_DIM)),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!("{} ", icon), Style::default().fg(status_col)),
            Span::styled(
                format!("{:<50} ", commit_msg),
                Style::default().fg(FG_PRIMARY),
            ),
            Span::styled(
                format!("[{}] ", pipeline.branch),
                Style::default().fg(FG_DIM),
            ),
            Span::styled(format!("{} ", short_sha), Style::default().fg(FG_DIM)),
            Span::styled(&pipeline.author, Style::default().fg(FG_DIM)),
        ])
    };

    vec![line]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_pipeline_item() {
        let pipeline = Pipeline {
            id: "123".to_string(),
            branch: "main".to_string(),
            commit_msg: "Fix bug".to_string(),
            author: "Alice".to_string(),
            status: "success".to_string(),
            sha: "abcdef1234567890".to_string(),
            trigger: "push".to_string(),
            created_at: "2024-01-01".to_string(),
        };

        let lines = render_pipeline_item(&pipeline, false);
        assert_eq!(lines.len(), 1);
    }
}
