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
use crate::ui::widgets::error_modal::{ErrorModal, ErrorAction};
use crate::ui::widgets::help_modal::{HelpModal, HelpAction};
use crate::ui::widgets::status_message::StatusMessage;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{backend::Backend, Frame, Terminal};
use ratatui::layout::{Constraint, Direction, Layout};
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
    /// Error modal for displaying API errors and other errors
    pub error_modal: Option<ErrorModal>,
    /// Help modal for keyboard shortcuts
    pub help_modal: Option<HelpModal>,
    /// Status message to display to the user (auto-hides after 5 seconds)
    pub status_message: Option<StatusMessage>,
    /// Pending workflow load (pipeline_id)
    pub pending_workflow_load: Option<String>,
    /// Pending job load (workflow_id)
    pub pending_job_load: Option<String>,
    /// Pending log load (job_number)
    pub pending_log_load: Option<u32>,
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
            error_modal: None,
            help_modal: None,
            status_message: None,
            is_loading: false,
            pending_workflow_load: None,
            pending_job_load: None,
            pending_log_load: None,
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
        // Global quit handler - 'q' always quits (highest priority)
        // This allows users to quit even if modals are open or API calls are pending
        if key.code == KeyCode::Char('q') {
            self.should_quit = true;
            return Ok(());
        }

        // Global help handler - '?' opens help modal
        if key.code == KeyCode::Char('?') && self.help_modal.is_none() {
            self.help_modal = Some(HelpModal::new());
            return Ok(());
        }

        // Priority -1: If help modal is open, handle it first (highest priority)
        if let Some(modal) = &mut self.help_modal {
            match modal.handle_input(key) {
                HelpAction::Close => {
                    self.help_modal = None;
                }
                HelpAction::None => {
                    // Continue showing modal
                }
            }
            // Modal consumes all input when open
            return Ok(());
        }

        // Priority 0: If error modal is open, handle error modal input first (highest priority)
        if let Some(modal) = &mut self.error_modal {
            match modal.handle_input(key) {
                ErrorAction::Close => {
                    self.error_modal = None;
                }
                ErrorAction::Retry => {
                    // Close modal and let the user retry manually
                    // In a real implementation, you would re-trigger the failed operation
                    self.error_modal = None;
                }
                ErrorAction::None => {
                    // Continue showing modal
                }
            }
            // Modal consumes all input when open
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
                            self.status_message = Some(StatusMessage::info(
                                format!("Rerunning workflow {}...", workflow_id)
                            ));
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
                        PipelineDetailAction::LoadJobs(workflow_id) => {
                            // Trigger job loading
                            self.trigger_job_load(workflow_id);
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

        // Check if status message expired and remove it
        if let Some(ref msg) = self.status_message {
            if msg.is_expired() {
                self.status_message = None;
            }
        }

        // Split layout: status bar (if present) + main content
        let chunks = if self.status_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Status bar
                    Constraint::Min(0),    // Main content
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(0), // No status bar
                    Constraint::Min(0),    // Main content
                ])
                .split(area)
        };

        // Render status message at top if present
        if let Some(ref msg) = self.status_message {
            f.render_widget(msg.render(), chunks[0]);
        }

        // Determine the main content area
        let main_area = if self.status_message.is_some() {
            chunks[1]
        } else {
            area
        };

        // Render base screen
        match &self.current_screen {
            Screen::Pipelines => {
                self.pipeline_screen.render(f, main_area);
            }
            Screen::PipelineDetail(_) => {
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    detail.render(f, main_area);
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
        // Render error modal on top of everything (highest priority)
        if let Some(modal) = &mut self.error_modal {
            modal.render(f, area);
        }
        // Render help modal on top of everything (absolute highest priority)
        if let Some(modal) = &self.help_modal {
            modal.render(f, area);
        }
    }

    /// Navigate to the pipeline detail screen (Screen 2)
    ///
    /// Opens the pipeline detail view showing workflow tree and jobs list.
    pub fn navigate_to_pipeline_detail(&mut self, pipeline: Pipeline) {
        // Create screen with empty data - workflows will be loaded async
        self.pipeline_detail_screen = Some(PipelineDetailScreen::new(pipeline.clone()));

        // Set the screen to show loading state
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.loading_workflows = true;
        }

        // Trigger async workflow loading
        self.pending_workflow_load = Some(pipeline.id.clone());
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
        let job_number = job.job_number;
        self.log_modal = Some(LogModal::new(job));

        // Mark this job as needing log load on next event loop iteration
        self.pending_log_load = Some(job_number);
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
        match self.api_client.get_workflows(pipeline_id).await {
            Ok(workflows) => {
                // Update detail screen with workflows
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    detail.set_workflows(workflows.clone());
                    detail.loading_workflows = false;

                    // Auto-load jobs for the first workflow
                    if !workflows.is_empty() {
                        detail.loading_jobs = true;
                        self.pending_job_load = Some(workflows[0].id.clone());
                    }
                }
            }
            Err(e) => {
                // Show error modal
                self.show_api_error(e.into());
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    detail.loading_workflows = false;
                }
            }
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
        match self.api_client.get_jobs(workflow_id).await {
            Ok((jobs, next_page_token)) => {
                // Update detail screen with jobs and pagination info
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    detail.set_jobs_with_pagination(jobs, next_page_token, None);
                    detail.loading_jobs = false;
                }
            }
            Err(e) => {
                // Show error modal
                self.show_api_error(e.into());
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    detail.loading_jobs = false;
                }
            }
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
        // Show loading message
        self.status_message = Some(StatusMessage::info(format!("Rerunning workflow {}...", workflow_id)));

        // Call API to rerun workflow
        match self.api_client.rerun_workflow(workflow_id).await {
            Ok(_) => {
                self.status_message = Some(StatusMessage::success(
                    format!("Workflow {} rerun successful!", workflow_id)
                ));
            }
            Err(e) => {
                self.status_message = Some(StatusMessage::error(
                    format!("Failed to rerun workflow: {}", e)
                ));
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

    /// Check if workflows need to be loaded
    ///
    /// Returns the pipeline_id if workflows need to be loaded.
    pub fn should_load_workflows(&self) -> Option<String> {
        self.pending_workflow_load.clone()
    }

    /// Check if jobs need to be loaded
    ///
    /// Returns the workflow_id if jobs need to be loaded.
    pub fn should_load_jobs(&self) -> Option<String> {
        self.pending_job_load.clone()
    }

    /// Trigger job loading for the selected workflow
    ///
    /// This is called when the user navigates between workflows.
    pub fn trigger_job_load(&mut self, workflow_id: String) {
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.loading_jobs = true;
        }
        self.pending_job_load = Some(workflow_id);
    }

    /// Show an error modal with a message
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the error modal
    /// * `message` - The error message to display
    ///
    /// # Examples
    ///
    /// ```
    /// app.show_error("API Error", "Failed to connect to CircleCI API");
    /// ```
    pub fn show_error(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.error_modal = Some(ErrorModal::new(title.into(), message.into()));
    }

    /// Show an error modal with a message and detailed information
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the error modal
    /// * `message` - The error message to display
    /// * `details` - Detailed error information
    ///
    /// # Examples
    ///
    /// ```
    /// app.show_error_with_details(
    ///     "API Error",
    ///     "API returned error 404: Not Found",
    ///     "GET /api/v2/pipeline/abc123\nResponse: {\"message\":\"not found\"}"
    /// );
    /// ```
    pub fn show_error_with_details(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        details: impl Into<String>,
    ) {
        self.error_modal = Some(ErrorModal::with_details(
            title.into(),
            message.into(),
            details.into(),
        ));
    }

    /// Show an error modal from an anyhow::Error
    ///
    /// # Arguments
    ///
    /// * `error` - The error to display
    ///
    /// # Examples
    ///
    /// ```
    /// if let Err(e) = api_call().await {
    ///     app.show_api_error(e);
    /// }
    /// ```
    pub fn show_api_error(&mut self, error: anyhow::Error) {
        let error_message = format!("{}", error);
        let error_details = format!("{:?}", error);

        // Determine title based on error type
        let title = if error_message.contains("timeout") {
            "Timeout Error"
        } else if error_message.contains("404") || error_message.contains("Not Found") {
            "Not Found"
        } else if error_message.contains("401") || error_message.contains("403") {
            "Authentication Error"
        } else if error_message.contains("connect") || error_message.contains("network") {
            "Network Error"
        } else {
            "API Error"
        };

        self.error_modal = Some(
            ErrorModal::with_details(
                title.to_string(),
                error_message,
                error_details,
            )
            .with_retry(),
        );
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
        let app = App::new(config).unwrap();

        assert!(matches!(app.current_screen, Screen::Pipelines));
        assert!(!app.should_quit);
        assert!(app.pipeline_detail_screen.is_none());
        assert!(app.log_modal.is_none());
        assert!(app.error_modal.is_none());
    }

    #[test]
    fn test_navigation_to_pipeline_detail() {
        let config = create_test_config();
        let mut app = App::new(config).unwrap();

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
        let mut app = App::new(config).unwrap();

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
        let mut app = App::new(config).unwrap();

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
        let mut app = App::new(config).unwrap();

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

    #[test]
    fn test_error_modal_operations() {
        let config = create_test_config();
        let mut app = App::new(config).unwrap();

        // Show simple error
        app.show_error("Test Error", "Something went wrong");
        assert!(app.error_modal.is_some());

        // Close error modal
        app.error_modal = None;
        assert!(app.error_modal.is_none());
    }

    #[test]
    fn test_error_modal_with_details() {
        let config = create_test_config();
        let mut app = App::new(config).unwrap();

        // Show error with details
        app.show_error_with_details("API Error", "Request failed", "Details here");
        assert!(app.error_modal.is_some());
        if let Some(modal) = &app.error_modal {
            assert!(modal.is_visible());
        }
    }
}
