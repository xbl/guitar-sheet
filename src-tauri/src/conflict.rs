//! Pure classification for one sheet row vs remote file metadata.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// Nothing to do (already aligned).
    Noop,
    /// Only update DB remote SHA (local bytes already match remote).
    MarkRemoteSha { remote_sha: String },
    /// Replace local file from remote bytes.
    Pull { remote_sha: String },
    /// Upload local file to GitHub (create or update).
    Push { create: bool },
    /// User must choose keep_local / take_remote / save_copy.
    Conflict {
        remote_sha: String,
        local_disk_hash: String,
        last_known_remote_sha: Option<String>,
    },
}

/// When `remote_sha` is `Some`, `remote_body_hash` must be the SHA-256 (hex) of the downloaded
/// GitHub file bytes so first-time pairing `(db_remote_sha == None)` can detect drift safely.
pub fn classify(
    disk_hash: &str,
    db_local_hash: &str,
    db_remote_sha: Option<&str>,
    remote_sha: Option<&str>,
    remote_body_hash: Option<&str>,
) -> Plan {
    match (db_remote_sha, remote_sha) {
        (None, None) => Plan::Push { create: true },
        (Some(_sr_old), None) => Plan::Push { create: true },
        (None, Some(rr)) => {
            let Some(rbh) = remote_body_hash else {
                return Plan::Conflict {
                    remote_sha: rr.to_string(),
                    local_disk_hash: disk_hash.to_string(),
                    last_known_remote_sha: None,
                };
            };
            if rbh == disk_hash {
                Plan::MarkRemoteSha {
                    remote_sha: rr.to_string(),
                }
            } else {
                Plan::Conflict {
                    remote_sha: rr.to_string(),
                    local_disk_hash: disk_hash.to_string(),
                    last_known_remote_sha: None,
                }
            }
        }
        (Some(sr), Some(rr)) => {
            if sr == rr {
                if disk_hash == db_local_hash {
                    Plan::Noop
                } else {
                    Plan::Push { create: false }
                }
            } else if disk_hash == db_local_hash {
                Plan::Pull {
                    remote_sha: rr.to_string(),
                }
            } else {
                Plan::Conflict {
                    remote_sha: rr.to_string(),
                    local_disk_hash: disk_hash.to_string(),
                    last_known_remote_sha: Some(sr.to_string()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_changed_local_clean_pull() {
        let p = classify("h1", "h1", Some("r1"), Some("r2"), None);
        assert_eq!(
            p,
            Plan::Pull {
                remote_sha: "r2".into()
            }
        );
    }

    #[test]
    fn remote_unchanged_local_dirty_push() {
        let p = classify("h2", "h1", Some("r1"), Some("r1"), None);
        assert_eq!(p, Plan::Push { create: false });
    }

    #[test]
    fn both_changed_conflict() {
        let p = classify("h2", "h1", Some("r1"), Some("r2"), None);
        assert_eq!(
            p,
            Plan::Conflict {
                remote_sha: "r2".into(),
                local_disk_hash: "h2".into(),
                last_known_remote_sha: Some("r1".into()),
            }
        );
    }

    #[test]
    fn first_remote_seen_matching_body_updates_sha_only() {
        let p = classify("ab", "ab", None, Some("rs"), Some("ab"));
        assert_eq!(p, Plan::MarkRemoteSha { remote_sha: "rs".into() });
    }

    #[test]
    fn first_remote_seen_mismatch_is_conflict() {
        let p = classify("ab", "ab", None, Some("rs"), Some("cd"));
        assert!(matches!(p, Plan::Conflict { .. }));
    }
}
