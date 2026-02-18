//! Main application state and event loop
//!
//! This module contains the main App struct that manages the application state,
//! handles events, and coordinates between different screens.

use crate::api::models::{Job, Pipeline};
use crate::config::Config;
use crate::ui::screens::PipelineScreen;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{backend::Backend, Frame, Terminal};
use std::time::Duration;

/// Application screens
#[derive(Debug, Clone)]
pub enum Screen {
    /// Screen 1: Pipeline list screen
    Pipelines,
    /// Screen 2: Pipeline detail screen (workflow tree + jobs list)
    PipelineDetail(Pipeline),
}

// Placeholder structs for screens that will be implemented
// TODO: Move these to their respective screen modules once implemented
pub struct PipelineDetailScreen {
    pub pipeline: Pipeline,
}

impl PipelineDetailScreen {
    pub fn new(pipeline: Pipeline) -> Self {
        Self { pipeline }
    }

    pub fn render(&mut self, _f: &mut Frame, _area: ratatui::layout::Rect) {
        // TODO: Implement render
    }

    pub fn handle_input(&mut self, _key: KeyEvent) -> PipelineDetailAction {
        // TODO: Implement input handling
        PipelineDetailAction::None
    }
}

pub struct LogModal {
    pub job: Job,
}

impl LogModal {
    pub fn new(job: Job) -> Self {
        Self { job }
    }

    pub fn render(&mut self, _f: &mut Frame, _area: ratatui::layout::Rect) {
        // TODO: Implement modal render as overlay
    }

    pub fn handle_input(&mut self, _key: KeyEvent) -> bool {
        // TODO: Implement input handling
        // Returns true if modal should close
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PipelineDetailAction {
    None,
    Back,
    OpenJobLog(Job),
}

/// Main application state
pub struct App {
    /// Current screen being displayed
    pub current_screen: Screen,
    /// Pipeline screen state (Screen 1)
    pub pipeline_screen: PipelineScreen,
    /// Pipeline detail screen state (Screen 2)
    pub pipeline_detail_screen: Option<PipelineDetailScreen>,
    /// Log modal overlay (shown on top of any screen)
    pub log_modal: Option<LogModal>,
    /// Should quit flag
    pub should_quit: bool,
    /// Application configuration
    pub config: Config,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Self {
        Self {
            current_screen: Screen::Pipelines,
            pipeline_screen: PipelineScreen::new(),
            pipeline_detail_screen: None,
            log_modal: None,
            should_quit: false,
            config,
        }
    }

    /// Run the main application loop
    ///
    /// This method handles the event loop, rendering, and input handling.
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Draw the UI
            terminal.draw(|f| self.render(f))?;

            // Handle input events
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Only process key press events, not key release
                    if key.kind == KeyEventKind::Press {
                        self.handle_event(key)?;
                    }
                }
            }

            // Check if we should quit
            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle input events
    fn handle_event(&mut self, key: KeyEvent) -> Result<()> {
        // Global quit handler
        if key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return Ok(());
        }

        // Priority 1: If modal is open, handle modal input first
        if let Some(modal) = &mut self.log_modal {
            if key.code == KeyCode::Esc {
                // Close modal on Esc
                self.close_log_modal();
                return Ok(());
            }

            // Handle modal-specific input
            if modal.handle_input(key) {
                // Modal requests close
                self.close_log_modal();
            }

            // Modal consumes all input when open
            return Ok(());
        }

        // Priority 2: Screen-specific handlers
        match &self.current_screen {
            Screen::Pipelines => {
                // Handle pipeline screen input
                if self.pipeline_screen.handle_input(key) {
                    // User pressed Enter to open pipeline detail
                    if let Some(pipeline) = self.pipeline_screen.get_selected_pipeline() {
                        self.navigate_to_pipeline_detail(pipeline.clone());
                    }
                }
            }
            Screen::PipelineDetail(_) => {
                // Handle Esc to go back to pipelines
                if key.code == KeyCode::Esc {
                    self.navigate_back_to_pipelines();
                    return Ok(());
                }

                // Handle pipeline detail screen input
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    match detail.handle_input(key) {
                        PipelineDetailAction::Back => {
                            self.navigate_back_to_pipelines();
                        }
                        PipelineDetailAction::OpenJobLog(job) => {
                            self.open_job_log_modal(job);
                        }
                        PipelineDetailAction::None => {
                            // No action needed
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Render the current screen
    fn render(&mut self, f: &mut Frame) {
        let area = f.size();

        // Render base screen
        match &self.current_screen {
            Screen::Pipelines => {
                self.pipeline_screen.render(f, area);
            }
            Screen::PipelineDetail(_) => {
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    detail.render(f, area);
                }
            }
        }

        // Render modal on top if open
        if let Some(modal) = &mut self.log_modal {
            modal.render(f, area);
        }
    }

    /// Navigate to the pipeline detail screen (Screen 2)
    ///
    /// Opens the pipeline detail view showing workflow tree and jobs list.
    pub fn navigate_to_pipeline_detail(&mut self, pipeline: Pipeline) {
        self.pipeline_detail_screen = Some(PipelineDetailScreen::new(pipeline.clone()));
        self.current_screen = Screen::PipelineDetail(pipeline);
    }

    /// Navigate back to the pipelines screen (Screen 1)
    ///
    /// Returns to the main pipelines list view.
    pub fn navigate_back_to_pipelines(&mut self) {
        self.current_screen = Screen::Pipelines;
        self.pipeline_detail_screen = None;
        // Close any open modals when navigating back
        self.log_modal = None;
    }

    /// Open the job log modal (overlay)
    ///
    /// Shows job logs as a modal overlay on top of the current screen.
    pub fn open_job_log_modal(&mut self, job: Job) {
        self.log_modal = Some(LogModal::new(job));
    }

    /// Close the log modal
    ///
    /// Hides the job log modal overlay.
    pub fn close_log_modal(&mut self) {
        self.log_modal = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::ExecutorInfo;
    use chrono::Utc;

    fn create_test_config() -> Config {
        Config {
            circle_token: "test_token".to_string(),
            project_slug: "gh/test/repo".to_string(),
        }
    }

    fn create_test_pipeline() -> Pipeline {
        use crate::api::models::{TriggerInfo, VcsInfo};

        Pipeline {
            id: "test-pipeline-id".to_string(),
            number: 123,
            state: "success".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            vcs: VcsInfo {
                branch: "main".to_string(),
                revision: "abc123".to_string(),
                commit_subject: "Test commit".to_string(),
                commit_author_name: "test-user".to_string(),
                commit_timestamp: Utc::now(),
            },
            trigger: TriggerInfo {
                trigger_type: "webhook".to_string(),
            },
            project_slug: "gh/test/repo".to_string(),
        }
    }

    fn create_test_job() -> Job {
        Job {
            id: "test-job-id".to_string(),
            name: "test-job".to_string(),
            status: "success".to_string(),
            job_number: 1,
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
    fn test_app_creation() {
        let config = create_test_config();
        let app = App::new(config);

        assert!(matches!(app.current_screen, Screen::Pipelines));
        assert!(!app.should_quit);
        assert!(app.pipeline_detail_screen.is_none());
        assert!(app.log_modal.is_none());
    }

    #[test]
    fn test_navigation_to_pipeline_detail() {
        let config = create_test_config();
        let mut app = App::new(config);

        let pipeline = create_test_pipeline();

        // Navigate to pipeline detail (Screen 2)
        app.navigate_to_pipeline_detail(pipeline.clone());
        assert!(matches!(
            app.current_screen,
            Screen::PipelineDetail(_)
        ));
        assert!(app.pipeline_detail_screen.is_some());
        assert!(app.log_modal.is_none());

        // Navigate back to pipelines (Screen 1)
        app.navigate_back_to_pipelines();
        assert!(matches!(app.current_screen, Screen::Pipelines));
        assert!(app.pipeline_detail_screen.is_none());
        assert!(app.log_modal.is_none());
    }

    #[test]
    fn test_log_modal_operations() {
        let config = create_test_config();
        let mut app = App::new(config);

        let job = create_test_job();

        // Open log modal
        app.open_job_log_modal(job.clone());
        assert!(app.log_modal.is_some());
        assert!(matches!(app.current_screen, Screen::Pipelines));

        // Close log modal
        app.close_log_modal();
        assert!(app.log_modal.is_none());
    }

    #[test]
    fn test_navigation_closes_modal() {
        let config = create_test_config();
        let mut app = App::new(config);

        let pipeline = create_test_pipeline();
        let job = create_test_job();

        // Navigate to detail and open modal
        app.navigate_to_pipeline_detail(pipeline);
        app.open_job_log_modal(job);
        assert!(app.log_modal.is_some());

        // Navigate back should close modal
        app.navigate_back_to_pipelines();
        assert!(app.log_modal.is_none());
    }

    #[test]
    fn test_two_screen_architecture() {
        let config = create_test_config();
        let mut app = App::new(config);

        let pipeline = create_test_pipeline();

        // Start on Screen 1 (Pipelines)
        assert!(matches!(app.current_screen, Screen::Pipelines));

        // Navigate to Screen 2 (Pipeline Detail)
        app.navigate_to_pipeline_detail(pipeline.clone());
        assert!(matches!(
            app.current_screen,
            Screen::PipelineDetail(_)
        ));

        // There are only 2 screens now, no third screen
        assert!(app.pipeline_detail_screen.is_some());

        // Navigate back to Screen 1
        app.navigate_back_to_pipelines();
        assert!(matches!(app.current_screen, Screen::Pipelines));
    }
}
