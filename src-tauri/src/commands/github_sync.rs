use std::fs;
use std::path::Path;

use tauri::State;
use uuid::Uuid;

use crate::db::{self, SheetRow};
use crate::github::GitHubRest;
use crate::hash;
use crate::library_paths;
use crate::secrets;
use crate::settings;
use crate::state::AppState;
use crate::sync_github::SyncOutcome;

#[tauri::command]
pub fn sync_pull_push(state: State<'_, AppState>) -> Result<SyncOutcome, String> {
    crate::sync_github::run(&*state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_sheet_conflict(
    state: State<'_, AppState>,
    sheet_id: String,
    choice: String,
) -> Result<(), String> {
    let token = secrets::get_pat().map_err(|e| e.to_string())?;
    let token = token.ok_or_else(|| "GitHub token not set".to_string())?;
    let gh = settings::load(&state.paths).map_err(|e| e.to_string())?;
    if !gh.is_complete_for_api() {
        return Err("Configure owner and repo".into());
    }
    let client = GitHubRest::new(
        token,
        gh.owner.trim().to_string(),
        gh.repo.trim().to_string(),
        gh.branch.clone(),
    )
    .map_err(|e| e.to_string())?;

    let row = {
        let c = state
            .conn
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        db::get_sheet(&c, &sheet_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("no sheet {sheet_id}"))?
    };

    let rp = row
        .remote_path
        .as_deref()
        .ok_or_else(|| "sheet has no remote_path".to_string())?;
    let disk_path = state.paths.data_dir.join(&row.local_rel_path);
    let now = chrono::Utc::now().to_rfc3339();

    match choice.as_str() {
        "keep_local" => {
            let bytes = fs::read(&disk_path).map_err(|e| e.to_string())?;
            let cur = client.get_contents(rp).map_err(|e| e.to_string())?;
            let new_sha = match cur {
                Some(r) => client
                    .put_contents(
                        rp,
                        &format!("Resolve conflict (keep local): {rp}"),
                        &bytes,
                        Some(&r.sha),
                    )
                    .map_err(|e| e.to_string())?,
                None => client
                    .put_contents(
                        rp,
                        &format!("Resolve conflict (keep local, create): {rp}"),
                        &bytes,
                        None,
                    )
                    .map_err(|e| e.to_string())?,
            };
            let nh = hash::sha256_file(&disk_path).map_err(|e| e.to_string())?;
            let c = state
                .conn
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            db::update_hashes_and_remote(&c, &sheet_id, &nh, Some(&new_sha), Some(&now))
                .map_err(|e| e.to_string())?;
        }
        "take_remote" => {
            let remote = client
                .get_contents(rp)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "remote file not found".to_string())?;
            fs::write(&disk_path, &remote.bytes).map_err(|e| e.to_string())?;
            let nh = hash::sha256_file(&disk_path).map_err(|e| e.to_string())?;
            let c = state
                .conn
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            db::update_hashes_and_remote(&c, &sheet_id, &nh, Some(&remote.sha), Some(&now))
                .map_err(|e| e.to_string())?;
        }
        "save_copy" => {
            let bytes = fs::read(&disk_path).map_err(|e| e.to_string())?;
            let new_id = Uuid::new_v4().to_string();
            let dest_dir = library_paths::content_root(&state.paths.library_dir).join(&new_id);
            fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
            let fname = Path::new(&row.local_rel_path)
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| "bad local_rel_path".to_string())?;
            let dest = dest_dir.join(fname);
            fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
            let nh = hash::sha256_file(&dest).map_err(|e| e.to_string())?;
            let prefix = gh.normalized_prefix();
            let local_rel =
                library_paths::rel_path_content_file(&format!("{new_id}/{fname}"));
            let gh_suffix = library_paths::strip_content_prefix(&local_rel);
            let new_remote = format!("{}{}", prefix, gh_suffix);
            let new_row = SheetRow {
                id: new_id.clone(),
                display_title: format!("{} (copy)", row.display_title),
                kind: row.kind.clone(),
                local_rel_path: local_rel,
                local_content_hash: nh.clone(),
                remote_path: Some(new_remote.clone()),
                remote_blob_sha: None,
                last_local_modified_at: now.clone(),
                last_synced_at: None,
                folder_id: None,
                artist: None,
            };
            let c = state
                .conn
                .lock()
                .map_err(|_| "database lock poisoned".to_string())?;
            db::insert_sheet(&c, &new_row).map_err(|e| e.to_string())?;
            let new_sha = client
                .put_contents(
                    &new_remote,
                    &format!("Add copy from {}", row.id),
                    &bytes,
                    None,
                )
                .map_err(|e| e.to_string())?;
            db::update_hashes_and_remote(&c, &new_id, &nh, Some(&new_sha), Some(&now))
                .map_err(|e| e.to_string())?;
        }
        other => return Err(format!("unknown choice: {other}")),
    }

    Ok(())
}
