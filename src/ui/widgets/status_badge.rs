/// Status badge helper functions
///
/// This module provides convenience re-exports of status badge functions from the theme module.
pub use crate::theme::get_status_icon;

/// Format a status badge with icon and text
pub fn format_status_badge(status: &str) -> String {
    format!("{} {}", get_status_icon(status), status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{BLOCKED, FAILED, PENDING, RUNNING, SUCCESS};

    #[test]
    fn test_status_icons() {
        assert_eq!(get_status_icon("running"), "●");
        assert_eq!(get_status_icon("success"), "✓");
        assert_eq!(get_status_icon("failed"), "✗");
        assert_eq!(get_status_icon("blocked"), "◆");
        assert_eq!(get_status_icon("pending"), "○");
    }

    #[test]
    fn test_status_colors() {
        assert_eq!(get_status_color("running"), RUNNING);
        assert_eq!(get_status_color("success"), SUCCESS);
        assert_eq!(get_status_color("failed"), FAILED);
        assert_eq!(get_status_color("blocked"), BLOCKED);
        assert_eq!(get_status_color("pending"), PENDING);
    }

    #[test]
    fn test_format_badge() {
        assert_eq!(format_status_badge("running"), "● running");
        assert_eq!(format_status_badge("success"), "✓ success");
    }
}
