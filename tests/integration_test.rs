use circleci_tui_rs::api::client::CircleCIClient;
use circleci_tui_rs::api::error::ApiError;
use mockito::{Mock, Server, ServerGuard};

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

/// Helper to create a valid pipeline response JSON
fn mock_pipeline_response() -> serde_json::Value {
    serde_json::json!({
        "items": [
            {
                "id": "pipe-001",
                "number": 123,
                "state": "success",
                "created_at": "2024-01-15T10:30:00Z",
                "updated_at": "2024-01-15T10:45:00Z",
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
                        "login": "testuser"
                    }
                }
            },
            {
                "id": "pipe-002",
                "number": 124,
                "state": "running",
                "created_at": "2024-01-15T11:00:00Z",
                "updated_at": "2024-01-15T11:05:00Z",
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
                        "login": "developer"
                    }
                }
            }
        ],
        "next_page_token": null
    })
}

/// Helper to create a workflow response JSON
fn mock_workflow_response() -> serde_json::Value {
    serde_json::json!({
        "items": [
            {
                "id": "wf-001",
                "name": "build-and-test",
                "status": "success",
                "created_at": "2024-01-15T10:30:00Z",
                "stopped_at": "2024-01-15T10:45:00Z",
                "pipeline_id": "pipe-001"
            },
            {
                "id": "wf-002",
                "name": "deploy",
                "status": "running",
                "created_at": "2024-01-15T10:45:00Z",
                "stopped_at": null,
                "pipeline_id": "pipe-001"
            }
        ],
        "next_page_token": null
    })
}

/// Helper to create a job response JSON
fn mock_job_response() -> serde_json::Value {
    serde_json::json!({
        "items": [
            {
                "id": "job-001",
                "name": "test",
                "status": "success",
                "job_number": 1001,
                "started_at": "2024-01-15T10:30:00Z",
                "stopped_at": "2024-01-15T10:35:00Z",
                "type": "docker"
            },
            {
                "id": "job-002",
                "name": "build",
                "status": "running",
                "job_number": 1002,
                "started_at": "2024-01-15T10:35:00Z",
                "stopped_at": null,
                "type": "machine"
            }
        ],
        "next_page_token": null
    })
}

// ============================================================================
// SUCCESS CASE TESTS
// ============================================================================

#[tokio::test]
async fn test_get_pipelines_success() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_pipeline_response().to_string())
        .create();

    let result = client.get_pipelines(10).await;
    mock.assert();

    assert!(result.is_ok());
    let pipelines = result.unwrap();
    assert_eq!(pipelines.len(), 2);
    assert_eq!(pipelines[0].id, "pipe-001");
    assert_eq!(pipelines[0].number, 123);
    assert_eq!(pipelines[0].state, "success");
    assert_eq!(pipelines[0].vcs.branch, "main");
    assert_eq!(pipelines[1].id, "pipe-002");
}

#[tokio::test]
async fn test_get_workflows_success() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/pipeline/pipe-001/workflow")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_workflow_response().to_string())
        .create();

    let result = client.get_workflows("pipe-001").await;
    mock.assert();

    assert!(result.is_ok());
    let workflows = result.unwrap();
    assert_eq!(workflows.len(), 2);
    assert_eq!(workflows[0].id, "wf-001");
    assert_eq!(workflows[0].name, "build-and-test");
    assert_eq!(workflows[0].status, "success");
    assert_eq!(workflows[1].id, "wf-002");
}

#[tokio::test]
async fn test_get_jobs_success() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_job_response().to_string())
        .create();

    let result = client.get_jobs("wf-001").await;
    mock.assert();

    assert!(result.is_ok());
    let (jobs, next_page_token) = result.unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].id, "job-001");
    assert_eq!(jobs[0].name, "test");
    assert_eq!(jobs[0].status, "success");
    assert_eq!(jobs[0].job_number, 1001);
    assert!(next_page_token.is_none());
}

#[tokio::test]
async fn test_rerun_workflow_success() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("POST", "/workflow/wf-001/rerun")
        .match_header("content-type", "application/json")
        .match_body(r#"{"from_failed":false}"#)
        .with_status(202)
        .create();

    let result = client.rerun_workflow("wf-001").await;
    mock.assert();

    assert!(result.is_ok());
}

// ============================================================================
// ERROR CASE TESTS - HTTP STATUS CODES
// ============================================================================

#[tokio::test]
async fn test_get_pipelines_404_not_found() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(404)
        .with_body("Project not found")
        .create();

    let result = client.get_pipelines(10).await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Http(status, msg) => {
            assert_eq!(status, 404);
            assert_eq!(msg, "Project not found");
        }
        _ => panic!("Expected Http error"),
    }
}

#[tokio::test]
async fn test_get_workflows_403_forbidden() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/pipeline/pipe-001/workflow")
        .with_status(403)
        .with_body("Forbidden")
        .create();

    let result = client.get_workflows("pipe-001").await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Http(status, msg) => {
            assert_eq!(status, 403);
            assert_eq!(msg, "Forbidden");
        }
        _ => panic!("Expected Http error"),
    }
}

#[tokio::test]
async fn test_get_jobs_429_rate_limit() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(429)
        .with_body("Rate limit exceeded")
        .create();

    let result = client.get_jobs("wf-001").await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Http(status, msg) => {
            assert_eq!(status, 429);
            assert_eq!(msg, "Rate limit exceeded");
        }
        _ => panic!("Expected Http error"),
    }
}

#[tokio::test]
async fn test_get_pipelines_500_server_error() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(500)
        .with_body("Internal server error")
        .create();

    let result = client.get_pipelines(10).await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Http(status, msg) => {
            assert_eq!(status, 500);
            assert_eq!(msg, "Internal server error");
        }
        _ => panic!("Expected Http error"),
    }
}

#[tokio::test]
async fn test_rerun_workflow_401_unauthorized() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("POST", "/workflow/wf-001/rerun")
        .with_status(401)
        .with_body("Unauthorized")
        .create();

    let result = client.rerun_workflow("wf-001").await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Http(status, _) => {
            assert_eq!(status, 401);
        }
        _ => panic!("Expected Http error"),
    }
}

// ============================================================================
// ERROR CASE TESTS - MALFORMED JSON
// ============================================================================

#[tokio::test]
async fn test_get_pipelines_malformed_json() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{invalid json}")
        .create();

    let result = client.get_pipelines(10).await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Parse(msg) => {
            assert!(msg.contains("Failed to parse pipelines"));
        }
        _ => panic!("Expected Parse error"),
    }
}

#[tokio::test]
async fn test_get_workflows_malformed_json() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/pipeline/pipe-001/workflow")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("not json at all")
        .create();

    let result = client.get_workflows("pipe-001").await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Parse(msg) => {
            assert!(msg.contains("Failed to parse workflows"));
        }
        _ => panic!("Expected Parse error"),
    }
}

#[tokio::test]
async fn test_get_jobs_malformed_json() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[1, 2, 3]")
        .create();

    let result = client.get_jobs("wf-001").await;
    mock.assert();

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Parse(msg) => {
            assert!(msg.contains("Failed to parse jobs"));
        }
        _ => panic!("Expected Parse error"),
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test]
async fn test_get_pipelines_empty_response() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"items": [], "next_page_token": null}"#)
        .create();

    let result = client.get_pipelines(10).await;
    mock.assert();

    assert!(result.is_ok());
    let pipelines = result.unwrap();
    assert_eq!(pipelines.len(), 0);
}

#[tokio::test]
async fn test_get_workflows_empty_response() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/pipeline/pipe-001/workflow")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"items": [], "next_page_token": null}"#)
        .create();

    let result = client.get_workflows("pipe-001").await;
    mock.assert();

    assert!(result.is_ok());
    let workflows = result.unwrap();
    assert_eq!(workflows.len(), 0);
}

#[tokio::test]
async fn test_get_jobs_empty_response() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"items": [], "next_page_token": null}"#)
        .create();

    let result = client.get_jobs("wf-001").await;
    mock.assert();

    assert!(result.is_ok());
    let (jobs, _) = result.unwrap();
    assert_eq!(jobs.len(), 0);
}

#[tokio::test]
async fn test_get_pipelines_with_pagination() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    // First page
    let first_page = serde_json::json!({
        "items": [
            {
                "id": "pipe-001",
                "number": 123,
                "state": "success",
                "created_at": "2024-01-15T10:30:00Z",
                "updated_at": "2024-01-15T10:45:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "main",
                    "revision": "abc123",
                    "commit": {"subject": "Test"}
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {"login": "user"}
                }
            }
        ],
        "next_page_token": "page2"
    });

    // Second page
    let second_page = serde_json::json!({
        "items": [
            {
                "id": "pipe-002",
                "number": 124,
                "state": "running",
                "created_at": "2024-01-15T11:00:00Z",
                "updated_at": "2024-01-15T11:05:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "feature",
                    "revision": "def456",
                    "commit": {"subject": "Feature"}
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {"login": "dev"}
                }
            }
        ],
        "next_page_token": null
    });

    let mock1 = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(first_page.to_string())
        .create();

    let mock2 = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .match_query(mockito::Matcher::UrlEncoded(
            "page-token".into(),
            "page2".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(second_page.to_string())
        .create();

    let result = client.get_pipelines(10).await;
    mock1.assert();
    mock2.assert();

    assert!(result.is_ok());
    let pipelines = result.unwrap();
    assert_eq!(pipelines.len(), 2);
    assert_eq!(pipelines[0].id, "pipe-001");
    assert_eq!(pipelines[1].id, "pipe-002");
}

#[tokio::test]
async fn test_get_jobs_with_pagination_token() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let response_with_token = serde_json::json!({
        "items": [
            {
                "id": "job-001",
                "name": "test",
                "status": "success",
                "job_number": 1001,
                "started_at": "2024-01-15T10:30:00Z",
                "stopped_at": "2024-01-15T10:35:00Z",
                "type": "docker"
            }
        ],
        "next_page_token": "next-page-123"
    });

    let mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_with_token.to_string())
        .create();

    let result = client.get_jobs("wf-001").await;
    mock.assert();

    assert!(result.is_ok());
    let (jobs, next_page_token) = result.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(next_page_token, Some("next-page-123".to_string()));
}

#[tokio::test]
async fn test_pipelines_with_missing_optional_fields() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let minimal_pipeline = serde_json::json!({
        "items": [
            {
                "id": "pipe-001",
                "number": 123,
                "state": "success",
                "created_at": "2024-01-15T10:30:00Z",
                "updated_at": "2024-01-15T10:45:00Z",
                "project_slug": "gh/test-org/test-repo"
                // vcs and trigger are optional
            }
        ],
        "next_page_token": null
    });

    let mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(minimal_pipeline.to_string())
        .create();

    let result = client.get_pipelines(10).await;
    mock.assert();

    assert!(result.is_ok());
    let pipelines = result.unwrap();
    assert_eq!(pipelines.len(), 1);
    assert_eq!(pipelines[0].vcs.branch, "unknown");
    assert_eq!(pipelines[0].vcs.commit_subject, "No commit message");
}

#[tokio::test]
async fn test_status_mapping() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let response = serde_json::json!({
        "items": [
            {
                "id": "job-001",
                "name": "created-job",
                "status": "created",
                "job_number": 1001,
                "type": "docker"
            },
            {
                "id": "job-002",
                "name": "failing-job",
                "status": "failing",
                "job_number": 1002,
                "type": "docker"
            },
            {
                "id": "job-003",
                "name": "on-hold-job",
                "status": "on_hold",
                "job_number": 1003,
                "type": "docker"
            }
        ],
        "next_page_token": null
    });

    let mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response.to_string())
        .create();

    let result = client.get_jobs("wf-001").await;
    mock.assert();

    assert!(result.is_ok());
    let (jobs, _) = result.unwrap();
    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].status, "pending"); // created -> pending
    assert_eq!(jobs[1].status, "failed"); // failing -> failed
    assert_eq!(jobs[2].status, "blocked"); // on_hold -> blocked
}

#[tokio::test]
async fn test_jobs_duration_calculation() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let response = serde_json::json!({
        "items": [
            {
                "id": "job-001",
                "name": "completed-job",
                "status": "success",
                "job_number": 1001,
                "started_at": "2024-01-15T10:30:00Z",
                "stopped_at": "2024-01-15T10:32:00Z", // 2 minutes = 120 seconds
                "type": "docker"
            },
            {
                "id": "job-002",
                "name": "running-job",
                "status": "running",
                "job_number": 1002,
                "started_at": "2024-01-15T10:30:00Z",
                "stopped_at": null, // Still running
                "type": "docker"
            }
        ],
        "next_page_token": null
    });

    let mock = server
        .mock("GET", "/workflow/wf-001/job")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response.to_string())
        .create();

    let result = client.get_jobs("wf-001").await;
    mock.assert();

    assert!(result.is_ok());
    let (jobs, _) = result.unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].duration, Some(120)); // 2 minutes
    assert_eq!(jobs[1].duration, None); // Still running
}

#[tokio::test]
async fn test_pipelines_revision_shortening() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let response = serde_json::json!({
        "items": [
            {
                "id": "pipe-001",
                "number": 123,
                "state": "success",
                "created_at": "2024-01-15T10:30:00Z",
                "updated_at": "2024-01-15T10:45:00Z",
                "project_slug": "gh/test-org/test-repo",
                "vcs": {
                    "branch": "main",
                    "revision": "abc123def456ghi789jkl012", // Long SHA
                    "commit": {"subject": "Test"}
                },
                "trigger": {
                    "type": "webhook",
                    "actor": {"login": "user"}
                }
            }
        ],
        "next_page_token": null
    });

    let mock = server
        .mock("GET", "/project/gh/test-org/test-repo/pipeline")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response.to_string())
        .create();

    let result = client.get_pipelines(10).await;
    mock.assert();

    assert!(result.is_ok());
    let pipelines = result.unwrap();
    assert_eq!(pipelines.len(), 1);
    assert_eq!(pipelines[0].vcs.revision, "abc123d"); // First 7 characters
}
