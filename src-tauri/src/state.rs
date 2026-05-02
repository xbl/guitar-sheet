use std::sync::Mutex;

use rusqlite::Connection;

use crate::paths::AppPaths;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub paths: AppPaths,
}
