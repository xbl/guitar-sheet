use tauri::State;

use crate::secrets;
use crate::settings::{self, GitHubSettings};
use crate::state::AppState;

#[tauri::command]
pub fn get_github_settings(state: State<'_, AppState>) -> Result<GitHubSettings, String> {
    settings::load(&state.paths).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_github_settings(state: State<'_, AppState>, value: GitHubSettings) -> Result<(), String> {
    settings::save(&state.paths, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_github_pat(token: String) -> Result<(), String> {
    secrets::set_pat(&token).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_github_pat() -> Result<(), String> {
    secrets::clear_pat().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn github_pat_configured() -> Result<bool, String> {
    secrets::pat_configured().map_err(|e| e.to_string())
}
