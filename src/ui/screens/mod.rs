/// Screen modules for different views
pub mod pipeline_detail;
pub mod pipelines;
pub mod workflow;
pub mod workflows_list;

pub use pipeline_detail::{PanelFocus, PipelineDetailAction, PipelineDetailScreen};
pub use pipelines::PipelineScreen;
pub use workflow::{NavigationAction, WorkflowScreen};
pub use workflows_list::WorkflowsListScreen;
