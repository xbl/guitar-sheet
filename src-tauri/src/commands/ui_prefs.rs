use serde::Deserialize;
use tauri::State;

use crate::state::AppState;
use crate::ui_prefs::{self, UiPrefs};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPrefsPatch {
    pub always_on_top: Option<bool>,
    pub theme_id: Option<String>,
}

#[tauri::command]
pub fn get_ui_prefs(state: State<'_, AppState>) -> Result<UiPrefs, String> {
    ui_prefs::load(&state.paths).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_ui_prefs(state: State<'_, AppState>, patch: UiPrefsPatch) -> Result<UiPrefs, String> {
    let mut prefs = ui_prefs::load(&state.paths).map_err(|e| e.to_string())?;
    if let Some(v) = patch.always_on_top {
        prefs.always_on_top = v;
    }
    if let Some(v) = patch.theme_id {
        prefs.theme_id = ui_prefs::normalize_theme_id(&v);
    }
    ui_prefs::save(&state.paths, &prefs).map_err(|e| e.to_string())?;
    Ok(prefs)
}
