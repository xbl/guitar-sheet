use std::collections::HashMap;

use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use crate::db::{self, FolderRow};
use crate::library_paths;
use crate::sanitize;
use crate::settings;
use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FolderDto {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FolderNode {
    pub id: String,
    pub name: String,
    pub children: Vec<FolderNode>,
}

#[tauri::command]
pub fn create_folder(
    state: State<'_, AppState>,
    parent_id: Option<String>,
    name: String,
) -> Result<FolderDto, String> {
    let name = sanitize::sanitize_segment(&name).map_err(|e| e.to_string())?;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;

    if let Some(ref pid) = parent_id {
        if db::get_folder(&conn, pid)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err(format!("parent folder not found: {pid}"));
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let row = FolderRow {
        id: id.clone(),
        parent_id: parent_id.clone(),
        name: name.clone(),
        created_at: now,
    };

    db::insert_folder(&conn, &row).map_err(|e| {
        let s = e.to_string();
        if s.contains("UNIQUE") || s.contains("unique") {
            "A folder with that name already exists here.".to_string()
        } else {
            s
        }
    })?;

    let parent_segments = match &parent_id {
        Some(pid) => db::folder_path_segments(&conn, pid).map_err(|e| e.to_string())?,
        None => Vec::new(),
    };
    let mut disk_segments = parent_segments;
    disk_segments.push(name.clone());
    let disk_path = library_paths::folder_disk_path(&state.paths.library_dir, &disk_segments);
    std::fs::create_dir_all(&disk_path).map_err(|e| e.to_string())?;

    Ok(FolderDto {
        id,
        parent_id,
        name,
    })
}

#[tauri::command]
pub fn list_folder_tree(state: State<'_, AppState>) -> Result<Vec<FolderNode>, String> {
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let all = db::list_all_folders(&conn).map_err(|e| e.to_string())?;

    let mut by_parent: HashMap<Option<String>, Vec<FolderRow>> = HashMap::new();
    for row in all {
        by_parent.entry(row.parent_id.clone()).or_default().push(row);
    }

    fn build(
        by_parent: &mut HashMap<Option<String>, Vec<FolderRow>>,
        parent: Option<String>,
    ) -> Vec<FolderNode> {
        let mut rows = by_parent.remove(&parent).unwrap_or_default();
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        rows
            .into_iter()
            .map(|row| {
                let children = build(by_parent, Some(row.id.clone()));
                FolderNode {
                    id: row.id,
                    name: row.name,
                    children,
                }
            })
            .collect()
    }

    Ok(build(&mut by_parent, None))
}

fn remote_path_for_local_rel(
    paths: &crate::paths::AppPaths,
    local_rel: &str,
) -> Result<Option<String>, String> {
    let gh = settings::load(paths).map_err(|e| e.to_string())?;
    let suffix = library_paths::strip_content_prefix(local_rel);
    Ok(Some(format!("{}{}", gh.normalized_prefix(), suffix)))
}

#[tauri::command]
pub fn move_folder(
    state: State<'_, AppState>,
    folder_id: String,
    new_parent_id: Option<String>,
) -> Result<FolderDto, String> {
    let paths = &state.paths;
    let mut conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;

    let folder = db::get_folder(&conn, &folder_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "folder not found".to_string())?;

    if let Some(ref np) = new_parent_id {
        if np == &folder_id {
            return Err("Cannot move a folder into itself.".into());
        }
        if db::get_folder(&conn, np)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err(format!("parent folder not found: {np}"));
        }
    }

    let forbidden = db::folder_descendant_ids_including_self(&conn, &folder_id).map_err(|e| e.to_string())?;
    if let Some(ref np) = new_parent_id {
        if forbidden.contains(np) {
            return Err("Cannot move a folder into one of its subfolders.".into());
        }
    }

    if db::folder_name_exists_under_parent(
        &conn,
        new_parent_id.as_deref(),
        &folder.name,
        &folder_id,
    )
    .map_err(|e| e.to_string())?
    {
        return Err("A folder with that name already exists in the destination.".into());
    }

    let old_segments = db::folder_path_segments(&conn, &folder_id).map_err(|e| e.to_string())?;
    let new_parent_segments: Vec<String> = match &new_parent_id {
        Some(pid) => db::folder_path_segments(&conn, pid).map_err(|e| e.to_string())?,
        None => Vec::new(),
    };
    let mut new_segments = new_parent_segments;
    new_segments.push(folder.name.clone());

    if old_segments == new_segments {
        return Ok(FolderDto {
            id: folder_id,
            parent_id: new_parent_id,
            name: folder.name,
        });
    }

    let old_disk = library_paths::folder_disk_path(&paths.library_dir, &old_segments);
    let new_disk = library_paths::folder_disk_path(&paths.library_dir, &new_segments);

    if !old_disk.is_dir() {
        return Err("Folder is missing on disk.".into());
    }
    if new_disk.exists() {
        return Err("A folder or file already exists at the destination.".into());
    }

    if let Some(parent) = new_disk.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::rename(&old_disk, &new_disk).map_err(|e| e.to_string())?;

    let old_prefix = format!("library/content/{}", old_segments.join("/"));
    let new_prefix = format!("library/content/{}", new_segments.join("/"));

    let affected = db::list_sheets_under_local_prefix(&conn, &old_prefix).map_err(|e| e.to_string())?;

    let tx_result: Result<(), String> = (|| {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        db::update_folder_parent(&tx, &folder_id, new_parent_id.as_deref()).map_err(|e| {
            let s = e.to_string();
            if s.contains("UNIQUE") || s.contains("unique") {
                "A folder with that name already exists in the destination.".to_string()
            } else {
                s
            }
        })?;
        for sheet in &affected {
            if sheet.local_rel_path == old_prefix {
                return Err("internal: sheet path conflicts with folder path".into());
            }
            let rest = sheet
                .local_rel_path
                .strip_prefix(&old_prefix)
                .ok_or_else(|| "internal: sheet path prefix".to_string())?;
            let rest = rest.trim_start_matches('/');
            let new_local = format!("{}/{}", new_prefix, rest);
            let new_remote = remote_path_for_local_rel(paths, &new_local)?;
            db::update_sheet_storage_paths(&tx, &sheet.id, &new_local, new_remote.as_deref())
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    })();

    if let Err(e) = tx_result {
        let _ = std::fs::rename(&new_disk, &old_disk);
        return Err(e);
    }

    Ok(FolderDto {
        id: folder_id,
        parent_id: new_parent_id,
        name: folder.name,
    })
}
