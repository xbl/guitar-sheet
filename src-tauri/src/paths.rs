use std::path::PathBuf;
use tauri::Manager;

use crate::error::{AppError, AppResult};

pub struct AppPaths {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub library_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &tauri::AppHandle) -> AppResult<Self> {
        let data_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|e| AppError::BadInput(e.to_string()))?;
        let library_dir = data_dir.join("library");
        let db_path = data_dir.join("index.sqlite3");
        Ok(Self {
            data_dir,
            db_path,
            library_dir,
        })
    }

    pub fn ensure_dirs(&self) -> AppResult<()> {
        std::fs::create_dir_all(&self.library_dir)?;
        Ok(())
    }
}
