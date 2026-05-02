mod db;
mod error;
mod paths;

use rusqlite::Connection;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
