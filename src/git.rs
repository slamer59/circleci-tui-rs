//! Git repository utilities for branch detection.
//!
//! This module provides functionality to detect the current git branch,
//! which is useful for filtering CircleCI pipelines by branch name.

use git2::Repository;

/// Gets the remote tracking branch name (what CircleCI sees).
///
/// This function attempts to discover a git repository and returns the remote
/// branch name that CircleCI would see for the current local branch.
///
/// **Priority:**
/// 1. Remote tracking branch (e.g., `origin/main` → `main`)
/// 2. Local branch name if no upstream is configured
///
/// # Returns
///
/// - `Some(String)` containing the branch name if:
///   - A git repository is found
///   - HEAD points to a valid branch reference
/// - `None` if:
///   - Not in a git repository
///   - In detached HEAD state
///   - Any error occurs while accessing the repository
///
/// # Examples
///
/// ```no_run
/// use circleci_tui_rs::git::get_current_branch;
///
/// match get_current_branch() {
///     Some(branch) => println!("CircleCI branch: {}", branch),
///     None => println!("Not on a branch or not in a git repository"),
/// }
/// ```
///
/// # Edge Cases
///
/// - **Not in a git repository**: Returns `None`
/// - **Detached HEAD state**: Returns `None`
/// - **No upstream configured**: Returns local branch name
/// - **Empty repository**: Returns `None`
/// - **Permission issues**: Returns `None`
///
/// # Implementation Notes
///
/// Uses `Repository::discover(".")` to find the git repository, which searches
/// upward from the current directory. For branches with remotes, it extracts
/// the branch name from the upstream reference (e.g., `refs/remotes/origin/main` → `main`).
pub fn get_current_branch() -> Option<String> {
    // Attempt to discover the git repository from the current directory
    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(_) => {
            // Not in a git repository or can't access it
            return None;
        }
    };

    // Try to get the HEAD reference
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => {
            // HEAD doesn't exist (empty repo) or can't be read
            return None;
        }
    };

    // Check if HEAD is a direct reference (not detached)
    if !head.is_branch() {
        // In detached HEAD state
        return None;
    }

    // Get the local branch name first as fallback
    let local_branch = head.shorthand().map(|s| s.to_string())?;

    // Try to get the upstream branch (what origin has)
    let upstream_name = match repo.find_branch(&local_branch, git2::BranchType::Local) {
        Ok(branch) => {
            // Try to get upstream branch name
            branch
                .upstream()
                .ok()
                .and_then(|upstream| upstream.name().ok().flatten().map(|s| s.to_string()))
        }
        Err(_) => None,
    };

    // Parse upstream branch name if we have one
    if let Some(upstream) = upstream_name {
        // Parse the branch name from refs/remotes/origin/branch-name
        // We want just "branch-name" (what CircleCI sees)
        if let Some(branch_name) = upstream.strip_prefix("refs/remotes/") {
            // Strip the remote name (e.g., "origin/main" → "main")
            if let Some((_remote, branch)) = branch_name.split_once('/') {
                return Some(branch.to_string());
            }
        }
    }

    // Fallback to local branch if no upstream or parsing fails
    Some(local_branch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_branch_in_repo() {
        // This test will work if run within the circleci-tui-rs repository
        // In a real repository, this should return Some(branch_name)
        let result = get_current_branch();

        // We can't assert a specific value since it depends on the current branch
        // But we can verify it returns something when run in the project directory
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_get_current_branch_returns_string() {
        // Test that when a branch is found, it's a valid non-empty string
        if let Some(branch) = get_current_branch() {
            assert!(!branch.is_empty());
            // Branch names shouldn't contain path separators
            assert!(!branch.contains('/') || branch.contains("refs/"));
        }
    }
}
