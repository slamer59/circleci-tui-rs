//! Faceted Search Widget
//!
//! A generic, reusable faceted search bar widget that can be used across different screens
//! (pipelines, workflows, jobs) to provide filtering functionality.
//!
//! # Features
//!
//! - Multiple filter facets with customizable icons and options
//! - Visual states: inactive → focused → pressed → filtered
//! - Dropdown menus with keyboard navigation
//! - Checkmarks for selected options
//! - Dynamic button labels that update with selection
//! - Generic design that works with any data type
//!
//! # Example
//!
//! ```rust,no_run
//! use circleci_tui_rs::ui::widgets::faceted_search::{Facet, FacetedSearchBar};
//!
//! let facets = vec![
//!     Facet::new("☍", vec!["All pipelines".to_string(), "My pipelines".to_string()], 0),
//!     Facet::new("●", vec!["All statuses".to_string(), "Success".to_string(), "Failed".to_string()], 0),
//! ];
//!
//! let mut search_bar = FacetedSearchBar::new(facets);
//! ```

use crate::theme;
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Represents a single filter facet (dimension) with multiple options
#[derive(Clone)]
pub struct Facet {
    /// Display name (updates when selection changes)
    name: String,
    /// Icon for visual identification
    icon: &'static str,
    /// All available options
    options: Vec<String>,
    /// Currently selected option index
    selected_index: usize,
    /// Default option index (for determining if filtered)
    default_index: usize,
}

impl Facet {
    /// Create a new facet with an icon, options, and a default selection.
    ///
    /// # Arguments
    ///
    /// * `icon` - A static string representing the icon (e.g., "☍", "●", "📅")
    /// * `options` - Vector of option strings (e.g., ["All pipelines", "My pipelines"])
    /// * `default_index` - Index of the default option (usually 0)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use circleci_tui_rs::ui::widgets::faceted_search::Facet;
    ///
    /// let facet = Facet::new(
    ///     "●",
    ///     vec!["All statuses".to_string(), "Success".to_string(), "Failed".to_string()],
    ///     0
    /// );
    /// ```
    pub fn new(icon: &'static str, options: Vec<String>, default_index: usize) -> Self {
        assert!(!options.is_empty(), "Facet must have at least one option");
        assert!(
            default_index < options.len(),
            "Default index out of bounds"
        );

        let name = options[default_index].clone();
        Self {
            name,
            icon,
            options,
            selected_index: default_index,
            default_index,
        }
    }

    /// Check if this facet has a non-default filter applied
    pub fn is_filtered(&self) -> bool {
        self.selected_index != self.default_index
    }

    /// Get the current selection
    pub fn selected_option(&self) -> &str {
        &self.options[self.selected_index]
    }

    /// Update the display name based on current selection
    fn update_name(&mut self) {
        self.name = self.selected_option().to_string();
    }

    /// Reset to default selection
    pub fn reset(&mut self) {
        self.selected_index = self.default_index;
        self.update_name();
    }
}

/// A faceted search bar widget that provides filtering functionality
pub struct FacetedSearchBar {
    /// All filter facets
    facets: Vec<Facet>,
    /// Which filter button is currently focused
    active_btn_idx: usize,
    /// Whether dropdown is currently open
    dropdown_open: bool,
    /// Index of focused option in dropdown (when open)
    dropdown_focus_idx: usize,
}

impl FacetedSearchBar {
    /// Create a new faceted search bar with the given facets.
    ///
    /// # Arguments
    ///
    /// * `facets` - Vector of Facet instances defining the available filters
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use circleci_tui_rs::ui::widgets::faceted_search::{Facet, FacetedSearchBar};
    ///
    /// let facets = vec![
    ///     Facet::new("☍", vec!["All pipelines".to_string(), "My pipelines".to_string()], 0),
    ///     Facet::new("●", vec!["All statuses".to_string(), "Success".to_string()], 0),
    /// ];
    ///
    /// let search_bar = FacetedSearchBar::new(facets);
    /// ```
    pub fn new(facets: Vec<Facet>) -> Self {
        assert!(!facets.is_empty(), "FacetedSearchBar must have at least one facet");

        Self {
            facets,
            active_btn_idx: 0,
            dropdown_open: false,
            dropdown_focus_idx: 0,
        }
    }

    /// Render the faceted search bar to the given frame area.
    ///
    /// This renders both the filter buttons and the dropdown (if open).
    ///
    /// # Arguments
    ///
    /// * `f` - The frame to render to
    /// * `area` - The area to render the filter bar in (typically 3 lines tall)
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.render_filter_bar(f, area);

        // Render dropdown if open
        if self.dropdown_open {
            self.render_dropdown(f, area);
        }
    }

    /// Handle keyboard input for the faceted search bar.
    ///
    /// Returns `true` if the key was handled, `false` otherwise.
    ///
    /// # Keyboard Controls
    ///
    /// - Left/Right: Navigate between filter buttons
    /// - Enter: Open dropdown or confirm selection
    /// - Up/Down: Navigate dropdown options (when open)
    /// - Esc: Close dropdown
    ///
    /// # Arguments
    ///
    /// * `key` - The key code to handle
    ///
    /// # Returns
    ///
    /// `true` if the key was handled by this widget, `false` otherwise
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Left => {
                if !self.dropdown_open && self.active_btn_idx > 0 {
                    self.active_btn_idx -= 1;
                    return true;
                }
            }
            KeyCode::Right => {
                if !self.dropdown_open && self.active_btn_idx < self.facets.len() - 1 {
                    self.active_btn_idx += 1;
                    return true;
                }
            }
            KeyCode::Enter => {
                if !self.dropdown_open {
                    // Open dropdown
                    self.dropdown_open = true;
                    self.dropdown_focus_idx = self.facets[self.active_btn_idx].selected_index;
                } else {
                    // Confirm selection and close dropdown
                    self.facets[self.active_btn_idx].selected_index = self.dropdown_focus_idx;
                    self.facets[self.active_btn_idx].update_name();
                    self.dropdown_open = false;
                }
                return true;
            }
            KeyCode::Up => {
                if self.dropdown_open && self.dropdown_focus_idx > 0 {
                    self.dropdown_focus_idx -= 1;
                    return true;
                }
            }
            KeyCode::Down => {
                if self.dropdown_open {
                    let max_idx = self.facets[self.active_btn_idx].options.len() - 1;
                    if self.dropdown_focus_idx < max_idx {
                        self.dropdown_focus_idx += 1;
                        return true;
                    }
                }
            }
            KeyCode::Esc => {
                if self.dropdown_open {
                    self.dropdown_open = false;
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// Get the selected value for a specific facet.
    ///
    /// # Arguments
    ///
    /// * `facet_idx` - The index of the facet to query
    ///
    /// # Returns
    ///
    /// The selected option as a string slice, or None if index is out of bounds
    pub fn get_filter_value(&self, facet_idx: usize) -> Option<&str> {
        self.facets.get(facet_idx).map(|f| f.selected_option())
    }

    /// Check if any non-default filters are active.
    ///
    /// # Returns
    ///
    /// `true` if at least one facet has a non-default selection, `false` otherwise
    pub fn is_filtered(&self) -> bool {
        self.facets.iter().any(|f| f.is_filtered())
    }

    /// Reset all filters to their default values.
    pub fn reset_filters(&mut self) {
        for facet in &mut self.facets {
            facet.reset();
        }
    }

    /// Get a list of active filters (non-default selections) for display.
    ///
    /// # Returns
    ///
    /// Vector of strings describing active filters (e.g., "☍: My pipelines")
    pub fn get_active_filters(&self) -> Vec<String> {
        self.facets
            .iter()
            .filter(|f| f.is_filtered())
            .map(|f| format!("{}: {}", f.icon, f.selected_option()))
            .collect()
    }

    /// Render the filter button bar
    fn render_filter_bar(&self, f: &mut Frame, area: Rect) {
        let mut spans = Vec::new();

        for (idx, facet) in self.facets.iter().enumerate() {
            let is_active = idx == self.active_btn_idx;
            let is_filtered = facet.is_filtered();

            // Determine button style based on state
            let style = if self.dropdown_open && is_active {
                // Pressed state (dropdown open)
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(theme::FG_BRIGHT)
                    .add_modifier(Modifier::BOLD)
            } else if is_filtered {
                // Filtered state (non-default selection)
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                // Focused state
                Style::default()
                    .fg(theme::FG_BRIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                // Inactive state
                Style::default().fg(theme::FG_DIM)
            };

            let button_text = format!(" {} {} ", facet.icon, facet.name);
            spans.push(Span::styled(button_text, style));
            spans.push(Span::raw(" "));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER)),
        );
        f.render_widget(paragraph, area);
    }

    /// Render the dropdown menu overlay
    fn render_dropdown(&self, f: &mut Frame, filter_bar_area: Rect) {
        let facet = &self.facets[self.active_btn_idx];

        // Calculate button position for dropdown placement
        let mut x_offset = filter_bar_area.x + 1;
        for (idx, fct) in self.facets.iter().enumerate() {
            if idx == self.active_btn_idx {
                break;
            }
            x_offset += fct.name.len() as u16 + fct.icon.len() as u16 + 3;
        }

        // Calculate dropdown dimensions
        let max_option_width = facet
            .options
            .iter()
            .map(|o| o.len())
            .max()
            .unwrap_or(10);
        let dropdown_width = (max_option_width + 6) as u16; // Add padding for checkbox and margins
        let dropdown_height = (facet.options.len() as u16) + 2; // Add borders

        // Position dropdown below the button
        let dropdown_area = Rect {
            x: x_offset,
            y: filter_bar_area.y + filter_bar_area.height,
            width: dropdown_width.min(f.area().width - x_offset),
            height: dropdown_height.min(
                f.area().height - filter_bar_area.y - filter_bar_area.height,
            ),
        };

        // Create list items with checkmarks for selected options
        let items: Vec<ListItem> = facet
            .options
            .iter()
            .enumerate()
            .map(|(idx, option)| {
                let is_selected = idx == facet.selected_index;
                let is_focused = idx == self.dropdown_focus_idx;

                let checkbox = if is_selected { "✓" } else { " " };
                let text = format!(" {} {}", checkbox, option);

                let style = if is_focused {
                    // Focused item in dropdown
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(theme::FG_BRIGHT)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    // Selected item (not focused)
                    Style::default().fg(theme::ACCENT)
                } else {
                    // Regular item
                    Style::default().fg(theme::FG_PRIMARY)
                };

                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::ACCENT)),
        );

        // Render dropdown as overlay
        f.render_widget(Clear, dropdown_area); // Clear background
        f.render_widget(list, dropdown_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_facet_creation() {
        let facet = Facet::new("●", vec!["Option 1".to_string(), "Option 2".to_string()], 0);
        assert_eq!(facet.selected_option(), "Option 1");
        assert!(!facet.is_filtered());
    }

    #[test]
    fn test_facet_filtering() {
        let mut facet = Facet::new("●", vec!["All".to_string(), "Filtered".to_string()], 0);
        assert!(!facet.is_filtered());

        facet.selected_index = 1;
        facet.update_name();
        assert!(facet.is_filtered());
        assert_eq!(facet.selected_option(), "Filtered");
    }

    #[test]
    fn test_facet_reset() {
        let mut facet = Facet::new("●", vec!["All".to_string(), "Filtered".to_string()], 0);
        facet.selected_index = 1;
        facet.update_name();
        assert!(facet.is_filtered());

        facet.reset();
        assert!(!facet.is_filtered());
        assert_eq!(facet.selected_option(), "All");
    }

    #[test]
    fn test_search_bar_creation() {
        let facets = vec![
            Facet::new("●", vec!["All".to_string(), "Some".to_string()], 0),
            Facet::new("☍", vec!["Any".to_string(), "Specific".to_string()], 0),
        ];
        let search_bar = FacetedSearchBar::new(facets);
        assert_eq!(search_bar.facets.len(), 2);
        assert_eq!(search_bar.active_btn_idx, 0);
        assert!(!search_bar.dropdown_open);
    }

    #[test]
    fn test_search_bar_navigation() {
        let facets = vec![
            Facet::new("●", vec!["All".to_string()], 0),
            Facet::new("☍", vec!["Any".to_string()], 0),
        ];
        let mut search_bar = FacetedSearchBar::new(facets);

        assert_eq!(search_bar.active_btn_idx, 0);
        search_bar.handle_key(KeyCode::Right);
        assert_eq!(search_bar.active_btn_idx, 1);
        search_bar.handle_key(KeyCode::Left);
        assert_eq!(search_bar.active_btn_idx, 0);
    }

    #[test]
    fn test_search_bar_dropdown() {
        let facets = vec![Facet::new(
            "●",
            vec!["Option 1".to_string(), "Option 2".to_string()],
            0,
        )];
        let mut search_bar = FacetedSearchBar::new(facets);

        assert!(!search_bar.dropdown_open);
        search_bar.handle_key(KeyCode::Enter);
        assert!(search_bar.dropdown_open);
        assert_eq!(search_bar.dropdown_focus_idx, 0);

        search_bar.handle_key(KeyCode::Down);
        assert_eq!(search_bar.dropdown_focus_idx, 1);

        search_bar.handle_key(KeyCode::Enter);
        assert!(!search_bar.dropdown_open);
        assert_eq!(search_bar.facets[0].selected_index, 1);
    }

    #[test]
    fn test_get_filter_value() {
        let facets = vec![
            Facet::new("●", vec!["All".to_string(), "Some".to_string()], 0),
            Facet::new("☍", vec!["Any".to_string(), "Specific".to_string()], 0),
        ];
        let search_bar = FacetedSearchBar::new(facets);

        assert_eq!(search_bar.get_filter_value(0), Some("All"));
        assert_eq!(search_bar.get_filter_value(1), Some("Any"));
        assert_eq!(search_bar.get_filter_value(2), None);
    }

    #[test]
    fn test_is_filtered() {
        let facets = vec![Facet::new(
            "●",
            vec!["All".to_string(), "Some".to_string()],
            0,
        )];
        let mut search_bar = FacetedSearchBar::new(facets);

        assert!(!search_bar.is_filtered());
        search_bar.facets[0].selected_index = 1;
        assert!(search_bar.is_filtered());
    }

    #[test]
    fn test_reset_filters() {
        let facets = vec![
            Facet::new("●", vec!["All".to_string(), "Some".to_string()], 0),
            Facet::new("☍", vec!["Any".to_string(), "Specific".to_string()], 0),
        ];
        let mut search_bar = FacetedSearchBar::new(facets);

        search_bar.facets[0].selected_index = 1;
        search_bar.facets[1].selected_index = 1;
        assert!(search_bar.is_filtered());

        search_bar.reset_filters();
        assert!(!search_bar.is_filtered());
        assert_eq!(search_bar.get_filter_value(0), Some("All"));
        assert_eq!(search_bar.get_filter_value(1), Some("Any"));
    }

    #[test]
    fn test_get_active_filters() {
        let facets = vec![
            Facet::new("●", vec!["All".to_string(), "Some".to_string()], 0),
            Facet::new("☍", vec!["Any".to_string(), "Specific".to_string()], 0),
        ];
        let mut search_bar = FacetedSearchBar::new(facets);

        assert_eq!(search_bar.get_active_filters().len(), 0);

        search_bar.facets[0].selected_index = 1;
        search_bar.facets[0].update_name();
        let active = search_bar.get_active_filters();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0], "●: Some");

        search_bar.facets[1].selected_index = 1;
        search_bar.facets[1].update_name();
        let active = search_bar.get_active_filters();
        assert_eq!(active.len(), 2);
    }
}
