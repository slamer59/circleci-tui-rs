//! String utility functions for UI rendering

/// Truncate a string to a maximum length, adding "..." if truncated.
///
/// This function safely handles edge cases where max_len < 3 by using
/// saturating_sub to prevent panics.
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_string_no_truncation() {
        assert_eq!(truncate_string("short", 10), "short");
    }

    #[test]
    fn test_truncate_string_with_truncation() {
        assert_eq!(
            truncate_string("this is a very long string", 10),
            "this is..."
        );
    }

    #[test]
    fn test_truncate_string_edge_cases() {
        assert_eq!(truncate_string("hello", 3), "...");
        assert_eq!(truncate_string("", 10), "");
    }
}
