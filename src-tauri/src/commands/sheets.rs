use std::path::Path;

use tauri::State;
use uuid::Uuid;

use crate::db::{self, SheetRow};
use crate::error::AppError;
use crate::hash;
use crate::library_paths;
use crate::settings;
use crate::state::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SheetMeta {
    pub id: String,
    pub display_title: String,
    pub kind: String,
    pub local_rel_path: String,
    pub absolute_path: String,
    pub local_content_hash: String,
    pub remote_path: Option<String>,
}

fn to_meta(paths: &crate::paths::AppPaths, row: SheetRow) -> SheetMeta {
    let absolute_path = paths.data_dir.join(&row.local_rel_path);
    SheetMeta {
        id: row.id,
        display_title: row.display_title,
        kind: row.kind,
        local_rel_path: row.local_rel_path.clone(),
        absolute_path: absolute_path.to_string_lossy().to_string(),
        local_content_hash: row.local_content_hash,
        remote_path: row.remote_path,
    }
}

#[tauri::command]
pub fn import_sheet(
    state: State<'_, AppState>,
    source_path: String,
    maybe_title: Option<String>,
) -> Result<SheetMeta, String> {
    let paths = &state.paths;
    let src = Path::new(&source_path);
    if !src.is_file() {
        return Err(AppError::BadInput("source is not a file".into()).to_string());
    }

    let orig_ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    let (kind, file_name): (&'static str, String) = match orig_ext.as_str() {
        "" => ("text", "content.txt".into()),
        "txt" => ("text", "content.txt".into()),
        "md" => ("text", "content.md".into()),
        "pdf" => ("pdf", "content.pdf".into()),
        "png" => ("image", "content.png".into()),
        "jpg" | "jpeg" => ("image", "content.jpg".into()),
        "webp" => ("image", "content.webp".into()),
        other => {
            return Err(AppError::BadInput(format!("unsupported extension: .{other}")).to_string());
        }
    };

    let ext_for_remote = file_name
        .strip_prefix("content")
        .unwrap_or(&file_name)
        .to_string();

    let id = Uuid::new_v4().to_string();
    let dest_dir = library_paths::content_root(&paths.library_dir).join(&id);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest = dest_dir.join(&file_name);
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;

    let hash = hash::sha256_file(&dest).map_err(|e| e.to_string())?;
    let modified = chrono::Utc::now().to_rfc3339();

    let gh = settings::load(paths).map_err(|e| e.to_string())?;
    let prefix = gh.normalized_prefix();
    let remote_path = format!("{}{}{}", prefix, id, ext_for_remote);

    let local_rel_path = library_paths::rel_path_content_file(&format!("{id}/{file_name}"));

    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sheet");
    let display_title = maybe_title.unwrap_or_else(|| stem.to_string());

    let row = SheetRow {
        id: id.clone(),
        display_title,
        kind: kind.to_string(),
        local_rel_path: local_rel_path.clone(),
        local_content_hash: hash,
        remote_path: Some(remote_path),
        remote_blob_sha: None,
        last_local_modified_at: modified,
        last_synced_at: None,
    };

    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db::insert_sheet(&conn, &row).map_err(|e| e.to_string())?;
    Ok(to_meta(paths, row))
}

#[tauri::command]
pub fn list_sheets(state: State<'_, AppState>, query: Option<String>) -> Result<Vec<SheetMeta>, String> {
    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let rows = db::list_sheets(&conn, query.as_deref()).map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|r| to_meta(paths, r)).collect())
}

#[tauri::command]
pub fn get_sheet(state: State<'_, AppState>, id: String) -> Result<SheetMeta, String> {
    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let row = db::get_sheet(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::BadInput(format!("no sheet with id {id}")).to_string())?;
    Ok(to_meta(paths, row))
}

#[tauri::command]
pub fn rename_sheet_title(state: State<'_, AppState>, id: String, title: String) -> Result<(), String> {
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db::update_display_title(&conn, &id, &title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_sheet(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let row = db::get_sheet(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::BadInput(format!("no sheet with id {id}")).to_string())?;

    let parts: Vec<&str> = row.local_rel_path.split('/').collect();
    if parts.len() >= 4 && parts[0] == "library" && parts[1] == "content" {
        let dir = paths
            .data_dir
            .join("library")
            .join("content")
            .join(parts[2]);
        let _ = std::fs::remove_dir_all(&dir);
    } else if parts.len() >= 2 && parts[0] == "library" {
        let dir = paths.data_dir.join("library").join(parts[1]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    db::delete_sheet(&conn, &id).map_err(|e| e.to_string())
}
