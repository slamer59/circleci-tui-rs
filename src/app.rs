//! Main application state and event loop
//!
//! This module contains the main App struct that manages the application state,
//! handles events, and coordinates between different screens.

use crate::api::client::{CircleCIClient, StepStreamMessage};
use crate::api::models::{Job, Pipeline, Workflow};
use crate::cache::{LogCacheManager, PrefetchCoordinator};
use crate::config::Config;
use crate::preferences::PreferencesManager;
use crate::ui::screens::{PipelineDetailAction, PipelineDetailScreen, PipelineScreen};
use crate::ui::widgets::confirm_modal::{ConfirmAction, ConfirmModal};
use crate::ui::widgets::error_modal::{ErrorAction, ErrorModal};
use crate::ui::widgets::help_modal::{HelpAction, HelpModal};
use crate::ui::widgets::log_modal::{LogModal, ModalAction};
use crate::ui::widgets::ssh_modal::{SshAction, SshModal};
use crate::ui::widgets::status_message::StatusMessage;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Results from background API tasks
pub enum BgTaskResult {
    /// Step metadata discovered - show step list immediately
    LogStepsDiscovered {
        steps: Vec<(String, String)>,
    },
    /// Logs fetched for a specific step
    LogStepFetched {
        step_index: usize,
        logs: Vec<String>,
    },
    /// All step logs fetched - cache and mark complete
    LogsComplete {
        job_number: u32,
        job_status: Option<String>,
    },
    LogsError(anyhow::Error),
    WorkflowsLoaded(Vec<Workflow>),
    WorkflowsError(anyhow::Error),
    JobsLoaded {
        jobs: Vec<Job>,
        next_page_token: Option<String>,
    },
    JobsError(anyhow::Error),
    MoreJobsLoaded {
        jobs: Vec<Job>,
        next_page_token: Option<String>,
    },
    MoreJobsError(anyhow::Error),
    PowerlineLogsLoaded {
        job_number: u32,
        logs: Vec<String>,
        job_status: Option<String>,
    },
    PowerlineLogsError(anyhow::Error),
    FailedJobsLogsReady(String),
    FailedJobsLogsWrittenToFile(std::path::PathBuf),
}

/// Application screens
#[derive(Debug, Clone)]
pub enum Screen {
    /// Screen 1: Pipeline list screen
    Pipelines,
    /// Screen 2: Pipeline detail screen (workflow tree + jobs list)
    PipelineDetail,
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
    /// SSH modal for displaying SSH commands
    pub ssh_modal: Option<SshModal>,
    /// Status message to display to the user (auto-hides after 5 seconds)
    pub status_message: Option<StatusMessage>,
    /// Pending workflow load (pipeline_id)
    pub pending_workflow_load: Option<String>,
    /// Pending job load (workflow_id)
    pub pending_job_load: Option<String>,
    /// Pending log load (job_number)
    pub pending_log_load: Option<u32>,
    /// Pending load more jobs (workflow_id)
    pub pending_load_more_jobs: Option<String>,
    /// Preferences manager
    pub preferences: PreferencesManager,
    /// Log cache manager for disk-based caching
    pub log_cache_manager: LogCacheManager,
    /// Prefetch coordinator for async log prefetching
    pub prefetch_coordinator: PrefetchCoordinator,
    /// Channel sender for background task results
    pub bg_sender: mpsc::UnboundedSender<BgTaskResult>,
    /// Channel receiver for background task results
    pub bg_receiver: mpsc::UnboundedReceiver<BgTaskResult>,
    /// Spinner for long-running fetch operations
    pub fetch_spinner: crate::ui::widgets::spinner::Spinner,
    /// Whether a fetch operation is in progress (drives spinner)
    pub is_fetching: bool,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        // Load preferences
        let mut preferences = PreferencesManager::load()?;

        // Create API client
        let api_client = Arc::new(CircleCIClient::new(
            config.circle_token.clone(),
            config.project_slug.clone(),
        )?);

        // Fetch user identity if cache is stale
        let (authenticated_user, authenticated_user_name) = if preferences.is_user_cache_stale() {
            match api_client.get_me().await {
                Ok(me) => {
                    let login = me.login.clone();
                    let name = me.name.clone();
                    preferences.update_user_cache(login.clone(), name.clone());
                    (Some(login), name)
                }
                Err(e) => {
                    // Log warning, use cached value
                    eprintln!("Warning: Failed to fetch user identity: {}", e);
                    preferences
                        .get_preferences()
                        .user
                        .as_ref()
                        .map(|u| (Some(u.login.clone()), u.name.clone()))
                        .unwrap_or((None, None))
                }
            }
        } else {
            preferences
                .get_preferences()
                .user
                .as_ref()
                .map(|u| (Some(u.login.clone()), u.name.clone()))
                .unwrap_or((None, None))
        };

        // Apply first-run defaults (owner filter only)
        if preferences.get_preferences().first_run {
            let prefs = preferences.get_preferences_mut();
            prefs.pipeline_filters.owner_index = 1; // "Mine"
            preferences.clear_first_run();
            preferences.save()?;
        }

        // Smart branch selection: LOCAL → GLOBAL
        // Always use current git branch if available, otherwise use saved preference
        let current_git_branch = crate::git::get_current_branch();
        let branch_to_use = current_git_branch.or_else(|| {
            preferences
                .get_preferences()
                .pipeline_filters
                .branch
                .clone()
        });

        // Create modified preferences with current branch priority
        let mut filter_prefs = preferences.get_preferences().pipeline_filters.clone();
        filter_prefs.branch = branch_to_use;

        // Create PipelineScreen with adjusted preferences
        let pipeline_screen = PipelineScreen::with_preferences(
            &filter_prefs,
            authenticated_user.clone(),
            authenticated_user_name.clone(),
        );

        // Initialize log cache manager
        let log_cache_manager = LogCacheManager::new()?;
        log_cache_manager.cleanup_old_entries()?; // Run cleanup on startup

        // Initialize prefetch coordinator
        let prefetch_coordinator =
            PrefetchCoordinator::new(Arc::new(log_cache_manager.clone()), Arc::clone(&api_client));

        let (bg_sender, bg_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            current_screen: Screen::Pipelines,
            pipeline_screen,
            pipeline_detail_screen: None,
            log_modal: None,
            should_quit: false,
            api_client,
            confirm_modal: None,
            error_modal: None,
            help_modal: None,
            ssh_modal: None,
            status_message: None,
            is_loading: false,
            pending_workflow_load: None,
            pending_job_load: None,
            pending_log_load: None,
            pending_load_more_jobs: None,
            preferences,
            log_cache_manager,
            prefetch_coordinator,
            bg_sender,
            bg_receiver,
            fetch_spinner: crate::ui::widgets::spinner::Spinner::new("Fetching..."),
            is_fetching: false,
        })
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
                    // Close log modal and trigger rerun confirmation
                    if let Some(log_modal) = &self.log_modal {
                        let workflow_id = log_modal.job.workflow_id.clone();
                        self.log_modal = None;

                        // Find the workflow name from the pipeline detail screen
                        if let Some(detail) = &self.pipeline_detail_screen {
                            if let Some(workflow) =
                                detail.workflows.iter().find(|w| w.id == workflow_id)
                            {
                                // Show confirmation modal for rerunning the workflow
                                self.confirm_modal = Some(ConfirmModal::new(format!(
                                    "Rerun workflow: {}?",
                                    workflow.name
                                )));
                                // Store the workflow_id for when user confirms
                                if let Some(d) = &mut self.pipeline_detail_screen {
                                    d.confirm_workflow_id = Some(workflow_id);
                                }
                            }
                        }
                    }
                }
                ModalAction::CopyStepLogs => {
                    self.status_message =
                        Some(StatusMessage::info("Step logs copied to clipboard"));
                }
                ModalAction::None => {
                    // Continue showing modal
                }
            }

            // Modal consumes all input when open
            return Ok(());
        }

        // Priority 1.5: If SSH modal is open, handle SSH modal input
        if let Some(modal) = &mut self.ssh_modal {
            match modal.handle_input(key) {
                SshAction::Close => {
                    self.ssh_modal = None;
                }
                SshAction::None => {
                    // Continue showing modal
                }
            }
            // Modal consumes all input when open
            return Ok(());
        }

        // Priority 1.6: If confirmation modal is open, handle confirm input
        if let Some(modal) = &mut self.confirm_modal {
            match modal.handle_input(key) {
                ConfirmAction::Yes => {
                    // User confirmed - execute rerun workflow
                    self.confirm_modal = None;
                    // Trigger rerun if we have a workflow_id stored
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        if let Some(workflow_id) = &detail.confirm_workflow_id {
                            // Set status message - the actual API call should be made asynchronously
                            self.status_message = Some(StatusMessage::info(format!(
                                "Rerunning workflow {}...",
                                workflow_id
                            )));
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
            Screen::PipelineDetail => {
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
                        PipelineDetailAction::OpenSsh(job) => {
                            self.open_ssh_modal(job);
                        }
                        PipelineDetailAction::LoadMoreJobs => {
                            // Trigger load more jobs for the current workflow
                            if let Some(detail) = &self.pipeline_detail_screen {
                                if !detail.workflows.is_empty() {
                                    let workflow_id =
                                        detail.workflows[detail.selected_workflow_index].id.clone();
                                    // Set loading state and trigger async load
                                    if let Some(d) = &mut self.pipeline_detail_screen {
                                        d.loading_more_jobs = true;
                                    }
                                    self.pending_load_more_jobs = Some(workflow_id);
                                }
                            }
                        }
                        PipelineDetailAction::LoadJobs(workflow_id) => {
                            // Trigger job loading
                            self.trigger_job_load(workflow_id);
                        }
                        PipelineDetailAction::RerunWorkflow(workflow_id) => {
                            // Show confirmation modal
                            if let Some(detail) = &self.pipeline_detail_screen {
                                let workflow = detail
                                    .workflows
                                    .iter()
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
                        PipelineDetailAction::CopyLogs(_job_number) => {
                            // Copy logs action - the screen handles the copy internally,
                            // we just need to check if there's a pending log fetch
                            // This will be checked in the async processing loop below
                        }
                        PipelineDetailAction::FetchFailedJobsLogs(failed_jobs) => {
                            self.is_fetching = true;
                            self.fetch_spinner = crate::ui::widgets::spinner::Spinner::new(
                                format!("Fetching logs for {} failed job(s)...", failed_jobs.len()),
                            );
                            self.status_message = Some(StatusMessage::pending(format!(
                                "⠋ Fetching logs for {} failed job(s)...",
                                failed_jobs.len()
                            )));
                            self.spawn_failed_jobs_log_fetch(failed_jobs);
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
        let area = f.area();

        // Check if status message expired and remove it
        if let Some(ref msg) = self.status_message {
            if msg.is_expired() {
                self.status_message = None;
            }
        }

        // Split layout: main content + status bar (if present) at bottom
        let chunks = if self.status_message.is_some() {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),    // Main content
                    Constraint::Length(1), // Status bar
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),    // Main content
                    Constraint::Length(0), // No status bar
                ])
                .split(area)
        };

        // Determine the main content area
        let main_area = chunks[0];

        // Render status message at bottom if present
        if let Some(ref msg) = self.status_message {
            f.render_widget(msg.render(), chunks[1]);
        }

        // Render base screen
        match &self.current_screen {
            Screen::Pipelines => {
                self.pipeline_screen.render(f, main_area);
            }
            Screen::PipelineDetail => {
                if let Some(detail) = &mut self.pipeline_detail_screen {
                    detail.render(f, main_area);
                }
            }
        }

        // Render modal on top if open
        if let Some(modal) = &mut self.log_modal {
            modal.render(f, area);
        }
        // Render SSH modal on top if open
        if let Some(modal) = &self.ssh_modal {
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
        let mut detail_screen = PipelineDetailScreen::new(pipeline.clone());

        // Apply saved filter preferences
        detail_screen.apply_filter_preferences(&self.preferences.get_preferences().detail_filters);

        // Set loading state
        detail_screen.loading_workflows = true;

        self.pipeline_detail_screen = Some(detail_screen);

        // Trigger async workflow loading
        self.pending_workflow_load = Some(pipeline.id.clone());
        self.current_screen = Screen::PipelineDetail;
    }

    /// Navigate back to the pipelines screen (Screen 1)
    ///
    /// Returns to the main pipelines list view.
    pub fn navigate_back_to_pipelines(&mut self) {
        self.current_screen = Screen::Pipelines;
        self.pipeline_detail_screen = None;
        // Close any open modals when navigating back
        self.log_modal = None;
        self.ssh_modal = None;
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

    /// Open the SSH modal (overlay)
    ///
    /// Shows the SSH command for a job as a modal overlay on top of the current screen.
    pub fn open_ssh_modal(&mut self, job: Job) {
        self.ssh_modal = Some(SshModal::new(job));
    }

    /// Load pipelines from the API
    ///
    /// This is an async method that fetches pipelines and updates the screen state.
    pub async fn load_pipelines(&mut self) -> Result<()> {
        self.is_loading = true;

        // Extract filters from pipeline screen
        let owner_filter = self
            .pipeline_screen
            .faceted_search
            .get_filter_value(0)
            .unwrap_or("All pipelines");
        let branch_filter = self.pipeline_screen.faceted_search.get_filter_value(1);

        // Convert filters to API parameters (server-side filtering)
        let mine = owner_filter == "Mine";
        let branch = if branch_filter == Some("All") {
            None
        } else {
            branch_filter
        };

        // Fetch pipelines from API with server-side filters
        // mine=true: Only pipelines I triggered (pushed, ran, etc.)
        // branch=X: Only pipelines on specific branch
        let pipelines = self
            .api_client
            .get_pipelines_filtered(100, branch, mine)
            .await?;

        // Update pipeline screen with new data
        self.pipeline_screen.set_pipelines(pipelines.clone());

        self.is_loading = false;

        // Pre-fetch workflows for all pipelines in the background
        self.prefetch_workflows(pipelines).await;

        Ok(())
    }

    /// Pre-fetch workflows for all pipelines
    ///
    /// This method fetches workflows for all pipelines in parallel and stores them
    /// in the pipeline_workflows HashMap. Errors are silently ignored to prevent
    /// blocking the UI.
    async fn prefetch_workflows(&mut self, pipelines: Vec<Pipeline>) {
        use std::collections::HashMap;

        // Set loading state
        self.pipeline_screen.loading_workflows = true;

        // Extract pipeline IDs
        let pipeline_ids: Vec<String> = pipelines.iter().map(|p| p.id.clone()).collect();

        // Fetch workflows in parallel
        let api_client = Arc::clone(&self.api_client);
        let mut tasks = Vec::new();

        for pipeline_id in pipeline_ids {
            let client = Arc::clone(&api_client);
            let task = tokio::spawn(async move {
                let workflows = client.get_workflows(&pipeline_id).await;
                (pipeline_id, workflows)
            });
            tasks.push(task);
        }

        // Collect results
        let mut pipeline_workflows = HashMap::new();
        for task in tasks {
            if let Ok((pipeline_id, Ok(workflows))) = task.await {
                pipeline_workflows.insert(pipeline_id, workflows);
                // Silently ignore errors - workflows will show loading state
            }
        }

        // Update pipeline screen with workflows
        self.pipeline_screen
            .set_pipeline_workflows(pipeline_workflows);
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

    /// Trigger prefetch for visible jobs
    pub fn trigger_prefetch(&mut self, viewport_height: u16) {
        if let Some(detail) = &self.pipeline_detail_screen {
            let visible_jobs = detail.get_visible_job_numbers(viewport_height);

            // Get full job details for status checking
            let jobs: Vec<crate::api::models::Job> = detail.jobs.clone();

            // Cancel tasks for jobs no longer visible
            let previous = &detail.previous_visible_jobs;
            let to_cancel: Vec<u32> = previous
                .iter()
                .filter(|j| !visible_jobs.contains(j))
                .cloned()
                .collect();
            self.prefetch_coordinator.cancel_jobs(to_cancel);

            // Start prefetch for visible jobs
            self.prefetch_coordinator
                .prefetch_jobs(visible_jobs.clone(), jobs);

            // Update tracking
            if let Some(detail) = &mut self.pipeline_detail_screen {
                detail.previous_visible_jobs = visible_jobs;
            }
        }
    }

    /// Poll prefetch results (non-blocking)
    pub fn process_prefetch_results(&mut self) {
        let results = self.prefetch_coordinator.poll_results();
        for result in results {
            // Results are already cached by the worker
            // Optionally log errors, but don't block UI
            if let Err(_e) = result.result {
                // Could show error in status message if desired
                // For now, silently continue - prefetch is best-effort
            }
        }
    }

    /// Trigger job loading for the selected workflow
    pub fn trigger_job_load(&mut self, workflow_id: String) {
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.loading_jobs = true;
        }
        self.pending_job_load = Some(workflow_id);
    }

    /// Tick powerline to handle notification expiry
    pub fn tick_powerline(&mut self) {
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.tick_powerline();
        }
    }

    /// Spawn a background task to load job logs (non-blocking)
    pub fn spawn_log_load(&mut self, job_number: u32) {
        use crate::cache::log_cache::CacheStatus;

        // Check cache first (sync, fast)
        if let Ok(CacheStatus::Valid(entry)) = self.log_cache_manager.get(job_number) {
            if let Some(modal) = &mut self.log_modal {
                modal.set_logs(entry.logs);
            }
            return;
        }

        // Cache miss - use step-based streaming
        let job_status = self.log_modal.as_ref().map(|m| m.job.status.clone());

        let client = Arc::clone(&self.api_client);
        let tx = self.bg_sender.clone();

        // Create a channel for step stream messages
        let (step_tx, mut step_rx) = mpsc::unbounded_channel::<StepStreamMessage>();

        // Forward step stream messages to the main bg channel
        let forward_tx = tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = step_rx.recv().await {
                match msg {
                    StepStreamMessage::StepsDiscovered(steps) => {
                        let _ = forward_tx.send(BgTaskResult::LogStepsDiscovered { steps });
                    }
                    StepStreamMessage::StepLogsFetched { step_index, logs } => {
                        let _ = forward_tx.send(BgTaskResult::LogStepFetched { step_index, logs });
                    }
                }
            }
        });

        tokio::spawn(async move {
            match client.stream_job_steps_and_logs(job_number, step_tx).await {
                Ok(()) => {
                    let _ = tx.send(BgTaskResult::LogsComplete {
                        job_number,
                        job_status,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgTaskResult::LogsError(e.into()));
                }
            }
        });
    }

    /// Spawn a background task to load workflows (non-blocking)
    pub fn spawn_workflow_load(&mut self, pipeline_id: String) {
        self.is_loading = true;
        let client = Arc::clone(&self.api_client);
        let tx = self.bg_sender.clone();
        tokio::spawn(async move {
            match client.get_workflows(&pipeline_id).await {
                Ok(workflows) => {
                    let _ = tx.send(BgTaskResult::WorkflowsLoaded(workflows));
                }
                Err(e) => {
                    let _ = tx.send(BgTaskResult::WorkflowsError(e.into()));
                }
            }
        });
    }

    /// Spawn a background task to load jobs (non-blocking)
    pub fn spawn_job_load(&mut self, workflow_id: String) {
        self.is_loading = true;
        let client = Arc::clone(&self.api_client);
        let tx = self.bg_sender.clone();
        tokio::spawn(async move {
            match client.get_jobs(&workflow_id).await {
                Ok((jobs, next_page_token)) => {
                    let _ = tx.send(BgTaskResult::JobsLoaded {
                        jobs,
                        next_page_token,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgTaskResult::JobsError(e.into()));
                }
            }
        });
    }

    /// Spawn a background task to load more jobs (non-blocking)
    pub fn spawn_more_jobs_load(&mut self, workflow_id: String, page_token: String) {
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.loading_more_jobs = true;
        }
        let client = Arc::clone(&self.api_client);
        let tx = self.bg_sender.clone();
        tokio::spawn(async move {
            match client.get_jobs_page(&workflow_id, Some(&page_token)).await {
                Ok((jobs, next_page_token)) => {
                    let _ = tx.send(BgTaskResult::MoreJobsLoaded {
                        jobs,
                        next_page_token,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgTaskResult::MoreJobsError(e.into()));
                }
            }
        });
    }

    /// Spawn a background task to fetch powerline logs (non-blocking)
    pub fn spawn_powerline_log_load(&mut self, job_number: u32) {
        use crate::cache::log_cache::CacheStatus;

        // Clear the pending flag immediately
        if let Some(detail) = &mut self.pipeline_detail_screen {
            detail.pending_log_fetch = None;
        }

        // Check cache first (sync, fast)
        if let Ok(CacheStatus::Valid(entry)) = self.log_cache_manager.get(job_number) {
            if let Some(detail) = &mut self.pipeline_detail_screen {
                detail.set_logs_for_job(job_number, entry.logs);
            }
            return;
        }

        // Get job status for caching later
        let job_status = self.pipeline_detail_screen.as_ref().and_then(|detail| {
            detail
                .jobs
                .iter()
                .find(|j| j.job_number == job_number)
                .map(|j| j.status.clone())
        });

        let client = Arc::clone(&self.api_client);
        let tx = self.bg_sender.clone();
        tokio::spawn(async move {
            match client.stream_job_log(job_number).await {
                Ok(logs) => {
                    let _ = tx.send(BgTaskResult::PowerlineLogsLoaded {
                        job_number,
                        logs,
                        job_status,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgTaskResult::PowerlineLogsError(e.into()));
                }
            }
        });
    }

    pub fn spawn_failed_jobs_log_fetch(&mut self, failed_jobs: Vec<(u32, String)>) {
        use crate::ui::widgets::log_modal::LogModal;
        let client = Arc::clone(&self.api_client);
        let tx = self.bg_sender.clone();
        let cwd = std::env::current_dir().ok();
        tokio::spawn(async move {
            use chrono::Local;
            let timestamp = Local::now().format("%Y-%m-%dT%H-%M-%S").to_string();

            struct JobResult {
                name: String,
                reproduce: String,
                summary_logs: String,
                full_logs: String,
                full_filename: String,
            }

            // Fetch all jobs in parallel
            let fetch_tasks: Vec<_> = failed_jobs
                .iter()
                .map(|(job_number, job_name)| {
                    let client = Arc::clone(&client);
                    let job_number = *job_number;
                    let job_name = job_name.clone();
                    tokio::spawn(async move {
                        // Get steps to find failed step's command (first log line of failed action)
                        let reproduce = async {
                            let steps = client.get_job_steps(job_number).await.ok()?;
                            let failed_step = steps.iter().find(|s| s.status == "failed")?;
                            let output_url = failed_step.actions.iter()
                                .find(|a| a.status == "failed")
                                .and_then(|a| a.output_url.as_ref())?;
                            let lines = client.fetch_log_output_pub(output_url).await.ok()?;
                            lines.first().map(|l| LogModal::strip_ansi_pub(l).trim().to_string())
                        }.await.unwrap_or_default();

                        let fetch = tokio::time::timeout(
                            std::time::Duration::from_secs(30),
                            client.stream_job_log(job_number),
                        )
                        .await;
                        let logs = match fetch {
                            Ok(Ok(l)) => l,
                            Ok(Err(e)) => vec![format!("(fetch error: {})", e)],
                            Err(_) => vec!["(timed out)".to_string()],
                        };
                        (job_name, reproduce, logs)
                    })
                })
                .collect();

            let fetched: Vec<(String, String, Vec<String>)> = futures::future::join_all(fetch_tasks)
                .await
                .into_iter()
                .filter_map(|r| r.ok())
                .collect();

            let mut results: Vec<JobResult> = Vec::new();

            for (job_name, reproduce, logs) in fetched {

                let clean_lines: Vec<String> =
                    logs.iter().map(|l| LogModal::strip_ansi_pub(l)).collect();

                let summary_logs = clean_lines
                    .iter()
                    .rev()
                    .take(150)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n");

                let full_logs = clean_lines.join("\n");
                let safe_name = job_name.replace('/', "-");
                let full_filename = format!("{}.md", safe_name);

                results.push(JobResult {
                    name: job_name.clone(),
                    reproduce,
                    summary_logs,
                    full_logs,
                    full_filename,
                });
            }

            // Build summary file
            let _summary_filename = "summary.md".to_string();
            // Top-level checklist
            let checklist: String = results
                .iter()
                .map(|r| format!("- [ ] fix {}", r.name))
                .collect::<Vec<_>>()
                .join("\n");

            // Per-job detail sections
            let mut detail_sections: Vec<String> = Vec::new();
            for r in &results {
                detail_sections.push(format!(
                    "## {}\n**Command:** `{}`  **Full log:** `{}`\n\n```\n{}\n```",
                    r.name, r.reproduce, r.full_filename, r.summary_logs
                ));
            }

            let delete_instruction = format!(
                "For each job fixed, amend the current commit. \
When all jobs are fixed, always ask the user for permission before running: `rm -rf ci-failures-{}`",
                timestamp
            );

            let prompt = format!(
                "You are a senior engineer. For each failing job below, identify the root cause \
and provide a concrete fix (file path, function name, exact code change). \
Only load the full log file if the summary is not enough to diagnose. \
{}\n\n",
                delete_instruction
            );

            let summary_content = format!(
                "# CI Failures - {}\n\n{}{}\n\n---\n\n{}\n",
                timestamp,
                prompt,
                checklist,
                detail_sections.join("\n\n---\n\n")
            );

            // Create run folder: ci-failures-<timestamp>/
            let base = cwd
                .as_deref()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap());
            let run_dir = base.join(format!("ci-failures-{}", timestamp));
            if let Err(e) = std::fs::create_dir_all(&run_dir) {
                let _ = tx.send(BgTaskResult::FailedJobsLogsReady(format!(
                    "Failed to create dir: {}",
                    e
                )));
                return;
            }

            // Write per-job full log files inside the folder
            for r in &results {
                let full_content = format!(
                    "# Full logs - {}\n\n**To reproduce:** `{}`\n\n```\n{}\n```\n",
                    r.name, r.reproduce, r.full_logs
                );
                let _ = std::fs::write(run_dir.join(&r.full_filename), full_content);
            }

            // Write summary inside the folder too
            let summary_path = run_dir.join("summary.md");
            if let Err(e) = std::fs::write(&summary_path, &summary_content) {
                let _ = tx.send(BgTaskResult::FailedJobsLogsReady(format!(
                    "Failed to write summary: {}",
                    e
                )));
                return;
            }

            let _ = LogModal::copy_to_clipboard_pub(&summary_content);
            let _ = tx.send(BgTaskResult::FailedJobsLogsWrittenToFile(summary_path));
        });
    }

    /// Process completed background task results (non-blocking)
    pub fn process_bg_results(&mut self) {
        // Tick spinner animation while fetching
        if self.is_fetching && self.fetch_spinner.tick() {
            let frame = self.fetch_spinner.current_frame();
            let msg = self.fetch_spinner.message().to_string();
            self.status_message = Some(StatusMessage::pending(format!("{} {}", frame, msg)));
        }

        while let Ok(result) = self.bg_receiver.try_recv() {
            match result {
                BgTaskResult::LogStepsDiscovered { steps } => {
                    if let Some(modal) = &mut self.log_modal {
                        modal.set_steps(steps);
                    }
                }
                BgTaskResult::LogStepFetched { step_index, logs } => {
                    if let Some(modal) = &mut self.log_modal {
                        modal.set_step_logs(step_index, logs);
                    }
                }
                BgTaskResult::LogsComplete {
                    job_number,
                    job_status,
                } => {
                    // Assemble flat logs from all steps for caching
                    if let Some(status) = &job_status {
                        if status != "running" {
                            if let Some(modal) = &self.log_modal {
                                let mut all_logs = Vec::new();
                                for (i, step) in modal.steps_ref().iter().enumerate() {
                                    if i > 0 {
                                        all_logs.push(String::new());
                                    }
                                    all_logs.push(step.name.clone());
                                    for line in &step.logs {
                                        if !line.is_empty() {
                                            all_logs.push(line.clone());
                                        }
                                    }
                                    all_logs.push(String::new());
                                }
                                let _ = self.log_cache_manager.put(
                                    job_number,
                                    all_logs,
                                    status.clone(),
                                );
                            }
                        }
                    }
                    if let Some(modal) = &mut self.log_modal {
                        modal.mark_loading_complete();
                    }
                }
                BgTaskResult::LogsError(e) => {
                    self.show_api_error(e);
                }
                BgTaskResult::WorkflowsLoaded(workflows) => {
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        detail.set_workflows(workflows.clone());
                        detail.loading_workflows = false;
                        if !workflows.is_empty() {
                            detail.loading_jobs = true;
                            self.pending_job_load = Some(workflows[0].id.clone());
                        }
                    }
                    self.is_loading = false;
                }
                BgTaskResult::WorkflowsError(e) => {
                    self.show_api_error(e);
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        detail.loading_workflows = false;
                    }
                    self.is_loading = false;
                }
                BgTaskResult::JobsLoaded {
                    jobs,
                    next_page_token,
                } => {
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        detail.set_jobs_with_pagination(jobs, next_page_token, None);
                        detail.loading_jobs = false;
                    }
                    self.is_loading = false;
                }
                BgTaskResult::JobsError(e) => {
                    self.show_api_error(e);
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        detail.loading_jobs = false;
                    }
                    self.is_loading = false;
                }
                BgTaskResult::MoreJobsLoaded {
                    jobs,
                    next_page_token,
                } => {
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        detail.append_jobs(jobs, next_page_token);
                        detail.loading_more_jobs = false;
                    }
                }
                BgTaskResult::MoreJobsError(e) => {
                    self.show_api_error(e);
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        detail.loading_more_jobs = false;
                    }
                }
                BgTaskResult::PowerlineLogsLoaded {
                    job_number,
                    logs,
                    job_status,
                } => {
                    // Cache the result if job is completed
                    if let Some(status) = &job_status {
                        if status != "running" {
                            let _ = self.log_cache_manager.put(
                                job_number,
                                logs.clone(),
                                status.clone(),
                            );
                        }
                    }
                    if let Some(detail) = &mut self.pipeline_detail_screen {
                        detail.set_logs_for_job(job_number, logs);
                    }
                }
                BgTaskResult::PowerlineLogsError(e) => {
                    self.show_api_error(e);
                }
                BgTaskResult::FailedJobsLogsReady(err) => {
                    self.is_fetching = false;
                    self.status_message = Some(StatusMessage::error(err));
                }
                BgTaskResult::FailedJobsLogsWrittenToFile(path) => {
                    self.is_fetching = false;
                    let folder = path
                        .parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("?");
                    self.status_message = Some(StatusMessage::info(format!(
                        "Saved to {}/summary.md",
                        folder
                    )));
                }
            }
        }
    }

    pub fn save_preferences(&mut self) -> Result<()> {
        // Extract current filter state from pipeline screen (Screen 1)
        let filter_prefs = self.pipeline_screen.get_filter_preferences();
        self.preferences.get_preferences_mut().pipeline_filters = filter_prefs;

        // Extract current filter state from detail screen (Screen 2) if active
        if let Some(detail) = &self.pipeline_detail_screen {
            let detail_prefs = detail.get_filter_preferences();
            self.preferences.get_preferences_mut().detail_filters = detail_prefs;
        }

        // Save to disk
        self.preferences.save()
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

        // Determine title and add helpful hints based on error type
        let (title, user_message, hints) = if error_message.contains("timeout") {
            (
                "Timeout Error",
                "The request to CircleCI API timed out",
                vec![
                    "Check your network connection",
                    "CircleCI API might be experiencing issues",
                    "Try again in a moment",
                ],
            )
        } else if error_message.contains("404") || error_message.contains("Not Found") {
            (
                "Not Found",
                "The requested resource was not found",
                vec![
                    "Verify your project slug is correct (gh/owner/repo)",
                    "Check that the pipeline/workflow/job still exists",
                    "You may need to refresh the pipeline list",
                ],
            )
        } else if error_message.contains("401") || error_message.contains("Unauthorized") {
            (
                "Authentication Error",
                "Failed to authenticate with CircleCI API",
                vec![
                    "Check your CIRCLECI_TOKEN environment variable",
                    "Verify your token is valid and not expired",
                    "Ensure your token has the required permissions",
                ],
            )
        } else if error_message.contains("403") || error_message.contains("Forbidden") {
            (
                "Permission Denied",
                "You don't have permission to access this resource",
                vec![
                    "Verify your CircleCI token has access to this project",
                    "Check that you're a member of the organization",
                    "Your token may need additional permissions",
                ],
            )
        } else if error_message.contains("connect")
            || error_message.contains("network")
            || error_message.contains("dns")
        {
            (
                "Network Error",
                "Failed to connect to CircleCI API",
                vec![
                    "Check your internet connection",
                    "Verify you can reach circleci.com",
                    "Check if you're behind a proxy or firewall",
                ],
            )
        } else if error_message.contains("rate limit") || error_message.contains("429") {
            (
                "Rate Limited",
                "Too many requests to CircleCI API",
                vec![
                    "Wait a moment before trying again",
                    "CircleCI has rate limits on API usage",
                    "Consider reducing polling frequency",
                ],
            )
        } else if error_message.contains("500")
            || error_message.contains("502")
            || error_message.contains("503")
        {
            (
                "Server Error",
                "CircleCI API returned a server error",
                vec![
                    "CircleCI may be experiencing issues",
                    "Check https://status.circleci.com/",
                    "Try again in a few minutes",
                ],
            )
        } else {
            (
                "API Error",
                "An error occurred while communicating with CircleCI",
                vec![
                    "Check the error details below for more information",
                    "Verify your configuration is correct",
                    "Try refreshing or restarting the application",
                ],
            )
        };

        // Format hints as a bulleted list
        let hints_text = hints
            .iter()
            .map(|hint| format!("  • {}", hint))
            .collect::<Vec<_>>()
            .join("\n");

        // Combine original error with hints
        let detailed_error = format!(
            "{}\n\nWhat to try:\n{}\n\nTechnical details:\n{}",
            user_message, hints_text, error_details
        );

        self.error_modal = Some(
            ErrorModal::with_details(title.to_string(), user_message.to_string(), detailed_error)
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

    #[tokio::test]
    async fn test_app_creation() {
        let config = create_test_config();
        let app = App::new(config).await.unwrap();

        assert!(matches!(app.current_screen, Screen::Pipelines));
        assert!(!app.should_quit);
        assert!(app.pipeline_detail_screen.is_none());
        assert!(app.log_modal.is_none());
        assert!(app.error_modal.is_none());
    }

    #[tokio::test]
    async fn test_navigation_to_pipeline_detail() {
        let config = create_test_config();
        let mut app = App::new(config).await.unwrap();

        let pipeline = create_test_pipeline();

        // Navigate to pipeline detail (Screen 2)
        app.navigate_to_pipeline_detail(pipeline.clone());
        assert!(matches!(app.current_screen, Screen::PipelineDetail));
        assert!(app.pipeline_detail_screen.is_some());
        assert!(app.log_modal.is_none());

        // Navigate back to pipelines (Screen 1)
        app.navigate_back_to_pipelines();
        assert!(matches!(app.current_screen, Screen::Pipelines));
        assert!(app.pipeline_detail_screen.is_none());
        assert!(app.log_modal.is_none());
    }

    #[tokio::test]
    async fn test_log_modal_operations() {
        let config = create_test_config();
        let mut app = App::new(config).await.unwrap();

        let job = create_test_job();

        // Open log modal
        app.open_job_log_modal(job.clone());
        assert!(app.log_modal.is_some());
        assert!(matches!(app.current_screen, Screen::Pipelines));

        // Close log modal
        app.close_log_modal();
        assert!(app.log_modal.is_none());
    }

    #[tokio::test]
    async fn test_navigation_closes_modal() {
        let config = create_test_config();
        let mut app = App::new(config).await.unwrap();

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

    #[tokio::test]
    async fn test_two_screen_architecture() {
        let config = create_test_config();
        let mut app = App::new(config).await.unwrap();

        let pipeline = create_test_pipeline();

        // Start on Screen 1 (Pipelines)
        assert!(matches!(app.current_screen, Screen::Pipelines));

        // Navigate to Screen 2 (Pipeline Detail)
        app.navigate_to_pipeline_detail(pipeline.clone());
        assert!(matches!(app.current_screen, Screen::PipelineDetail));

        // There are only 2 screens now, no third screen
        assert!(app.pipeline_detail_screen.is_some());

        // Navigate back to Screen 1
        app.navigate_back_to_pipelines();
        assert!(matches!(app.current_screen, Screen::Pipelines));
    }

    #[tokio::test]
    async fn test_error_modal_operations() {
        let config = create_test_config();
        let mut app = App::new(config).await.unwrap();

        // Show simple error
        app.show_error("Test Error", "Something went wrong");
        assert!(app.error_modal.is_some());

        // Close error modal
        app.error_modal = None;
        assert!(app.error_modal.is_none());
    }

    #[tokio::test]
    async fn test_error_modal_with_details() {
        let config = create_test_config();
        let mut app = App::new(config).await.unwrap();

        // Show error with details
        app.show_error_with_details("API Error", "Request failed", "Details here");
        assert!(app.error_modal.is_some());
        if let Some(modal) = &app.error_modal {
            assert!(modal.is_visible());
        }
    }
}
