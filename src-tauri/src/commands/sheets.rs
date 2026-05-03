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
    pub folder_id: Option<String>,
    pub artist: Option<String>,
    pub tags: Vec<String>,
}

fn remote_path_for_local_rel(
    paths: &crate::paths::AppPaths,
    local_rel: &str,
) -> Result<Option<String>, String> {
    let gh = settings::load(paths).map_err(|e| e.to_string())?;
    let suffix = library_paths::strip_content_prefix(local_rel);
    Ok(Some(format!("{}{}", gh.normalized_prefix(), suffix)))
}

fn to_meta(paths: &crate::paths::AppPaths, row: SheetRow, tags: Vec<String>) -> SheetMeta {
    let absolute_path = paths.data_dir.join(&row.local_rel_path);
    SheetMeta {
        id: row.id,
        display_title: row.display_title,
        kind: row.kind,
        local_rel_path: row.local_rel_path.clone(),
        absolute_path: absolute_path.to_string_lossy().to_string(),
        local_content_hash: row.local_content_hash,
        remote_path: row.remote_path,
        folder_id: row.folder_id,
        artist: row.artist,
        tags,
    }
}

#[tauri::command]
pub fn import_sheet(
    state: State<'_, AppState>,
    source_path: String,
    maybe_title: Option<String>,
    folder_id: Option<String>,
) -> Result<SheetMeta, String> {
    let paths = &state.paths;
    let src = Path::new(&source_path);
    if !src.is_file() {
        return Err(AppError::BadInput("source is not a file".into()).to_string());
    }

    let segments: Vec<String> = {
        let conn = state
            .conn
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        match &folder_id {
            Some(fid) => {
                if db::get_folder(&conn, fid)
                    .map_err(|e| e.to_string())?
                    .is_none()
                {
                    return Err(format!("folder not found: {fid}"));
                }
                db::folder_path_segments(&conn, fid).map_err(|e| e.to_string())?
            }
            None => Vec::new(),
        }
    };

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

    let id = Uuid::new_v4().to_string();
    let id_nodash: String = id.chars().filter(|c| *c != '-').collect();
    let id_short: String = id_nodash.chars().take(12).collect();

    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("sheet");
    let display_title = maybe_title
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| stem.to_string());

    let stem_safe = crate::sanitize::storage_stem(&display_title);
    let ext_for_file = Path::new(&file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt");
    let storage_filename = format!("{stem_safe}_{id_short}.{ext_for_file}");

    let dest_dir = library_paths::folder_disk_path(&paths.library_dir, &segments);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest = dest_dir.join(&storage_filename);
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;

    let rel_under = if segments.is_empty() {
        storage_filename.clone()
    } else {
        format!("{}/{}", segments.join("/"), storage_filename)
    };
    let local_rel_path = library_paths::rel_path_content_file(&rel_under);
    let gh_suffix = library_paths::strip_content_prefix(&local_rel_path);

    let hash = hash::sha256_file(&dest).map_err(|e| e.to_string())?;
    let modified = chrono::Utc::now().to_rfc3339();

    let gh = settings::load(paths).map_err(|e| e.to_string())?;
    let prefix = gh.normalized_prefix();
    let remote_path = format!("{}{}", prefix, gh_suffix);

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
        folder_id,
        artist: None,
        reader_state_json: None,
    };

    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db::insert_sheet(&conn, &row).map_err(|e| e.to_string())?;
    Ok(to_meta(paths, row, vec![]))
}

/// Create a new empty (or initial-content) text tab on disk and index.
#[tauri::command]
pub fn create_text_sheet(
    state: State<'_, AppState>,
    title: String,
    folder_id: Option<String>,
    initial_content: Option<String>,
) -> Result<SheetMeta, String> {
    let display_title = title.trim().to_string();
    if display_title.is_empty() {
        return Err(AppError::BadInput("曲谱名称不能为空".into()).to_string());
    }

    let paths = &state.paths;
    let segments: Vec<String> = {
        let conn = state
            .conn
            .lock()
            .map_err(|_| "database lock poisoned".to_string())?;
        match &folder_id {
            Some(fid) => {
                if db::get_folder(&conn, fid)
                    .map_err(|e| e.to_string())?
                    .is_none()
                {
                    return Err(format!("folder not found: {fid}"));
                }
                db::folder_path_segments(&conn, fid).map_err(|e| e.to_string())?
            }
            None => Vec::new(),
        }
    };

    let id = Uuid::new_v4().to_string();
    let id_nodash: String = id.chars().filter(|c| *c != '-').collect();
    let id_short: String = id_nodash.chars().take(12).collect();

    let stem_safe = crate::sanitize::storage_stem(&display_title);
    let storage_filename = format!("{stem_safe}_{id_short}.txt");

    let dest_dir = library_paths::folder_disk_path(&paths.library_dir, &segments);
    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest = dest_dir.join(&storage_filename);

    let body = initial_content.unwrap_or_default();
    std::fs::write(&dest, body.as_bytes()).map_err(|e| e.to_string())?;

    let rel_under = if segments.is_empty() {
        storage_filename.clone()
    } else {
        format!("{}/{}", segments.join("/"), storage_filename)
    };
    let local_rel_path = library_paths::rel_path_content_file(&rel_under);

    let hash = hash::sha256_file(&dest).map_err(|e| e.to_string())?;
    let modified = chrono::Utc::now().to_rfc3339();

    let remote_path = remote_path_for_local_rel(paths, &local_rel_path)?;

    let row = SheetRow {
        id: id.clone(),
        display_title,
        kind: "text".to_string(),
        local_rel_path: local_rel_path.clone(),
        local_content_hash: hash,
        remote_path,
        remote_blob_sha: None,
        last_local_modified_at: modified,
        last_synced_at: None,
        folder_id,
        artist: None,
        reader_state_json: None,
    };

    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db::insert_sheet(&conn, &row).map_err(|e| e.to_string())?;
    Ok(to_meta(paths, row, vec![]))
}

#[tauri::command]
pub fn list_sheets(
    state: State<'_, AppState>,
    query: Option<String>,
    folder_id: Option<String>,
    tag_substring: Option<String>,
) -> Result<Vec<SheetMeta>, String> {
    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let rows = db::list_sheets_filtered(
        &conn,
        query.as_deref(),
        folder_id.as_deref(),
        tag_substring.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    let ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let tag_map = db::tags_for_sheet_ids(&conn, &ids).map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let tags = tag_map.get(&r.id).cloned().unwrap_or_default();
            to_meta(paths, r, tags)
        })
        .collect())
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
    let tags = db::tags_for_sheet_ids(&conn, &[id.clone()])
        .map_err(|e| e.to_string())?
        .remove(&id)
        .unwrap_or_default();
    Ok(to_meta(paths, row, tags))
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

    let abs = paths.data_dir.join(&row.local_rel_path);
    if abs.is_file() {
        let _ = std::fs::remove_file(&abs);
    } else if abs.is_dir() {
        let _ = std::fs::remove_dir_all(&abs);
    }

    db::delete_sheet(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_sheet(
    state: State<'_, AppState>,
    sheet_id: String,
    target_folder_id: Option<String>,
) -> Result<SheetMeta, String> {
    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;

    if let Some(ref fid) = target_folder_id {
        if db::get_folder(&conn, fid)
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Err(format!("folder not found: {fid}"));
        }
    }

    let mut row = db::get_sheet(&conn, &sheet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::BadInput(format!("no sheet with id {sheet_id}")).to_string())?;

    let same_folder = match (&row.folder_id, &target_folder_id) {
        (None, None) => true,
        (Some(a), Some(b)) => a == b,
        _ => false,
    };

    let segments: Vec<String> = match &target_folder_id {
        Some(fid) => db::folder_path_segments(&conn, fid).map_err(|e| e.to_string())?,
        None => Vec::new(),
    };

    let file_name = Path::new(&row.local_rel_path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::BadInput("invalid sheet path".into()).to_string())?;

    let rel_under = if segments.is_empty() {
        file_name.to_string()
    } else {
        format!("{}/{}", segments.join("/"), file_name)
    };
    let new_local_rel = library_paths::rel_path_content_file(&rel_under);

    if same_folder && new_local_rel == row.local_rel_path {
        let tags = db::tags_for_sheet_ids(&conn, &[sheet_id.clone()])
            .map_err(|e| e.to_string())?
            .remove(&sheet_id)
            .unwrap_or_default();
        return Ok(to_meta(paths, row, tags));
    }

    let old_abs = paths.data_dir.join(&row.local_rel_path);
    let new_abs = paths.data_dir.join(&new_local_rel);
    if new_abs != old_abs && new_abs.exists() {
        return Err("A file already exists at the destination.".into());
    }

    if let Some(parent) = new_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    if old_abs != new_abs {
        std::fs::rename(&old_abs, &new_abs).map_err(|e| e.to_string())?;
    }

    let new_remote = remote_path_for_local_rel(paths, &new_local_rel)?;

    let update_result = db::update_sheet_folder_paths(
        &conn,
        &sheet_id,
        target_folder_id.as_deref(),
        &new_local_rel,
        new_remote.as_deref(),
    );

    if let Err(e) = update_result {
        if old_abs != new_abs {
            let _ = std::fs::rename(&new_abs, &old_abs);
        }
        return Err(e.to_string());
    }

    row.folder_id = target_folder_id;
    row.local_rel_path = new_local_rel;
    row.remote_path = new_remote;

    let tags = db::tags_for_sheet_ids(&conn, &[sheet_id.clone()])
        .map_err(|e| e.to_string())?
        .remove(&sheet_id)
        .unwrap_or_default();
    Ok(to_meta(paths, row, tags))
}

#[tauri::command]
pub fn save_text_sheet(
    state: State<'_, AppState>,
    id: String,
    content: String,
) -> Result<SheetMeta, String> {
    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let row = db::get_sheet(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::BadInput(format!("no sheet with id {id}")).to_string())?;
    if row.kind != "text" {
        return Err("Only text sheets can be saved this way.".into());
    }
    let abs = paths.data_dir.join(&row.local_rel_path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&abs, content.as_bytes()).map_err(|e| e.to_string())?;
    let hash = hash::sha256_file(&abs).map_err(|e| e.to_string())?;
    db::update_sheet_local_hash_and_modified(&conn, &id, &hash).map_err(|e| e.to_string())?;
    let row = db::get_sheet(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "sheet missing after save".to_string())?;
    let tags = db::tags_for_sheet_ids(&conn, &[id.clone()])
        .map_err(|e| e.to_string())?
        .remove(&id)
        .unwrap_or_default();
    Ok(to_meta(paths, row, tags))
}

/// Saves a pasted image next to the text file; returns one line `{{IMG:filename}}` for the reader preview (same directory).
#[tauri::command]
pub fn save_sheet_clipboard_image(
    state: State<'_, AppState>,
    sheet_id: String,
    image_base64: String,
    extension: String,
) -> Result<String, String> {
    use base64::Engine;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| format!("无效的图片数据: {e}"))?;
    if bytes.is_empty() {
        return Err(AppError::BadInput("empty image".into()).to_string());
    }

    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let row = db::get_sheet(&conn, &sheet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::BadInput(format!("no sheet with id {sheet_id}")).to_string())?;
    if row.kind != "text" {
        return Err("仅文本曲谱支持剪切板贴图。".into());
    }

    let ext_raw = extension.to_lowercase();
    let ext_raw = ext_raw.trim_start_matches('.').trim();
    let ext_norm = match ext_raw {
        "png" => "png",
        "jpg" | "jpeg" => "jpg",
        "webp" => "webp",
        "gif" => "gif",
        other => {
            return Err(AppError::BadInput(format!("unsupported image type: {other}")).to_string());
        }
    };

    let tab_path = paths.data_dir.join(&row.local_rel_path);
    let dir = tab_path
        .parent()
        .ok_or_else(|| AppError::BadInput("invalid sheet path".into()).to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let id_short: String = Uuid::new_v4()
        .to_string()
        .chars()
        .filter(|c| *c != '-')
        .take(8)
        .collect();
    let filename = format!("paste_{id_short}.{ext_norm}");
    let dest = dir.join(&filename);
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;

    Ok(format!("{{{{IMG:{filename}}}}}"))
}

const MAX_READER_STATE_JSON: usize = 65536;

#[tauri::command]
pub fn get_sheet_reader_state(
    state: State<'_, AppState>,
    sheet_id: String,
) -> Result<Option<String>, String> {
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let row = db::get_sheet(&conn, &sheet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::BadInput(format!("no sheet with id {sheet_id}")).to_string())?;
    Ok(row.reader_state_json)
}

#[tauri::command]
pub fn set_sheet_reader_state(
    state: State<'_, AppState>,
    sheet_id: String,
    json: String,
) -> Result<(), String> {
    if json.len() > MAX_READER_STATE_JSON {
        return Err(AppError::BadInput("reader state JSON too large".into()).to_string());
    }
    serde_json::from_str::<serde_json::Value>(&json)
        .map_err(|e| AppError::BadInput(format!("invalid JSON: {e}")).to_string())?;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db::update_sheet_reader_state(&conn, &sheet_id, &json).map_err(|e| e.to_string())
}

/// Saves an image or PDF next to the text sheet; returns `{{IMG:…}}` or `{{PDF:…}}` line token.
#[tauri::command]
pub fn save_sheet_text_attachment(
    state: State<'_, AppState>,
    sheet_id: String,
    file_base64: String,
    extension: String,
) -> Result<String, String> {
    use base64::Engine;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(file_base64.trim())
        .map_err(|e| format!("无效的附件数据: {e}"))?;
    if bytes.is_empty() {
        return Err(AppError::BadInput("empty attachment".into()).to_string());
    }

    let ext_raw = extension.to_lowercase();
    let ext_norm = ext_raw.trim_start_matches('.').trim();

    let paths = &state.paths;
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    let row = db::get_sheet(&conn, &sheet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| AppError::BadInput(format!("no sheet with id {sheet_id}")).to_string())?;
    if row.kind != "text" {
        return Err("仅文本曲谱支持嵌入附件。".into());
    }

    let tab_path = paths.data_dir.join(&row.local_rel_path);
    let dir = tab_path
        .parent()
        .ok_or_else(|| AppError::BadInput("invalid sheet path".into()).to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let id_short: String = Uuid::new_v4()
        .to_string()
        .chars()
        .filter(|c| *c != '-')
        .take(8)
        .collect();

    match ext_norm {
        "png" | "jpg" | "jpeg" | "webp" | "gif" => {
            let ext_final = if ext_norm == "jpeg" { "jpg" } else { ext_norm };
            let filename = format!("paste_{id_short}.{ext_final}");
            let dest = dir.join(&filename);
            std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
            Ok(format!("{{{{IMG:{filename}}}}}"))
        }
        "pdf" => {
            if !bytes.starts_with(b"%PDF") {
                return Err(AppError::BadInput("not a PDF file".into()).to_string());
            }
            let filename = format!("drop_{id_short}.pdf");
            let dest = dir.join(&filename);
            std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
            Ok(format!("{{{{PDF:{filename}}}}}"))
        }
        other => Err(
            AppError::BadInput(format!("unsupported attachment type: {other}")).to_string(),
        ),
    }
}

#[cfg(test)]
mod embed_token_format_tests {
    #[test]
    fn img_return_line_is_double_braced() {
        let filename = "paste_abcdef12.png";
        let s = format!("{{{{IMG:{filename}}}}}");
        assert_eq!(s, "{{IMG:paste_abcdef12.png}}");
    }

    #[test]
    fn pdf_return_line_is_double_braced() {
        let filename = "drop_abcdef12.pdf";
        let s = format!("{{{{PDF:{filename}}}}}");
        assert_eq!(s, "{{PDF:drop_abcdef12.pdf}}");
    }
}
