//! Main application state and event loop
//!
//! This module contains the main App struct that manages the application state,
//! handles events, and coordinates between different screens.

use crate::api::client::CircleCIClient;
use crate::api::models::{Job, Pipeline};
use crate::config::Config;
use crate::ui::screens::{PipelineDetailAction, PipelineDetailScreen, PipelineScreen};
use crate::ui::widgets::log_modal::{LogModal, ModalAction};
use crate::ui::widgets::confirm_modal::{ConfirmModal, ConfirmAction};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{backend::Backend, Frame, Terminal};
use std::sync::Arc;
use std::time::Duration;

/// Application screens
#[derive(Debug, Clone)]
pub enum Screen {
    /// Screen 1: Pipeline list screen
    Pipelines,
    /// Screen 2: Pipeline detail screen (workflow tree + jobs list)
    PipelineDetail(Pipeline),
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
    /// CircleCI API client
    pub api_client: Arc<CircleCIClient>,
    /// Loading state
    pub is_loading: bool,
    /// Confirmation modal for actions like rerun workflow
    pub confirm_modal: Option<ConfirmModal>,
    /// Status message to display to the user
    pub status_message: Option<String>,
}

impl App {
    /// Create a new application instance
    pub fn new(config: Config) -> Result<Self> {
        // Create API client
        let api_client = Arc::new(CircleCIClient::new(
            config.circle_token.clone(),
            config.project_slug.clone(),
        )?);

        Ok(Self {
            current_screen: Screen::Pipelines,
            pipeline_screen: PipelineScreen::new(),
            pipeline_detail_screen: None,
            log_modal: None,
            should_quit: false,
            config,
            api_client,
            confirm_modal: None,
            status_message: None,
            is_loading: false,
        })
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
    pub fn handle_event(&mut self, key: KeyEvent) -> Result<()> {
        // Global quit handler
        if key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return Ok(());
        }

        // Priority 1: If modal is open, handle modal input first
        if let Some(modal) = &mut self.log_modal {
            // Handle modal-specific input
            match modal.handle_input(key) {
                ModalAction::Close => {
                    self.close_log_modal();
                }
                ModalAction::Rerun => {
                    // TODO: Implement rerun functionality
                    self.close_log_modal();
                }
                ModalAction::None => {
                    // Continue showing modal
                }
            }

            // Modal consumes all input when open
            return Ok(());
        }

        // Priority 1.5: If confirmation modal is open, handle confirm input
        if let Some(modal) = &mut self.confirm_modal {
            match modal.handle_input(key) {
                ConfirmAction::Yes => {
                    // User confirmed - execute rerun workflow
                    self.confirm_modal = None;
                    // Trigger rerun if we have a workflow_id stored
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        if let Some(workflow_id) = &detail.confirm_workflow_id {
                            // Set status message - the actual API call should be made asynchronously
                            self.status_message = Some(format!("Rerunning workflow {}...", workflow_id));
                            // Note: In a real async implementation, you would call:
                            // tokio::spawn(self.api_client.rerun_workflow(workflow_id.clone()));
                        }
                        detail.confirm_workflow_id = None;
                    }
                }
                ConfirmAction::No => {
                    // User cancelled - close modal
                    self.confirm_modal = None;
                }
                ConfirmAction::None => {
                    // Continue showing modal
                }
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
                        PipelineDetailAction::LoadMoreJobs => {
                            // Note: This triggers an async load_more_jobs call
                            // The actual loading should be handled in the main event loop
                            // For now, this is a placeholder for the action
                        }
                        PipelineDetailAction::RerunWorkflow(workflow_id) => {
                            // Show confirmation modal
                            if let Some(detail) = &self.pipeline_detail_screen {
                                let workflow = detail.workflows.iter()
                                    .find(|w| w.id == workflow_id)
                                    .cloned();
                                if let Some(workflow) = workflow {
                                    let message = format!("Rerun workflow: {}?", workflow.name);
                                    self.confirm_modal = Some(ConfirmModal::new(message));
                                    // Store workflow_id in detail screen for later use
                                    if let Some(detail) = &mut self.pipeline_detail_screen {
                                        detail.confirm_workflow_id = Some(workflow_id);
                                    }
                                }
                            }
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
    pub fn render(&mut self, f: &mut Frame) {
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
        // Render confirmation modal on top if open
        if let Some(modal) = &self.confirm_modal {
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

    /// Load pipelines from the API
    ///
    /// This is an async method that fetches pipelines and updates the screen state.
    pub async fn load_pipelines(&mut self) -> Result<()> {
        self.is_loading = true;

        // Fetch pipelines from API (limit to 50)
        let pipelines = self.api_client.get_pipelines(50).await?;

        // Update pipeline screen with new data
        self.pipeline_screen.set_pipelines(pipelines);

        self.is_loading = false;
        Ok(())
    }

    /// Load workflows for a pipeline
    ///
    /// This is an async method that fetches workflows and updates the detail screen.
    pub async fn load_workflows(&mut self, pipeline_id: &str) -> Result<()> {
        self.is_loading = true;

        // Fetch workflows from API
        let workflows = self.api_client.get_workflows(pipeline_id).await?;

        // Update detail screen with workflows
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.set_workflows(workflows);
        }

        self.is_loading = false;
        Ok(())
    }

    /// Load jobs for a workflow
    ///
    /// This is an async method that fetches jobs and updates the detail screen.
    pub async fn load_jobs(&mut self, workflow_id: &str) -> Result<()> {
        self.is_loading = true;

        // Fetch jobs from API (returns tuple with pagination info)
        let (jobs, next_page_token) = self.api_client.get_jobs(workflow_id).await?;

        // Update detail screen with jobs and pagination info
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.set_jobs_with_pagination(jobs, next_page_token, None);
        }

        self.is_loading = false;
        Ok(())
    }

    /// Load more jobs for the current workflow (pagination)
    ///
    /// This is an async method that fetches the next page of jobs.
    pub async fn load_more_jobs(&mut self, workflow_id: &str) -> Result<()> {
        // Get the next page token from the detail screen
        let page_token = if let Some(detail) = &self.pipeline_detail_screen {
            detail.next_page_token.clone()
        } else {
            return Ok(());
        };

        if let Some(token) = page_token {
            // Set loading state
            if let Some(detail) = &mut self.pipeline_detail_screen {
                detail.loading_more_jobs = true;
            }

            // Fetch next page of jobs
            let (jobs, next_page_token) = self
                .api_client
                .get_jobs_page(workflow_id, Some(&token))
                .await?;

            // Append jobs to existing list
            if let Some(detail) = &mut self.pipeline_detail_screen {
                detail.append_jobs(jobs, next_page_token);
                detail.loading_more_jobs = false;
            }
        }

        Ok(())
    }
    /// Rerun a workflow
    ///
    /// This is an async method that triggers a workflow rerun.
    pub async fn rerun_workflow(&mut self, workflow_id: &str) -> Result<()> {
        // Call API to rerun workflow
        match self.api_client.rerun_workflow(workflow_id).await {
            Ok(_) => {
                self.status_message = Some(format!("Workflow {} rerun successfully", workflow_id));
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to rerun workflow: {}", e));
            }
        }
        Ok(())
    }


    /// Load logs for a job
    ///
    /// This is an async method that fetches job logs and updates the modal.
    pub async fn load_job_logs(&mut self, job_number: u32) -> Result<()> {
        // Fetch logs from API
        let logs = self.api_client.stream_job_log(job_number).await?;

        // Update modal with logs
        if let Some(modal) = &mut self.log_modal {
            modal.set_logs(logs);
        }

        Ok(())
    }

    /// Check if the log modal needs to refresh logs
    ///
    /// This should be called in the event loop to auto-refresh streaming logs.
    pub fn should_refresh_logs(&self) -> Option<u32> {
        if let Some(modal) = &self.log_modal {
            if modal.should_refresh() {
                return Some(modal.job_number());
            }
        }
        None
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
