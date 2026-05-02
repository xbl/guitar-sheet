mod commands;
mod conflict;
mod db;
mod error;
mod github;
mod hash;
mod paths;
mod secrets;
mod settings;
mod state;
mod sync_github;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let paths = paths::AppPaths::resolve(app.handle())
                .and_then(|p| {
                    p.ensure_dirs()?;
                    Ok(p)
                })
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
            let conn = Connection::open(&paths.db_path)
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
            db::init_schema(&conn)
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
            db::migrate(&conn, &paths.data_dir)
                .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
            app.manage(state::AppState {
                conn: Mutex::new(conn),
                paths,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sheets::import_sheet,
            commands::sheets::list_sheets,
            commands::sheets::get_sheet,
            commands::sheets::rename_sheet_title,
            commands::sheets::delete_sheet,
            commands::github_settings::get_github_settings,
            commands::github_settings::set_github_settings,
            commands::github_settings::set_github_pat,
            commands::github_settings::clear_github_pat,
            commands::github_settings::github_pat_configured,
            commands::github_settings::test_github_connection,
            commands::github_sync::sync_pull_push,
            commands::github_sync::resolve_sheet_conflict,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
