# Integration Tests

This directory contains integration tests for the CircleCI TUI application. These tests verify that the API client correctly handles various scenarios when communicating with the CircleCI API.

## Test Structure

### `integration_test.rs`
Comprehensive integration tests for individual API client methods using HTTP mocking with `mockito`.

**Test Categories:**

1. **Success Cases** - Tests that verify correct behavior when API calls succeed:
   - `test_get_pipelines_success` - Fetches and parses pipeline data
   - `test_get_workflows_success` - Fetches workflow data for a pipeline
   - `test_get_jobs_success` - Fetches job data for a workflow
   - `test_rerun_workflow_success` - Triggers a workflow rerun

2. **HTTP Error Cases** - Tests that verify correct error handling for different HTTP status codes:
   - `test_get_pipelines_404_not_found` - Handles missing resources
   - `test_get_workflows_403_forbidden` - Handles permission errors
   - `test_get_jobs_429_rate_limit` - Handles rate limiting
   - `test_get_pipelines_500_server_error` - Handles server errors
   - `test_rerun_workflow_401_unauthorized` - Handles authentication errors

3. **Malformed JSON Cases** - Tests that verify correct error handling when API returns invalid JSON:
   - `test_get_pipelines_malformed_json` - Handles invalid pipeline JSON
   - `test_get_workflows_malformed_json` - Handles invalid workflow JSON
   - `test_get_jobs_malformed_json` - Handles invalid job JSON

4. **Edge Cases** - Tests that verify correct handling of boundary conditions:
   - `test_get_pipelines_empty_response` - Handles empty result sets
   - `test_get_workflows_empty_response` - Handles workflows with no items
   - `test_get_jobs_empty_response` - Handles jobs with no items
   - `test_get_pipelines_with_pagination` - Handles multi-page results
   - `test_get_jobs_with_pagination_token` - Verifies pagination token handling
   - `test_pipelines_with_missing_optional_fields` - Handles missing VCS/trigger info
   - `test_status_mapping` - Verifies correct status translation
   - `test_jobs_duration_calculation` - Verifies duration calculation logic
   - `test_pipelines_revision_shortening` - Verifies SHA shortening to 7 chars

### `workflow_test.rs`
End-to-end workflow tests that simulate complete user journeys through the API.

**Test Scenarios:**

1. **Happy Path** - `test_complete_pipeline_to_jobs_flow`:
   - Fetches pipelines
   - Selects a pipeline and fetches its workflows
   - Selects a workflow and fetches its jobs
   - Verifies data integrity throughout the chain

2. **Failure Scenario** - `test_complete_flow_with_failed_pipeline`:
   - Tests flow with a failed pipeline
   - Verifies status propagation through workflows and jobs
   - Checks handling of canceled/blocked jobs

3. **Pagination Scenario** - `test_complete_flow_with_pagination`:
   - Tests pagination across multiple API endpoints
   - Verifies correct handling of page tokens
   - Ensures all data is retrieved across pages

4. **Rerun Scenario** - `test_complete_flow_with_workflow_rerun`:
   - Tests the complete flow from viewing to rerunning a workflow
   - Verifies rerun API interaction

## Running Tests

### Run all integration tests
```bash
cargo test --test integration_test
```

### Run all end-to-end workflow tests
```bash
cargo test --test workflow_test
```

### Run all tests in the tests directory
```bash
cargo test --tests
```

### Run a specific test
```bash
cargo test --test integration_test test_get_pipelines_success
```

### Run tests with output
```bash
cargo test --test integration_test -- --nocapture
```

### Run tests with multiple threads
```bash
cargo test --test integration_test -- --test-threads=4
```

## Test Coverage

The test suite covers:

- **API Success Cases**: 4 tests
- **HTTP Error Cases**: 5 tests
- **Malformed JSON Cases**: 3 tests
- **Edge Cases**: 9 tests
- **End-to-End Workflows**: 4 tests

**Total: 25+ integration tests**

## Adding New Tests

### Adding a new API method test

1. **Create a helper function** for mock responses if needed:
```rust
fn mock_new_endpoint_response() -> serde_json::Value {
    serde_json::json!({
        "field": "value"
    })
}
```

2. **Write the test**:
```rust
#[tokio::test]
async fn test_new_endpoint_success() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    let mock = server
        .mock("GET", "/new-endpoint")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(mock_new_endpoint_response().to_string())
        .create();

    let result = client.call_new_endpoint().await;
    mock.assert();

    assert!(result.is_ok());
    // Add assertions...
}
```

3. **Add corresponding error tests**:
   - Test 404, 403, 500 status codes
   - Test malformed JSON
   - Test empty responses
   - Test edge cases specific to the endpoint

### Adding a new workflow test

1. **Identify the user journey** you want to test
2. **Mock all required API endpoints** in sequence
3. **Execute the flow** step by step
4. **Verify results** at each step

Example template:
```rust
#[tokio::test]
async fn test_new_workflow() {
    let mut server = create_mock_server().await;
    let client = create_test_client(&server);

    // Mock endpoints
    let mock1 = server.mock("GET", "/endpoint1")...create();
    let mock2 = server.mock("GET", "/endpoint2")...create();

    // Execute flow
    let step1_result = client.step1().await.unwrap();
    mock1.assert();

    let step2_result = client.step2(&step1_result.id).await.unwrap();
    mock2.assert();

    // Verify
    assert_eq!(step2_result.status, "expected");
}
```

## Test Helpers

### `create_mock_server()`
Creates a new mockito test server for HTTP mocking.

### `create_test_client(server)`
Creates a test CircleCI client that points to the mock server instead of the real API.

### Mock Response Helpers
- `mock_pipeline_response()` - Valid pipeline API response
- `mock_workflow_response()` - Valid workflow API response
- `mock_job_response()` - Valid job API response
- `mock_job_steps_response()` - Valid job steps response (v1.1 API)

## Test Patterns

### Pattern 1: Success Case
```rust
let mock = server.mock("GET", "/endpoint")
    .with_status(200)
    .with_body(json_response.to_string())
    .create();

let result = client.method().await;
mock.assert();
assert!(result.is_ok());
```

### Pattern 2: Error Case
```rust
let mock = server.mock("GET", "/endpoint")
    .with_status(404)
    .with_body("Not found")
    .create();

let result = client.method().await;
mock.assert();
assert!(result.is_err());
match result.unwrap_err() {
    ApiError::Http(status, _) => assert_eq!(status, 404),
    _ => panic!("Wrong error type"),
}
```

### Pattern 3: Pagination
```rust
// First page with next_page_token
let page1_mock = server.mock("GET", "/endpoint")
    .with_body(r#"{"items":[...], "next_page_token":"page2"}"#)
    .create();

// Second page without next_page_token
let page2_mock = server.mock("GET", "/endpoint")
    .match_query(Matcher::UrlEncoded("page-token".into(), "page2".into()))
    .with_body(r#"{"items":[...], "next_page_token":null}"#)
    .create();

let result = client.method().await;
page1_mock.assert();
page2_mock.assert();
```

## Debugging Tests

### View test output
```bash
cargo test --test integration_test -- --nocapture
```

### Run a single test with debug output
```bash
RUST_LOG=debug cargo test --test integration_test test_name -- --nocapture
```

### Check which mocks were called
Mockito automatically reports which mocks were not called when `mock.assert()` fails.

## CI/CD Integration

These tests are designed to run in CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Run integration tests
  run: |
    cargo test --tests
    cargo test --test integration_test
    cargo test --test workflow_test
```

## Dependencies

- **mockito**: HTTP mocking library for testing
- **tokio**: Async runtime required for async tests
- **serde_json**: JSON manipulation for test data

## Best Practices

1. **Keep tests isolated** - Each test should be independent
2. **Use descriptive names** - Test names should clearly describe what they test
3. **Test edge cases** - Don't just test the happy path
4. **Mock realistically** - Mock responses should match actual API responses
5. **Assert thoroughly** - Verify all important fields, not just that the call succeeded
6. **Document complex tests** - Add comments explaining non-obvious test logic

## Troubleshooting

### Tests fail with "connection refused"
- Mockito server might not be starting correctly
- Check that `create_mock_server().await` is being called

### Tests hang indefinitely
- Missing `.await` on async call
- Tokio runtime not configured correctly (should use `#[tokio::test]`)

### Mock not being matched
- Check URL path matches exactly
- Check query parameters if using pagination
- Check request headers if required
- Use `mock.assert()` to see detailed error message

### JSON parsing errors
- Verify mock response JSON is valid
- Check that all required fields are present
- Verify field types match struct definitions

## Future Improvements

Potential areas for test expansion:

1. **Performance tests** - Test with large result sets
2. **Concurrent request tests** - Test multiple simultaneous requests
3. **Retry logic tests** - Test exponential backoff on failures
4. **Timeout tests** - Verify timeout handling
5. **Authentication tests** - Test token refresh scenarios
6. **Logging tests** - Verify log streaming functionality
