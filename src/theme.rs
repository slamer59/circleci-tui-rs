//! CircleCI TUI Theme - Modern Cyberpunk Aesthetic
//!
//! This module provides a comprehensive color palette inspired by gitui/glim,
//! featuring a cyberpunk aesthetic with magenta accents and dark backgrounds.
//!
//! Color Palette:
//! - Primary Accent (Bright Magenta): #f71abd
//! - Dark Accent (Deep Brown-Red): #370e0c
//! - Light Accent (Soft Pink): #f7a8b3
//! - Coral Accent (Warm Red): #eb636b
//! - Background: #0b0e14 (dark background)
//! - Panel backgrounds: #1e2230, #12161e (elevated surfaces)
//! - Borders: #2c313a, #3e4451 (subtle borders)
//! - Text: #abb2bf (primary), #ffffff (high contrast)
//! - Success: #98c379 (green for passed status)
//! - Dim text: #5c6370

use ratatui::style::Color;

// =============================================================================
// Background Colors
// =============================================================================

/// Primary dark background color
pub const BG_DARK: Color = Color::Rgb(11, 14, 20); // #0b0e14

/// Elevated panel background
pub const BG_PANEL: Color = Color::Rgb(30, 34, 48); // #1e2230

// /// Highlight background for active elements
// pub const BG_HIGHLIGHT: Color = Color::Rgb(18, 22, 30); // #12161e

// /// Slightly lighter background for hover states
// pub const BG_HOVER: Color = Color::Rgb(24, 28, 38); // #181c26

// /// Deep background for input fields
// pub const BG_INPUT: Color = Color::Rgb(15, 18, 25); // #0f1219

// =============================================================================
// Foreground Colors
// =============================================================================

/// Primary text color
pub const FG_PRIMARY: Color = Color::Rgb(171, 178, 191); // #abb2bf

/// Bright text for high contrast
pub const FG_BRIGHT: Color = Color::Rgb(255, 255, 255); // #ffffff

/// Dimmed text for secondary information
pub const FG_DIM: Color = Color::Rgb(92, 99, 112); // #5c6370

// =============================================================================
// Status Colors
// =============================================================================

/// Success status (green) - used for "success", "passed" states
pub const SUCCESS: Color = Color::Rgb(152, 195, 121); // #98c379

/// Failed status (deep brown-red) - used for "failed", "error" states
pub const FAILED: Color = Color::Rgb(55, 14, 12); // #370e0c

/// Failed text color (soft pink) - used for error message text
pub const FAILED_TEXT: Color = Color::Rgb(247, 168, 179); // #f7a8b3

/// Running status (bright magenta) - used for "running", "in_progress" states
pub const RUNNING: Color = Color::Rgb(247, 26, 189); // #f71abd

/// Blocked status (coral) - used for "blocked", "waiting" states
pub const BLOCKED: Color = Color::Rgb(235, 99, 107); // #eb636b

/// Pending status (dim) - used for "pending", "queued" states
pub const PENDING: Color = Color::Rgb(92, 99, 112); // #5c6370

/// Canceled status (dim) - used for "canceled", "aborted" states
pub const CANCELED: Color = Color::Rgb(92, 99, 112); // #5c6370

// =============================================================================
// Accent Colors
// =============================================================================

/// Primary accent color (bright magenta) - will become Kraken Primary #F050F8
pub const ACCENT: Color = Color::Rgb(247, 26, 189); // #f71abd

/// Secondary accent color (cyan/light blue) - Voltage brand color for selected items
pub const SECONDARY: Color = Color::Rgb(96, 240, 248); // #60F0F8

// /// Dimmed accent color (soft pink)
// pub const ACCENT_DIM: Color = Color::Rgb(247, 168, 179); // #f7a8b3

/// Warning accent color (coral)
pub const ACCENT_WARN: Color = Color::Rgb(235, 99, 107); // #eb636b

// =============================================================================
// Border Colors
// =============================================================================

/// Default border color
pub const BORDER: Color = Color::Rgb(44, 49, 58); // #2c313a

/// Focused border color
pub const BORDER_FOCUSED: Color = Color::Rgb(247, 26, 189); // #f71abd

// /// Subtle border color for nested elements
// pub const BORDER_SUBTLE: Color = Color::Rgb(62, 68, 81); // #3e4451

// =============================================================================
// Helper Functions
// =============================================================================

/// Get the appropriate color for a given status string.
///
/// # Arguments
///
/// * `status` - The status string (e.g., "success", "failed", "running")
///
/// # Returns
///
/// The corresponding color for the status. Returns PENDING color for unknown statuses.
///
/// # Examples
///
/// ```
/// use circleci_tui_rs::theme::get_status_color;
/// use ratatui::style::Color;
///
/// let color = get_status_color("success");
/// assert_eq!(color, Color::Rgb(152, 195, 121));
/// ```
pub fn get_status_color(status: &str) -> Color {
    match status.to_lowercase().as_str() {
        "success" | "passed" | "fixed" | "successful" => SUCCESS,
        "failed" | "error" | "failure" => FAILED,
        "running" | "in_progress" | "in-progress" => RUNNING,
        "blocked" | "waiting" | "on_hold" | "on-hold" => BLOCKED,
        "pending" | "queued" | "not_run" | "not-run" | "not_running" => PENDING,
        "canceled" | "cancelled" | "aborted" | "terminated" => CANCELED,
        _ => PENDING,
    }
}

/// Get the appropriate icon for a given status string.
///
/// # Arguments
///
/// * `status` - The status string (e.g., "success", "failed", "running")
///
/// # Returns
///
/// A Unicode icon character representing the status.
///
/// # Examples
///
/// ```
/// use circleci_tui_rs::theme::get_status_icon;
///
/// let icon = get_status_icon("success");
/// assert_eq!(icon, "✓");
/// ```
pub fn get_status_icon(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        "success" | "passed" | "fixed" | "successful" => "✓",
        "failed" | "error" | "failure" => "✗",
        "running" | "in_progress" | "in-progress" => "●",
        "blocked" | "waiting" | "on_hold" | "on-hold" => "◆",
        "pending" | "queued" | "not_run" | "not-run" | "not_running" => "○",
        "canceled" | "cancelled" | "aborted" | "terminated" => "◌",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_status_color_success() {
        assert_eq!(get_status_color("success"), SUCCESS);
        assert_eq!(get_status_color("passed"), SUCCESS);
        assert_eq!(get_status_color("successful"), SUCCESS);
    }

    #[test]
    fn test_get_status_color_failed() {
        assert_eq!(get_status_color("failed"), FAILED);
        assert_eq!(get_status_color("error"), FAILED);
        assert_eq!(get_status_color("failure"), FAILED);
    }

    #[test]
    fn test_get_status_color_running() {
        assert_eq!(get_status_color("running"), RUNNING);
        assert_eq!(get_status_color("in_progress"), RUNNING);
        assert_eq!(get_status_color("in-progress"), RUNNING);
    }

    #[test]
    fn test_get_status_color_case_insensitive() {
        assert_eq!(get_status_color("SUCCESS"), SUCCESS);
        assert_eq!(get_status_color("Failed"), FAILED);
        assert_eq!(get_status_color("RUNNING"), RUNNING);
    }

    #[test]
    fn test_get_status_icon() {
        assert_eq!(get_status_icon("success"), "✓");
        assert_eq!(get_status_icon("failed"), "✗");
        assert_eq!(get_status_icon("running"), "●");
        assert_eq!(get_status_icon("blocked"), "◆");
        assert_eq!(get_status_icon("pending"), "○");
        assert_eq!(get_status_icon("canceled"), "◌");
        assert_eq!(get_status_icon("unknown"), "?");
    }
}
