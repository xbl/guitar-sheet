//! Paths under `library/content/` for nested folders (see library spec).

use std::path::{Path, PathBuf};

pub fn content_root(library_dir: &Path) -> PathBuf {
    library_dir.join("content")
}

/// Relative path under `data_dir` for a file stored under the content tree.
pub fn rel_path_content_file(rel_under_content: &str) -> String {
    format!("library/content/{}", rel_under_content.trim_start_matches('/'))
}

/// Absolute directory path under `library_dir/content/` for nested folders.
pub fn folder_disk_path(library_dir: &Path, segments: &[String]) -> PathBuf {
    let mut p = content_root(library_dir);
    for s in segments {
        p.push(s);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn content_root_appends_content() {
        let lib = Path::new("/data/library");
        assert_eq!(content_root(lib), PathBuf::from("/data/library/content"));
    }

    #[test]
    fn rel_path_content_file_strips_leading_slash() {
        assert_eq!(
            rel_path_content_file("/foo/bar.txt"),
            "library/content/foo/bar.txt"
        );
        assert_eq!(
            rel_path_content_file("foo/bar.txt"),
            "library/content/foo/bar.txt"
        );
    }

    #[test]
    fn folder_disk_path_joins_segments() {
        let lib = Path::new("/L");
        let segs = vec!["a".into(), "b".into()];
        assert_eq!(folder_disk_path(lib, &segs), PathBuf::from("/L/content/a/b"));
    }

    #[test]
    fn folder_disk_path_empty_segments_is_content_root() {
        let lib = Path::new("/L");
        let segs: Vec<String> = vec![];
        assert_eq!(folder_disk_path(lib, &segs), PathBuf::from("/L/content"));
    }
}
