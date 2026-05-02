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

#[tauri::command]
pub fn test_github_connection() -> Result<String, String> {
    let token = secrets::get_pat()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "GitHub token not set".to_string())?;
    let client = reqwest::blocking::Client::builder()
        .user_agent("guitar-sheet/0.1 (Tauri)")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get("https://api.github.com/user")
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("{status}: {body}"));
    }
    let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    let login = v["login"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    Ok(login)
}
