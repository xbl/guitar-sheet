use std::collections::HashMap;

use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use crate::db::{self, FolderRow};
use crate::library_paths;
use crate::sanitize;
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
