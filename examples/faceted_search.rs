use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;

/// Represents a single filter facet (dimension) with multiple options
#[derive(Clone)]
struct Facet {
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
    fn new(icon: &'static str, options: Vec<String>, default_index: usize) -> Self {
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
    fn is_filtered(&self) -> bool {
        self.selected_index != self.default_index
    }

    /// Get the current selection
    fn selected_option(&self) -> &str {
        &self.options[self.selected_index]
    }

    /// Update the display name based on current selection
    fn update_name(&mut self) {
        self.name = self.selected_option().to_string();
    }
}

/// Application state
struct App {
    /// All filter facets
    facets: Vec<Facet>,
    /// Which filter button is currently focused
    active_btn_idx: usize,
    /// Whether dropdown is currently open
    dropdown_open: bool,
    /// Index of focused option in dropdown (when open)
    dropdown_focus_idx: usize,
}

impl App {
    fn new() -> Self {
        let facets = vec![
            Facet::new(
                "☍",
                vec![
                    "All pipelines".to_string(),
                    "My pipelines".to_string(),
                    "Frontend builds".to_string(),
                    "Backend tests".to_string(),
                ],
                0,
            ),
            Facet::new(
                "❐",
                vec![
                    "All projects".to_string(),
                    "Project Alpha".to_string(),
                    "Project Beta".to_string(),
                ],
                0,
            ),
            Facet::new(
                "📅",
                vec![
                    "Any time".to_string(),
                    "Last 24 hours".to_string(),
                    "Last week".to_string(),
                ],
                0,
            ),
            Facet::new(
                "●",
                vec![
                    "All statuses".to_string(),
                    "Success".to_string(),
                    "Failed".to_string(),
                    "Running".to_string(),
                ],
                0,
            ),
        ];

        Self {
            facets,
            active_btn_idx: 0,
            dropdown_open: false,
            dropdown_focus_idx: 0,
        }
    }

    /// Get active filter summary for display
    fn get_active_filters(&self) -> Vec<String> {
        self.facets
            .iter()
            .filter(|f| f.is_filtered())
            .map(|f| format!("{}: {}", f.icon, f.selected_option()))
            .collect()
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') => {
                if !self.dropdown_open {
                    return true; // Signal to quit
                }
            }
            KeyCode::Left => {
                if !self.dropdown_open && self.active_btn_idx > 0 {
                    self.active_btn_idx -= 1;
                }
            }
            KeyCode::Right => {
                if !self.dropdown_open && self.active_btn_idx < self.facets.len() - 1 {
                    self.active_btn_idx += 1;
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
            }
            KeyCode::Up => {
                if self.dropdown_open && self.dropdown_focus_idx > 0 {
                    self.dropdown_focus_idx -= 1;
                }
            }
            KeyCode::Down => {
                if self.dropdown_open {
                    let max_idx = self.facets[self.active_btn_idx].options.len() - 1;
                    if self.dropdown_focus_idx < max_idx {
                        self.dropdown_focus_idx += 1;
                    }
                }
            }
            KeyCode::Esc => {
                if self.dropdown_open {
                    self.dropdown_open = false;
                }
            }
            _ => {}
        }
        false
    }
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if app.handle_key(key.code) {
                return Ok(());
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Filter bar
            Constraint::Min(0),    // Content area
        ])
        .split(f.area());

    // Render filter buttons
    render_filter_bar(f, app, chunks[0]);

    // Render content area with filter summary
    render_content(f, app, chunks[1]);

    // Render dropdown if open
    if app.dropdown_open {
        render_dropdown(f, app, chunks[0]);
    }
}

fn render_filter_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();
    let mut x_offset = area.x + 1; // Start position

    for (idx, facet) in app.facets.iter().enumerate() {
        let is_active = idx == app.active_btn_idx;
        let is_filtered = facet.is_filtered();

        // Determine button style
        let style = if app.dropdown_open && is_active {
            // Pressed state
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else if is_filtered {
            // Filtered state (non-default selection)
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else if is_active {
            // Focused state
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            // Inactive state
            Style::default().fg(Color::DarkGray)
        };

        let button_text = format!(" {} {} ", facet.icon, facet.name);
        spans.push(Span::styled(button_text, style));
        spans.push(Span::raw(" "));

        x_offset += facet.name.len() as u16 + facet.icon.len() as u16 + 3;
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).block(Block::default().borders(Borders::ALL));
    f.render_widget(paragraph, area);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    let active_filters = app.get_active_filters();

    let mut text = vec![];

    if !active_filters.is_empty() {
        text.push(Line::from(vec![Span::styled(
            "Active filters: ",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        for filter in active_filters {
            text.push(Line::from(vec![Span::styled(
                format!("  • {}", filter),
                Style::default().fg(Color::Cyan),
            )]));
        }
        text.push(Line::from(""));
    } else {
        text.push(Line::from(vec![Span::styled(
            "No filters applied",
            Style::default().fg(Color::DarkGray),
        )]));
        text.push(Line::from(""));
    }

    text.push(Line::from("Filter results will appear here..."));
    text.push(Line::from(""));
    text.push(Line::from(vec![Span::styled(
        "Controls:",
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    text.push(Line::from("  Left/Right: Navigate filters"));
    text.push(Line::from("  Enter/Down: Open dropdown or confirm"));
    text.push(Line::from("  Up/Down: Navigate options (in dropdown)"));
    text.push(Line::from("  Esc: Close dropdown"));
    text.push(Line::from("  q: Quit"));

    let paragraph = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Faceted Search Demo"),
    );
    f.render_widget(paragraph, area);
}

fn render_dropdown(f: &mut Frame, app: &App, filter_bar_area: Rect) {
    let facet = &app.facets[app.active_btn_idx];

    // Calculate button position and dropdown area
    let mut x_offset = filter_bar_area.x + 1;
    for (idx, f) in app.facets.iter().enumerate() {
        if idx == app.active_btn_idx {
            break;
        }
        x_offset += f.name.len() as u16 + f.icon.len() as u16 + 3;
    }

    // Dropdown dimensions
    let max_option_width = facet.options.iter().map(|o| o.len()).max().unwrap_or(10);
    let dropdown_width = (max_option_width + 6) as u16; // Add padding for checkbox and margins
    let dropdown_height = (facet.options.len() as u16) + 2; // Add borders

    // Position dropdown below the button
    let dropdown_area = Rect {
        x: x_offset,
        y: filter_bar_area.y + filter_bar_area.height,
        width: dropdown_width.min(f.area().width - x_offset),
        height: dropdown_height.min(f.area().height - filter_bar_area.y - filter_bar_area.height),
    };

    // Create list items
    let items: Vec<ListItem> = facet
        .options
        .iter()
        .enumerate()
        .map(|(idx, option)| {
            let is_selected = idx == facet.selected_index;
            let is_focused = idx == app.dropdown_focus_idx;

            let checkbox = if is_selected { "✓" } else { " " };
            let text = format!(" {} {}", checkbox, option);

            let style = if is_focused {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            ListItem::new(text).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    // Render dropdown as overlay
    f.render_widget(Clear, dropdown_area); // Clear background
    f.render_widget(list, dropdown_area);
}
