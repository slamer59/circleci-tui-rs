//! Main application state and event loop
//!
//! This module contains the main App struct that manages the application state,
//! handles events, and coordinates between different screens.

use crate::api::models::{Pipeline, Workflow};
use crate::config::Config;
use crate::ui::screens::{NavigationAction, PipelineScreen, WorkflowScreen};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{backend::Backend, Frame, Terminal};
use std::time::Duration;

/// Application screens
#[derive(Debug, Clone)]
pub enum Screen {
    /// Pipeline list screen
    Pipelines,
    /// Workflow detail screen with pipeline and workflow data
    Workflow(Pipeline, Workflow),
}

/// Main application state
pub struct App {
    /// Current screen being displayed
    pub current_screen: Screen,
    /// Pipeline screen state
    pub pipeline_screen: PipelineScreen,
    /// Workflow screen state (only when on workflow screen)
    pub workflow_screen: Option<WorkflowScreen>,
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
            workflow_screen: None,
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

        // Screen-specific handlers
        match &self.current_screen {
            Screen::Pipelines => {
                // Handle pipeline screen input
                if self.pipeline_screen.handle_input(key) {
                    // User pressed Enter to open a pipeline/workflow
                    if let Some(pipeline) = self.pipeline_screen.get_selected_pipeline() {
                        // Get the first workflow for this pipeline
                        let workflows = crate::api::models::mock_data::mock_workflows(&pipeline.id);
                        if let Some(workflow) = workflows.first() {
                            self.navigate_to_workflow(pipeline.clone(), workflow.clone());
                        }
                    }
                }
            }
            Screen::Workflow(..) => {
                // Handle workflow screen input
                if let Some(ws) = &mut self.workflow_screen {
                    match ws.handle_input(key) {
                        NavigationAction::Back => {
                            self.navigate_back();
                        }
                        NavigationAction::Quit => {
                            self.should_quit = true;
                        }
                        NavigationAction::None => {
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
        let size = f.size();

        match &self.current_screen {
            Screen::Pipelines => {
                self.pipeline_screen.render(f, size);
            }
            Screen::Workflow(..) => {
                if let Some(ws) = &mut self.workflow_screen {
                    ws.render(f, size);
                }
            }
        }
    }

    /// Navigate to the workflow screen
    pub fn navigate_to_workflow(&mut self, pipeline: Pipeline, workflow: Workflow) {
        self.workflow_screen = Some(WorkflowScreen::new(pipeline.clone(), workflow.clone()));
        self.current_screen = Screen::Workflow(pipeline, workflow);
    }

    /// Navigate back to the pipeline screen
    pub fn navigate_back(&mut self) {
        self.current_screen = Screen::Pipelines;
        self.workflow_screen = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            circle_token: "test_token".to_string(),
            project_slug: "gh/test/repo".to_string(),
        }
    }

    #[test]
    fn test_app_creation() {
        let config = create_test_config();
        let app = App::new(config);

        assert!(matches!(app.current_screen, Screen::Pipelines));
        assert!(!app.should_quit);
        assert!(app.workflow_screen.is_none());
    }

    #[test]
    fn test_navigation() {
        let config = create_test_config();
        let mut app = App::new(config);

        // Get a test pipeline and workflow
        if let Some(pipeline) = app.pipeline_screen.get_selected_pipeline() {
            let workflows = crate::api::models::mock_data::mock_workflows(&pipeline.id);
            if let Some(workflow) = workflows.first() {
                // Navigate to workflow
                app.navigate_to_workflow(pipeline.clone(), workflow.clone());
                assert!(matches!(app.current_screen, Screen::Workflow(..)));
                assert!(app.workflow_screen.is_some());

                // Navigate back
                app.navigate_back();
                assert!(matches!(app.current_screen, Screen::Pipelines));
                assert!(app.workflow_screen.is_none());
            }
        }
    }
}
