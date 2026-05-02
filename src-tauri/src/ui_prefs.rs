use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::paths::AppPaths;

const FILE_NAME: &str = "ui_prefs.json";

/// Only built-in themes accepted until user override import exists.
pub fn normalize_theme_id(id: &str) -> String {
    let t = id.trim();
    if t == "light-paper" {
        t.to_string()
    } else {
        "light-paper".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPrefs {
    #[serde(default)]
    pub always_on_top: bool,
    #[serde(default = "default_theme")]
    pub theme_id: String,
}

fn default_theme() -> String {
    "light-paper".to_string()
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            always_on_top: false,
            theme_id: default_theme(),
        }
    }
}

pub fn load(paths: &AppPaths) -> AppResult<UiPrefs> {
    let p = paths.data_dir.join(FILE_NAME);
    if !p.exists() {
        return Ok(UiPrefs::default());
    }
    let s = std::fs::read_to_string(&p).map_err(AppError::Io)?;
    let mut v: UiPrefs = serde_json::from_str(&s).map_err(AppError::Json)?;
    v.theme_id = normalize_theme_id(&v.theme_id);
    Ok(v)
}

pub fn save(paths: &AppPaths, prefs: &UiPrefs) -> AppResult<()> {
    let p = paths.data_dir.join(FILE_NAME);
    let body = serde_json::to_string_pretty(prefs).map_err(AppError::Json)?;
    std::fs::write(&p, body).map_err(AppError::Io)?;
    Ok(())
}
