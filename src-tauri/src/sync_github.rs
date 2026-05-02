//! GitHub REST sync: pull/push with conflict detection (spec §6).

use std::fs;

use chrono::Utc;
use serde::Serialize;

use crate::conflict::Plan;
use crate::db::{self, SheetRow};
use crate::error::{AppError, AppResult};
use crate::github::GitHubRest;
use crate::hash;
use crate::secrets;
use crate::settings;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ConflictEntry {
    pub sheet_id: String,
    pub display_title: String,
    pub remote_path: String,
    pub remote_sha: String,
    pub local_disk_hash: String,
    pub last_known_remote_sha: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SyncOutcome {
    pub ok: bool,
    pub conflicts: Vec<ConflictEntry>,
    pub pulled: u32,
    pub pushed: u32,
}

struct Prepared {
    row: SheetRow,
    disk_hash: String,
    remote: Option<crate::github::RemoteFile>,
    remote_body_hash: Option<String>,
}

pub fn run(state: &AppState) -> AppResult<SyncOutcome> {
    let token = secrets::get_pat()?.ok_or_else(|| AppError::BadInput("GitHub token not set".into()))?;
    let gh = settings::load(&state.paths)?;
    if !gh.is_complete_for_api() {
        return Err(AppError::BadInput(
            "Set GitHub owner and repo in settings.".into(),
        ));
    }
    let client = GitHubRest::new(
        token,
        gh.owner.trim().to_string(),
        gh.repo.trim().to_string(),
        gh.branch.clone(),
    )?;

    let rows: Vec<SheetRow> = {
        let c = state
            .conn
            .lock()
            .map_err(|_| AppError::BadInput("database lock poisoned".into()))?;
        db::list_sheets(&c, None)?
    };

    let mut prepared: Vec<Prepared> = Vec::new();
    for row in rows {
        let rp = match row.remote_path.as_ref() {
            Some(p) if !p.is_empty() => p.clone(),
            _ => continue,
        };
        let disk_path = state.paths.data_dir.join(&row.local_rel_path);
        let disk_hash = hash::sha256_file(&disk_path)?;
        let remote = client.get_contents(&rp)?;
        let remote_body_hash = remote.as_ref().map(|r| hash::sha256_bytes(&r.bytes));
        prepared.push(Prepared {
            row,
            disk_hash,
            remote,
            remote_body_hash,
        });
    }

    let mut conflicts: Vec<ConflictEntry> = Vec::new();
    for p in &prepared {
        let rp = p.row.remote_path.as_deref().unwrap();
        let plan = crate::conflict::classify(
            &p.disk_hash,
            &p.row.local_content_hash,
            p.row.remote_blob_sha.as_deref(),
            p.remote.as_ref().map(|r| r.sha.as_str()),
            p.remote_body_hash.as_deref(),
        );
        if let Plan::Conflict {
            remote_sha,
            local_disk_hash,
            last_known_remote_sha,
        } = plan
        {
            conflicts.push(ConflictEntry {
                sheet_id: p.row.id.clone(),
                display_title: p.row.display_title.clone(),
                remote_path: rp.to_string(),
                remote_sha,
                local_disk_hash,
                last_known_remote_sha,
            });
        }
    }

    if !conflicts.is_empty() {
        return Ok(SyncOutcome {
            ok: false,
            conflicts,
            pulled: 0,
            pushed: 0,
        });
    }

    let now = Utc::now().to_rfc3339();
    let mut pulled: u32 = 0;
    let mut pushed: u32 = 0;
    let conn = state
        .conn
        .lock()
        .map_err(|_| AppError::BadInput("database lock poisoned".into()))?;

    for p in prepared {
        let rp = p.row.remote_path.as_deref().unwrap();
        let plan = crate::conflict::classify(
            &p.disk_hash,
            &p.row.local_content_hash,
            p.row.remote_blob_sha.as_deref(),
            p.remote.as_ref().map(|r| r.sha.as_str()),
            p.remote_body_hash.as_deref(),
        );
        let disk_path = state.paths.data_dir.join(&p.row.local_rel_path);
        match plan {
            Plan::Noop => {}
            Plan::MarkRemoteSha { remote_sha } => {
                db::update_hashes_and_remote(
                    &conn,
                    &p.row.id,
                    &p.disk_hash,
                    Some(&remote_sha),
                    Some(&now),
                )?;
            }
            Plan::Pull { remote_sha } => {
                let bytes = p
                    .remote
                    .as_ref()
                    .map(|r| r.bytes.as_slice())
                    .ok_or_else(|| AppError::BadInput("pull missing remote body".into()))?;
                fs::write(&disk_path, bytes)?;
                let nh = hash::sha256_file(&disk_path)?;
                db::update_hashes_and_remote(&conn, &p.row.id, &nh, Some(&remote_sha), Some(&now))?;
                pulled += 1;
            }
            Plan::Push { create } => {
                let bytes = fs::read(&disk_path)?;
                let prev_sha = if create {
                    None
                } else {
                    client.get_contents(rp)?.map(|r| r.sha)
                };
                let msg = if create {
                    format!("Add {rp}")
                } else {
                    format!("Update {rp}")
                };
                let new_sha = client.put_contents(rp, &msg, &bytes, prev_sha.as_deref())?;
                let nh = hash::sha256_file(&disk_path)?;
                db::update_hashes_and_remote(&conn, &p.row.id, &nh, Some(&new_sha), Some(&now))?;
                pushed += 1;
            }
            Plan::Conflict { .. } => unreachable!("conflicts were preflighted"),
        }
    }

    Ok(SyncOutcome {
        ok: true,
        conflicts: vec![],
        pulled,
        pushed,
    })
}
