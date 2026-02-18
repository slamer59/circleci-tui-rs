/// Breadcrumb navigation rendering
use crate::theme::FG_DIM;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

/// Render a breadcrumb navigation from path segments
///
/// # Arguments
/// * `segments` - Array of path segments to join with " › " separator
///
/// # Returns
/// A Paragraph widget with the breadcrumb trail
///
/// # Example
/// ```
/// let breadcrumb = render_breadcrumb(&["Home", "Pipelines", "main"]);
/// ```
pub fn render_breadcrumb<'a>(segments: &[&'a str]) -> Paragraph<'a> {
    let breadcrumb_text = segments.join(" › ");
    let line = Line::from(vec![
        Span::styled(breadcrumb_text, Style::default().fg(FG_DIM)),
    ]);

    Paragraph::new(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_breadcrumb_single() {
        let breadcrumb = render_breadcrumb(&["Home"]);
        // Test that it creates a valid Paragraph
        // We can't directly test the content without rendering, but we can ensure it doesn't panic
    }

    #[test]
    fn test_render_breadcrumb_multiple() {
        let breadcrumb = render_breadcrumb(&["Home", "Pipelines", "main"]);
        // Test that it creates a valid Paragraph
    }

    #[test]
    fn test_render_breadcrumb_empty() {
        let breadcrumb = render_breadcrumb(&[]);
        // Test that it handles empty array gracefully
    }
}
