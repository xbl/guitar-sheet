use crate::error::AppError;

/// Validates a single path segment for folder or display-relative naming (Task 3+).
#[allow(dead_code)] // wired in folder/create/move commands
pub fn sanitize_segment(name: &str) -> Result<String, AppError> {
    let t = name.trim();
    if t.is_empty() || t.contains('/') || t.contains('\\') || t == ".." {
        return Err(AppError::BadInput("invalid name".into()));
    }
    if t.chars().any(|c| c.is_control()) {
        return Err(AppError::BadInput("invalid characters".into()));
    }
    Ok(t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_segment_rejects_traversal() {
        assert!(sanitize_segment("../x").is_err());
        assert!(sanitize_segment("..").is_err());
    }

    #[test]
    fn sanitize_segment_trims_and_keeps_simple_name() {
        assert_eq!(sanitize_segment("  pop  ").unwrap(), "pop");
    }

    #[test]
    fn sanitize_segment_rejects_slash() {
        assert!(sanitize_segment("a/b").is_err());
    }

    #[test]
    fn sanitize_segment_rejects_control() {
        assert!(sanitize_segment("a\u{1}b").is_err());
    }
}
