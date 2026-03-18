use circleci_tui_rs::api::client::CircleCIClient;
use mockito::{Server, ServerGuard};

/// Helper function to create a mock CircleCI server
async fn create_mock_server() -> ServerGuard {
    Server::new_async().await
}

/// Helper function to create a test client pointing at the mock server
fn create_test_client(server: &ServerGuard) -> CircleCIClient {
    CircleCIClient::new_with_base_url(
        "test-token".to_string(),
        "gh/test-org/test-repo".to_string(),
        server.url(),
    )
    .expect("Failed to create test client")
}

/// End-to-end test: Complete pipeline → workflows → jobs flow
///
/// This test simulates the full user journey:
/// 1. Fetch pipelines
/// 2. Select a pipeline and fetch its workflows
/// 3. Select a workflow and fetch its jobs
#[tokio::test]
async fn test_complete_pipeline_to_jobs_flow() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    // Step 1: Mock GET pipelines
    let pipelines_response = serde_json::json!({
        "items": [
            {
                "id": "pipe-123",
                "number": 1234,
                "state": "success",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "main",
                    "revision": "abc123def456",
                    "commit": {
                        "subject": "feat: add new feature"
                    }
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {
                        "login": "alice"
                    }
                }
            },
            {
                "id": "pipe-124",
                "number": 1235,
                "state": "running",
                "created_at": "2024-01-15T11:00:00Z",
                "updated_at": "2024-01-15T11:15:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "feature-branch",
                    "revision": "def456ghi789",
                    "commit": {
                        "subject": "fix: bug fix"
                    }
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {
                        "login": "bob"
                    }
                }
            }
        ],
        "next_page_token": null
    });

    let pipelines_mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(pipelines_response.to_string())
        .create();

    // Step 2: Mock GET workflows for the first pipeline
    let workflows_response = serde_json::json!({
        "items": [
            {
                "id": "wf-001",
                "name": "build-and-test",
                "status": "success",
                "created_at": "2024-01-15T10:00:00Z",
                "stopped_at": "2024-01-15T10:20:00Z",
                "pipeline_id": "pipe-123"
            },
            {
                "id": "wf-002",
                "name": "deploy-staging",
                "status": "success",
                "created_at": "2024-01-15T10:20:00Z",
                "stopped_at": "2024-01-15T10:30:00Z",
                "pipeline_id": "pipe-123"
            }
        ],
        "next_page_token": null
    });

    let workflows_mock = server
        .mock("GET", "/pipeline/pipe-123/workflow")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(workflows_response.to_string())
        .create();

    // Step 3: Mock GET jobs for the first workflow
    let jobs_response = serde_json::json!({
        "items": [
            {
                "id": "job-001",
                "name": "checkout",
                "status": "success",
                "job_number": 5001,
                "started_at": "2024-01-15T10:00:00Z",
                "stopped_at": "2024-01-15T10:01:00Z",
                "type": "docker"
            },
            {
                "id": "job-002",
                "name": "test",
                "status": "success",
                "job_number": 5002,
                "started_at": "2024-01-15T10:01:00Z",
                "stopped_at": "2024-01-15T10:05:00Z",
                "type": "docker"
            },
            {
                "id": "job-003",
                "name": "build",
                "status": "success",
                "job_number": 5003,
                "started_at": "2024-01-15T10:05:00Z",
                "stopped_at": "2024-01-15T10:15:00Z",
                "type": "machine"
            }
        ],
        "next_page_token": null
    });

    let jobs_mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_response.to_string())
        .create();

    // Execute the full workflow
    // Step 1: Get pipelines
    let pipelines = client
        .get_pipelines(10)
        .await
        .expect("Failed to get pipelines");
    pipelines_mock.assert();

    assert_eq!(pipelines.len(), 2);
    assert_eq!(pipelines[0].id, "pipe-123");
    assert_eq!(pipelines[0].number, 1234);
    assert_eq!(pipelines[0].state, "success");
    assert_eq!(pipelines[0].vcs.branch, "main");
    assert_eq!(pipelines[0].vcs.revision, "abc123d"); // Shortened SHA

    // Step 2: Get workflows for the first pipeline
    let selected_pipeline = &pipelines[0];
    let workflows = client
        .get_workflows(&selected_pipeline.id)
        .await
        .expect("Failed to get workflows");
    workflows_mock.assert();

    assert_eq!(workflows.len(), 2);
    assert_eq!(workflows[0].id, "wf-001");
    assert_eq!(workflows[0].name, "build-and-test");
    assert_eq!(workflows[0].status, "success");
    assert_eq!(workflows[0].pipeline_id, "pipe-123");

    // Step 3: Get jobs for the first workflow
    let selected_workflow = &workflows[0];
    let (jobs, next_page_token) = client
        .get_jobs(&selected_workflow.id)
        .await
        .expect("Failed to get jobs");
    jobs_mock.assert();

    assert_eq!(jobs.len(), 3);
    assert!(next_page_token.is_none());

    // Verify job details
    assert_eq!(jobs[0].id, "job-001");
    assert_eq!(jobs[0].name, "checkout");
    assert_eq!(jobs[0].status, "success");
    assert_eq!(jobs[0].job_number, 5001);
    assert_eq!(jobs[0].duration, Some(60)); // 1 minute

    assert_eq!(jobs[1].id, "job-002");
    assert_eq!(jobs[1].name, "test");
    assert_eq!(jobs[1].duration, Some(240)); // 4 minutes

    assert_eq!(jobs[2].id, "job-003");
    assert_eq!(jobs[2].name, "build");
    assert_eq!(jobs[2].duration, Some(600)); // 10 minutes
    assert_eq!(jobs[2].executor.executor_type, "machine");
}

/// Test the complete flow with a failed pipeline
///
/// This verifies that the system correctly handles failed pipelines
/// and workflows, propagating status information through the chain.
#[tokio::test]
async fn test_complete_flow_with_failed_pipeline() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    // Mock a failed pipeline
    let pipelines_response = serde_json::json!({
        "items": [
            {
                "id": "pipe-failed",
                "number": 2000,
                "state": "failed",
                "created_at": "2024-01-15T12:00:00Z",
                "updated_at": "2024-01-15T12:15:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "feature-broken",
                    "revision": "bad123commit",
                    "commit": {
                        "subject": "fix: attempted bug fix"
                    }
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {
                        "login": "charlie"
                    }
                }
            }
        ],
        "next_page_token": null
    });

    let pipelines_mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(pipelines_response.to_string())
        .create();

    // Mock workflow with mixed statuses
    let workflows_response = serde_json::json!({
        "items": [
            {
                "id": "wf-failed-001",
                "name": "build-and-test",
                "status": "failed",
                "created_at": "2024-01-15T12:00:00Z",
                "stopped_at": "2024-01-15T12:15:00Z",
                "pipeline_id": "pipe-failed"
            }
        ],
        "next_page_token": null
    });

    let workflows_mock = server
        .mock("GET", "/pipeline/pipe-failed/workflow")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(workflows_response.to_string())
        .create();

    // Mock jobs where one failed
    let jobs_response = serde_json::json!({
        "items": [
            {
                "id": "job-pass-001",
                "name": "checkout",
                "status": "success",
                "job_number": 6001,
                "started_at": "2024-01-15T12:00:00Z",
                "stopped_at": "2024-01-15T12:01:00Z",
                "type": "docker"
            },
            {
                "id": "job-fail-002",
                "name": "test",
                "status": "failed",
                "job_number": 6002,
                "started_at": "2024-01-15T12:01:00Z",
                "stopped_at": "2024-01-15T12:10:00Z",
                "type": "docker"
            },
            {
                "id": "job-canceled-003",
                "name": "build",
                "status": "canceled",
                "job_number": 6003,
                "started_at": null,
                "stopped_at": null,
                "type": "machine"
            }
        ],
        "next_page_token": null
    });

    let jobs_mock = server
        .mock("GET", "/workflow/wf-failed-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_response.to_string())
        .create();

    // Execute flow
    let pipelines = client.get_pipelines(10).await.unwrap();
    pipelines_mock.assert();
    assert_eq!(pipelines[0].state, "failed");

    let workflows = client.get_workflows(&pipelines[0].id).await.unwrap();
    workflows_mock.assert();
    assert_eq!(workflows[0].status, "failed");

    let (jobs, _) = client.get_jobs(&workflows[0].id).await.unwrap();
    jobs_mock.assert();

    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].status, "success");
    assert_eq!(jobs[1].status, "failed");
    assert_eq!(jobs[2].status, "blocked"); // canceled -> blocked
    assert_eq!(jobs[2].duration, None); // Job never ran
}

/// Test pagination across the entire flow
///
/// This verifies that pagination works correctly when fetching
/// multiple pages of pipelines and jobs.
#[tokio::test]
async fn test_complete_flow_with_pagination() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    // First page of pipelines
    let page1_response = serde_json::json!({
        "items": [
            {
                "id": "pipe-001",
                "number": 100,
                "state": "success",
                "created_at": "2024-01-15T10:00:00Z",
                "updated_at": "2024-01-15T10:30:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "main",
                    "revision": "aaa111",
                    "commit": {"subject": "First commit"}
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {"login": "user1"}
                }
            }
        ],
        "next_page_token": "token-page2"
    });

    // Second page of pipelines
    let page2_response = serde_json::json!({
        "items": [
            {
                "id": "pipe-002",
                "number": 101,
                "state": "running",
                "created_at": "2024-01-15T11:00:00Z",
                "updated_at": "2024-01-15T11:30:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "develop",
                    "revision": "bbb222",
                    "commit": {"subject": "Second commit"}
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {"login": "user2"}
                }
            }
        ],
        "next_page_token": null
    });

    let page1_mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page1_response.to_string())
        .create();

    let page2_mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .match_query(mockito::Matcher::UrlEncoded(
            "page-token".into(),
            "token-page2".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page2_response.to_string())
        .create();

    // Mock workflows for the first pipeline
    let workflows_response = serde_json::json!({
        "items": [
            {
                "id": "wf-001",
                "name": "ci",
                "status": "success",
                "created_at": "2024-01-15T10:00:00Z",
                "stopped_at": "2024-01-15T10:30:00Z",
                "pipeline_id": "pipe-001"
            }
        ],
        "next_page_token": null
    });

    let workflows_mock = server
        .mock("GET", "/pipeline/pipe-001/workflow")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(workflows_response.to_string())
        .create();

    // Mock jobs with pagination
    let jobs_page1 = serde_json::json!({
        "items": [
            {
                "id": "job-001",
                "name": "test-1",
                "status": "success",
                "job_number": 1001,
                "started_at": "2024-01-15T10:00:00Z",
                "stopped_at": "2024-01-15T10:05:00Z",
                "type": "docker"
            }
        ],
        "next_page_token": "jobs-page2"
    });

    let jobs_page2 = serde_json::json!({
        "items": [
            {
                "id": "job-002",
                "name": "test-2",
                "status": "success",
                "job_number": 1002,
                "started_at": "2024-01-15T10:05:00Z",
                "stopped_at": "2024-01-15T10:10:00Z",
                "type": "docker"
            }
        ],
        "next_page_token": null
    });

    let jobs_page1_mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page1.to_string())
        .create();

    let jobs_page2_mock = server
        .mock("GET", "/workflow/wf-001/job")
        .match_query(mockito::Matcher::UrlEncoded(
            "page-token".into(),
            "jobs-page2".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page2.to_string())
        .create();

    // Execute flow
    let pipelines = client.get_pipelines(10).await.unwrap();
    page1_mock.assert();
    page2_mock.assert();
    assert_eq!(pipelines.len(), 2);

    let workflows = client.get_workflows(&pipelines[0].id).await.unwrap();
    workflows_mock.assert();
    assert_eq!(workflows.len(), 1);

    // Get first page of jobs
    let (jobs_page1, token1) = client.get_jobs(&workflows[0].id).await.unwrap();
    jobs_page1_mock.assert();
    assert_eq!(jobs_page1.len(), 1);
    assert_eq!(token1, Some("jobs-page2".to_string()));

    // Get second page of jobs
    let (jobs_page2, token2) = client
        .get_jobs_page(&workflows[0].id, Some("jobs-page2"))
        .await
        .unwrap();
    jobs_page2_mock.assert();
    assert_eq!(jobs_page2.len(), 1);
    assert!(token2.is_none());
}

/// Test workflow rerun in the context of a complete flow
///
/// This verifies that after viewing a workflow, we can trigger
/// a rerun operation successfully.
#[tokio::test]
async fn test_complete_flow_with_workflow_rerun() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    // Mock pipelines
    let pipelines_response = serde_json::json!({
        "items": [
            {
                "id": "pipe-rerun",
                "number": 3000,
                "state": "failed",
                "created_at": "2024-01-15T14:00:00Z",
                "updated_at": "2024-01-15T14:10:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "fix-attempt",
                    "revision": "fix123",
                    "commit": {"subject": "Trying to fix"}
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {"login": "dev"}
                }
            }
        ],
        "next_page_token": null
    });

    let pipelines_mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(pipelines_response.to_string())
        .create();

    // Mock workflows
    let workflows_response = serde_json::json!({
        "items": [
            {
                "id": "wf-rerun",
                "name": "test-workflow",
                "status": "failed",
                "created_at": "2024-01-15T14:00:00Z",
                "stopped_at": "2024-01-15T14:10:00Z",
                "pipeline_id": "pipe-rerun"
            }
        ],
        "next_page_token": null
    });

    let workflows_mock = server
        .mock("GET", "/pipeline/pipe-rerun/workflow")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(workflows_response.to_string())
        .create();

    // Mock rerun endpoint
    let rerun_mock = server
        .mock("POST", "/workflow/wf-rerun/rerun")
        .match_header("content-type", "application/json")
        .match_body(r#"{"from_failed":false}"#)
        .with_status(202)
        .create();

    // Execute flow
    let pipelines = client.get_pipelines(10).await.unwrap();
    pipelines_mock.assert();

    let workflows = client.get_workflows(&pipelines[0].id).await.unwrap();
    workflows_mock.assert();

    // User decides to rerun the failed workflow
    let workflow_to_rerun = &workflows[0];
    let rerun_result = client.rerun_workflow(&workflow_to_rerun.id).await;
    rerun_mock.assert();

    assert!(rerun_result.is_ok());
}
