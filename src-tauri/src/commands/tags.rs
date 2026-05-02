use tauri::State;

use crate::db;
use crate::state::AppState;

#[tauri::command]
pub fn set_sheet_tags(
    state: State<'_, AppState>,
    sheet_id: String,
    tag_names: Vec<String>,
) -> Result<(), String> {
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    if db::get_sheet(&conn, &sheet_id)
        .map_err(|e| e.to_string())?
        .is_none()
    {
        return Err(format!("no sheet with id {sheet_id}"));
    }
    db::replace_sheet_tags(&conn, &sheet_id, &tag_names).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_all_tags(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let conn = state
        .conn
        .lock()
        .map_err(|_| "database lock poisoned".to_string())?;
    db::list_all_tag_names(&conn).map_err(|e| e.to_string())
}
