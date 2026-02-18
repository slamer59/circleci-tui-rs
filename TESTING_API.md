# Testing the CircleCI API Integration

## Prerequisites

1. **CircleCI API Token**: You need a personal API token from CircleCI
   - Go to https://app.circleci.com/settings/user/tokens
   - Create a new token
   - Copy the token value

2. **Project Slug**: Your project identifier in format `gh/owner/repo` or `bb/owner/repo`
   - For GitHub: `gh/username/repository-name`
   - For Bitbucket: `bb/username/repository-name`

## Configuration

Create a `.env` file in the project root:

```bash
CIRCLECI_TOKEN=your_actual_token_here
PROJECT_SLUG=gh/your-org/your-repo
```

Example:
```bash
CIRCLECI_TOKEN=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0
PROJECT_SLUG=gh/acme/api-service
```

## Running the Application

### Build and Run
```bash
cargo run
```

The application will:
1. Load configuration from `.env`
2. Create API client with authentication
3. Fetch initial pipeline data (50 most recent pipelines)
4. Display the pipeline list screen

### Expected Output

On successful startup, you should see:
- List of recent pipelines with commit messages
- Real data from your CircleCI project
- Status badges (success, failed, running, etc.)
- Branch names, authors, and timestamps

### Common Issues

#### Authentication Error
```
Failed to load pipelines: HTTP error 401: Unauthorized
```
**Solution:** Check your CIRCLECI_TOKEN is valid and not expired.

#### Project Not Found
```
Failed to load pipelines: HTTP error 404: Project not found
```
**Solution:** Verify PROJECT_SLUG format is correct (e.g., `gh/owner/repo`).

#### Network Timeout
```
Failed to load pipelines: Request timeout
```
**Solution:** Check your internet connection. The default timeout is 30 seconds.

#### Rate Limiting
```
Failed to load pipelines: HTTP error 429: Too Many Requests
```
**Solution:** Wait a few minutes before trying again. CircleCI has rate limits on API calls.

## Testing Individual API Methods

You can test the API client directly using a simple test program:

Create `examples/test_api.rs`:
```rust
use circleci_tui_rs::api::client::CircleCIClient;
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let token = env::var("CIRCLECI_TOKEN")?;
    let project_slug = env::var("PROJECT_SLUG")?;

    println!("Testing CircleCI API...");
    println!("Project: {}", project_slug);

    let client = CircleCIClient::new(token, project_slug)?;

    // Test 1: Get pipelines
    println!("\n1. Fetching pipelines...");
    let pipelines = client.get_pipelines(5).await?;
    println!("   Found {} pipelines", pipelines.len());
    for p in &pipelines {
        println!("   - {} ({}): {}",
            p.number, p.state, p.vcs.commit_subject);
    }

    // Test 2: Get workflows for first pipeline
    if let Some(pipeline) = pipelines.first() {
        println!("\n2. Fetching workflows for pipeline {}...", pipeline.number);
        let workflows = client.get_workflows(&pipeline.id).await?;
        println!("   Found {} workflows", workflows.len());
        for w in &workflows {
            println!("   - {} ({})", w.name, w.status);
        }

        // Test 3: Get jobs for first workflow
        if let Some(workflow) = workflows.first() {
            println!("\n3. Fetching jobs for workflow '{}'...", workflow.name);
            let jobs = client.get_jobs(&workflow.id).await?;
            println!("   Found {} jobs", jobs.len());
            for j in &jobs {
                println!("   - {} ({}) - {}",
                    j.name, j.status, j.duration_formatted());
            }
        }
    }

    println!("\n✓ All tests passed!");
    Ok(())
}
```

Run with:
```bash
cargo run --example test_api
```

## Debugging

### Enable Debug Logging

Add to your `.env`:
```bash
RUST_LOG=debug
```

Then add logging to your code:
```rust
env_logger::init();
```

### Inspect API Responses

Use a tool like `curl` to test API endpoints directly:

```bash
# Test authentication
curl -H "Circle-Token: YOUR_TOKEN" \
  https://circleci.com/api/v2/me

# Test pipelines endpoint
curl -H "Circle-Token: YOUR_TOKEN" \
  "https://circleci.com/api/v2/project/gh/owner/repo/pipeline?page-token=&limit=10"

# Test workflows endpoint
curl -H "Circle-Token: YOUR_TOKEN" \
  https://circleci.com/api/v2/pipeline/PIPELINE_ID/workflow

# Test jobs endpoint
curl -H "Circle-Token: YOUR_TOKEN" \
  https://circleci.com/api/v2/workflow/WORKFLOW_ID/job
```

## Performance Testing

The API client includes these performance features:
- 30-second timeout per request
- Automatic pagination for large result sets
- Single request per screen load (no unnecessary calls)

Expected response times:
- Get pipelines: 200-500ms (for 50 items)
- Get workflows: 100-300ms (per pipeline)
- Get jobs: 100-300ms (per workflow)

## Next Steps

After verifying the API integration works:

1. Test navigation between screens
2. Verify workflow and job loading
3. Test filtering and search
4. Check error handling for edge cases
5. Implement background refresh (future enhancement)

## Comparison with Python Version

To verify compatibility, compare outputs:

**Rust version:**
```bash
cargo run
```

**Python version:**
```bash
cd /home/pedot/projects/circleci-tui
python -m cci.api
```

Both should show identical pipeline data from the CircleCI API.

## Security Notes

⚠️ **Important Security Reminders:**

1. **Never commit `.env` file** - It contains your API token
2. **Add `.env` to `.gitignore`** - Prevent accidental commits
3. **Use `.env.example`** - Provide template without secrets
4. **Rotate tokens regularly** - CircleCI allows token rotation
5. **Use read-only tokens** - If available, prefer read-only permissions

## API Rate Limits

CircleCI API v2 rate limits (as of 2024):
- **Free tier:** ~300 requests per minute
- **Paid tier:** ~3000 requests per minute

The TUI makes approximately:
- 1 request on startup (pipelines)
- 1 request per pipeline detail view (workflows)
- 1 request per workflow selection (jobs)

Normal usage should stay well within rate limits.
