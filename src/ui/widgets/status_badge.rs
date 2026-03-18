//! Status badge helper functions
//!
//! This module provides convenience re-exports of status badge functions from the theme module.

#[cfg(test)]
mod tests {
    use crate::theme::{
        get_status_color, get_status_icon, BLOCKED, FAILED, PENDING, RUNNING, SUCCESS,
    };

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
}
