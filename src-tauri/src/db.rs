use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};

/// Bump when `migrate` gains new steps; keep in sync with SQL in `migrate_to_v2`.
pub const SCHEMA_VERSION: i32 = 2;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sheets (
  id TEXT PRIMARY KEY,
  display_title TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('text','pdf','image')),
  local_rel_path TEXT NOT NULL,
  local_content_hash TEXT NOT NULL,
  remote_path TEXT,
  remote_blob_sha TEXT,
  last_local_modified_at TEXT NOT NULL,
  last_synced_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_sheets_display_title ON sheets(display_title);
"#;

pub fn init_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}

/// Applies incremental migrations after `init_schema`. Safe to call on every startup.
pub fn migrate(conn: &Connection, data_dir: &Path) -> AppResult<()> {
    let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if v >= SCHEMA_VERSION {
        return Ok(());
    }
    if v < 2 {
        migrate_to_v2(conn, data_dir)?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

fn migrate_to_v2(conn: &Connection, data_dir: &Path) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS folders (
          id TEXT PRIMARY KEY,
          parent_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
          name TEXT NOT NULL,
          created_at TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_parent_name
          ON folders (COALESCE(parent_id, ''), name);

        CREATE INDEX IF NOT EXISTS idx_folders_parent ON folders(parent_id);

        CREATE TABLE IF NOT EXISTS tags (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS sheet_tags (
          sheet_id TEXT NOT NULL REFERENCES sheets(id) ON DELETE CASCADE,
          tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
          PRIMARY KEY (sheet_id, tag_id)
        );
        "#,
    )?;

    add_column_if_missing(conn, "sheets", "folder_id", "TEXT")?;
    add_column_if_missing(conn, "sheets", "artist", "TEXT")?;

    migrate_legacy_library_paths(conn, data_dir)?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> AppResult<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
            [],
        )?;
    }
    Ok(())
}

/// Moves `library/<sheet_id>/<file>` → `library/content/<sheet_id>/<file>` and updates rows.
fn migrate_legacy_library_paths(conn: &Connection, data_dir: &Path) -> AppResult<()> {
    let rows: Vec<(String, String)> = {
        let mut stmt = conn.prepare("SELECT id, local_rel_path FROM sheets")?;
        let mapped = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        mapped.collect::<Result<Vec<_>, _>>()?
    };
    for (id, rel) in rows {
        if rel.starts_with("library/content/") {
            continue;
        }
        let parts: Vec<&str> = rel.split('/').collect();
        if parts.len() < 3 || parts[0] != "library" {
            continue;
        }
        let folder = parts[1];
        if folder == "content" {
            continue;
        }
        let file_suffix = parts[2..].join("/");
        if folder != id {
            continue;
        }
        let new_rel = format!("library/content/{id}/{file_suffix}");
        let old_abs = data_dir.join(&rel);
        let new_abs = data_dir.join(&new_rel);
        if old_abs.exists() {
            if let Some(parent) = new_abs.parent() {
                std::fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
            std::fs::rename(&old_abs, &new_abs).map_err(AppError::Io)?;
        } else if !new_abs.exists() {
            continue;
        }
        conn.execute(
            "UPDATE sheets SET local_rel_path = ?1 WHERE id = ?2",
            params![new_rel, id],
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SheetRow {
    pub id: String,
    pub display_title: String,
    pub kind: String,
    pub local_rel_path: String,
    pub local_content_hash: String,
    pub remote_path: Option<String>,
    pub remote_blob_sha: Option<String>,
    pub last_local_modified_at: String,
    pub last_synced_at: Option<String>,
    pub folder_id: Option<String>,
    pub artist: Option<String>,
}

fn row_from_stmt(row: &rusqlite::Row<'_>) -> rusqlite::Result<SheetRow> {
    Ok(SheetRow {
        id: row.get(0)?,
        display_title: row.get(1)?,
        kind: row.get(2)?,
        local_rel_path: row.get(3)?,
        local_content_hash: row.get(4)?,
        remote_path: row.get(5)?,
        remote_blob_sha: row.get(6)?,
        last_local_modified_at: row.get(7)?,
        last_synced_at: row.get(8)?,
        folder_id: row.get(9)?,
        artist: row.get(10)?,
    })
}

/// Use as `folder_id` filter value in [`list_sheets_filtered`] to list sheets not in any user folder (`folder_id IS NULL`).
pub const FOLDER_ROOT_SENTINEL: &str = "__root__";

fn query_sheet_rows(conn: &Connection, sql: &str, args: &[String]) -> AppResult<Vec<SheetRow>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = match args.len() {
        0 => stmt.query_map([], row_from_stmt)?,
        1 => stmt.query_map(params![args[0]], row_from_stmt)?,
        2 => stmt.query_map(params![args[0], args[1]], row_from_stmt)?,
        3 => stmt.query_map(params![args[0], args[1], args[2]], row_from_stmt)?,
        4 => stmt.query_map(params![args[0], args[1], args[2], args[3]], row_from_stmt)?,
        5 => stmt.query_map(
            params![args[0], args[1], args[2], args[3], args[4]],
            row_from_stmt,
        )?,
        6 => stmt.query_map(
            params![args[0], args[1], args[2], args[3], args[4], args[5]],
            row_from_stmt,
        )?,
        _ => {
            return Err(AppError::BadInput(format!(
                "unexpected bind count {}",
                args.len()
            )));
        }
    };
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Filter sheets by optional text query (title, path, artist, tag names), folder (`None` = any folder;
/// [`FOLDER_ROOT_SENTINEL`] = root only), and tag name substring.
pub fn list_sheets_filtered(
    conn: &Connection,
    query: Option<&str>,
    folder_id: Option<&str>,
    tag_substring: Option<&str>,
) -> AppResult<Vec<SheetRow>> {
    let mut buf = String::from(
        "SELECT DISTINCT s.id, s.display_title, s.kind, s.local_rel_path, s.local_content_hash, \
         s.remote_path, s.remote_blob_sha, s.last_local_modified_at, s.last_synced_at, s.folder_id, s.artist \
         FROM sheets s WHERE 1=1",
    );
    let mut args: Vec<String> = Vec::new();

    if let Some(q) = query {
        let l = format!("%{q}%");
        for _ in 0..4 {
            args.push(l.clone());
        }
        buf.push_str(
            " AND (s.display_title LIKE ? OR s.local_rel_path LIKE ? OR IFNULL(s.artist,'') LIKE ? \
             OR EXISTS (SELECT 1 FROM sheet_tags st INNER JOIN tags tg ON st.tag_id = tg.id \
             WHERE st.sheet_id = s.id AND tg.name LIKE ?))",
        );
    }
    if let Some(fid) = folder_id {
        if fid == FOLDER_ROOT_SENTINEL {
            buf.push_str(" AND s.folder_id IS NULL");
        } else {
            buf.push_str(" AND s.folder_id = ?");
            args.push(fid.to_string());
        }
    }
    if let Some(t) = tag_substring {
        let l = format!("%{t}%");
        args.push(l);
        buf.push_str(
            " AND EXISTS (SELECT 1 FROM sheet_tags st INNER JOIN tags tg ON st.tag_id = tg.id \
             WHERE st.sheet_id = s.id AND tg.name LIKE ?)",
        );
    }
    buf.push_str(" ORDER BY s.last_local_modified_at DESC");
    query_sheet_rows(conn, &buf, &args)
}

pub fn insert_sheet(conn: &Connection, row: &SheetRow) -> AppResult<()> {
    conn.execute(
        r#"INSERT INTO sheets (
            id, display_title, kind, local_rel_path, local_content_hash,
            remote_path, remote_blob_sha, last_local_modified_at, last_synced_at,
            folder_id, artist
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
        params![
            row.id,
            row.display_title,
            row.kind,
            row.local_rel_path,
            row.local_content_hash,
            row.remote_path,
            row.remote_blob_sha,
            row.last_local_modified_at,
            row.last_synced_at,
            row.folder_id,
            row.artist,
        ],
    )?;
    Ok(())
}

pub fn list_sheets(conn: &Connection, query: Option<&str>) -> AppResult<Vec<SheetRow>> {
    list_sheets_filtered(conn, query, None, None)
}

pub fn get_sheet(conn: &Connection, id: &str) -> AppResult<Option<SheetRow>> {
    let mut stmt = conn.prepare(
        r#"SELECT id, display_title, kind, local_rel_path, local_content_hash,
                  remote_path, remote_blob_sha, last_local_modified_at, last_synced_at,
                  folder_id, artist
           FROM sheets WHERE id = ?1"#,
    )?;
    let row = stmt.query_row(params![id], row_from_stmt).optional()?;
    Ok(row)
}

pub fn update_hashes_and_remote(
    conn: &Connection,
    id: &str,
    local_hash: &str,
    remote_sha: Option<&str>,
    synced_at_rfc3339: Option<&str>,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        r#"UPDATE sheets SET
            local_content_hash = ?2,
            remote_blob_sha = ?3,
            last_synced_at = ?4,
            last_local_modified_at = ?5
           WHERE id = ?1"#,
        params![id, local_hash, remote_sha, synced_at_rfc3339, now],
    )?;
    Ok(())
}

pub fn update_display_title(conn: &Connection, id: &str, title: &str) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE sheets SET display_title = ?2 WHERE id = ?1",
        params![id, title],
    )?;
    if n == 0 {
        return Err(AppError::BadInput(format!("no sheet with id {id}")));
    }
    Ok(())
}

/// After a local file write (e.g. text tab edit), update hash and modified time; leaves sync fields unchanged.
pub fn update_sheet_local_hash_and_modified(
    conn: &Connection,
    id: &str,
    local_hash: &str,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    let n = conn.execute(
        "UPDATE sheets SET local_content_hash = ?2, last_local_modified_at = ?3 WHERE id = ?1",
        params![id, local_hash, now],
    )?;
    if n == 0 {
        return Err(AppError::BadInput(format!("no sheet with id {id}")));
    }
    Ok(())
}

pub fn delete_sheet(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM sheets WHERE id = ?1", params![id])?;
    Ok(())
}

/// Updates folder assignment and stored paths for a sheet (e.g. after moving the file on disk).
pub fn update_sheet_folder_paths(
    conn: &Connection,
    id: &str,
    folder_id: Option<&str>,
    local_rel_path: &str,
    remote_path: Option<&str>,
) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE sheets SET folder_id = ?2, local_rel_path = ?3, remote_path = ?4 WHERE id = ?1",
        params![id, folder_id, local_rel_path, remote_path],
    )?;
    if n == 0 {
        return Err(AppError::BadInput(format!("no sheet with id {id}")));
    }
    Ok(())
}

/// Updates only on-disk path columns (same logical folder row); used when a parent folder is moved on disk.
pub fn update_sheet_storage_paths(
    conn: &Connection,
    id: &str,
    local_rel_path: &str,
    remote_path: Option<&str>,
) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE sheets SET local_rel_path = ?2, remote_path = ?3 WHERE id = ?1",
        params![id, local_rel_path, remote_path],
    )?;
    if n == 0 {
        return Err(AppError::BadInput(format!("no sheet with id {id}")));
    }
    Ok(())
}

/// Prefix match for `library/content/...` paths without LIKE meta-char issues (`%`, `_`).
fn row_matches_local_path_prefix(local_rel_path: &str, prefix: &str) -> bool {
    if local_rel_path == prefix {
        return true;
    }
    let pl = prefix.len();
    if local_rel_path.len() <= pl {
        return false;
    }
    local_rel_path.get(..pl) == Some(prefix)
        && local_rel_path.as_bytes().get(pl).copied() == Some(b'/')
}

/// All sheets whose stored path lies under the folder tree rooted at `path_prefix`
/// (`library/content/a/b` — files directly in that folder or in subfolders).
pub fn list_sheets_under_local_prefix(conn: &Connection, path_prefix: &str) -> AppResult<Vec<SheetRow>> {
    let mut stmt = conn.prepare(
        r#"SELECT id, display_title, kind, local_rel_path, local_content_hash,
                  remote_path, remote_blob_sha, last_local_modified_at, last_synced_at,
                  folder_id, artist
           FROM sheets"#,
    )?;
    let rows = stmt.query_map([], row_from_stmt)?;
    let mut out = Vec::new();
    for r in rows {
        let row = r?;
        if row_matches_local_path_prefix(&row.local_rel_path, path_prefix) {
            out.push(row);
        }
    }
    Ok(out)
}

pub fn folder_descendant_ids_including_self(
    conn: &Connection,
    root_id: &str,
) -> AppResult<Vec<String>> {
    let mut out = vec![root_id.to_string()];
    let mut queue = vec![root_id.to_string()];
    while let Some(fid) = queue.pop() {
        let mut stmt = conn.prepare("SELECT id FROM folders WHERE parent_id = ?1")?;
        let children: Vec<String> = stmt
            .query_map(params![fid], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for c in children {
            out.push(c.clone());
            queue.push(c);
        }
    }
    Ok(out)
}

/// Whether `folder_id` already exists under `parent_id` (root = `None`), excluding `except_id`.
pub fn folder_name_exists_under_parent(
    conn: &Connection,
    parent_id: Option<&str>,
    name: &str,
    except_id: &str,
) -> AppResult<bool> {
    let mut stmt = conn.prepare(
        "SELECT 1 FROM folders WHERE COALESCE(parent_id, '') = COALESCE(?1, '') \
         AND name = ?2 AND id != ?3 LIMIT 1",
    )?;
    let found: Option<i32> = stmt
        .query_row(params![parent_id, name, except_id], |r| r.get(0))
        .optional()?;
    Ok(found.is_some())
}

pub fn update_folder_parent(
    conn: &Connection,
    folder_id: &str,
    new_parent_id: Option<&str>,
) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE folders SET parent_id = ?2 WHERE id = ?1",
        params![folder_id, new_parent_id],
    )?;
    if n == 0 {
        return Err(AppError::BadInput(format!("no folder with id {folder_id}")));
    }
    Ok(())
}

pub fn update_folder_name(conn: &Connection, folder_id: &str, name: &str) -> AppResult<()> {
    let n = conn.execute(
        "UPDATE folders SET name = ?2 WHERE id = ?1",
        params![folder_id, name],
    )?;
    if n == 0 {
        return Err(AppError::BadInput(format!("no folder with id {folder_id}")));
    }
    Ok(())
}

pub fn count_child_folders(conn: &Connection, parent_id: &str) -> AppResult<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM folders WHERE parent_id = ?1",
        params![parent_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

pub fn count_sheets_in_folder(conn: &Connection, folder_id: &str) -> AppResult<i64> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sheets WHERE folder_id = ?1",
        params![folder_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

pub fn delete_folder_row(conn: &Connection, folder_id: &str) -> AppResult<()> {
    let n = conn.execute("DELETE FROM folders WHERE id = ?1", params![folder_id])?;
    if n == 0 {
        return Err(AppError::BadInput(format!("no folder with id {folder_id}")));
    }
    Ok(())
}

// --- Tags (schema v2) ---

pub fn normalize_tag_name(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn ensure_tag(conn: &Connection, name: &str) -> AppResult<String> {
    let mut stmt = conn.prepare("SELECT id FROM tags WHERE name = ?1")?;
    let found: Option<String> = stmt
        .query_row(params![name], |r| r.get(0))
        .optional()?;
    if let Some(id) = found {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO tags (id, name) VALUES (?1, ?2)",
        params![id, name],
    )?;
    Ok(id)
}

pub fn replace_sheet_tags(conn: &Connection, sheet_id: &str, raw_names: &[String]) -> AppResult<()> {
    conn.execute("DELETE FROM sheet_tags WHERE sheet_id = ?1", params![sheet_id])?;
    for raw in raw_names {
        let n = normalize_tag_name(raw);
        if n.is_empty() {
            continue;
        }
        let tid = ensure_tag(conn, &n)?;
        conn.execute(
            "INSERT INTO sheet_tags (sheet_id, tag_id) VALUES (?1, ?2)",
            params![sheet_id, tid],
        )?;
    }
    Ok(())
}

pub fn list_all_tag_names(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM tags ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn tags_for_sheet_ids(
    conn: &Connection,
    sheet_ids: &[String],
) -> AppResult<HashMap<String, Vec<String>>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for id in sheet_ids {
        let mut stmt = conn.prepare(
            "SELECT tg.name FROM sheet_tags st INNER JOIN tags tg ON st.tag_id = tg.id \
             WHERE st.sheet_id = ?1 ORDER BY tg.name",
        )?;
        let names: Vec<String> = stmt
            .query_map(params![id.as_str()], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        map.insert(id.clone(), names);
    }
    Ok(map)
}

// --- Folders (schema v2) ---

#[derive(Debug, Clone)]
pub struct FolderRow {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub created_at: String,
}

fn folder_row_from_stmt(row: &rusqlite::Row<'_>) -> rusqlite::Result<FolderRow> {
    Ok(FolderRow {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        name: row.get(2)?,
        created_at: row.get(3)?,
    })
}

pub fn insert_folder(conn: &Connection, row: &FolderRow) -> AppResult<()> {
    conn.execute(
        "INSERT INTO folders (id, parent_id, name, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![row.id, row.parent_id, row.name, row.created_at],
    )?;
    Ok(())
}

pub fn get_folder(conn: &Connection, id: &str) -> AppResult<Option<FolderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, created_at FROM folders WHERE id = ?1",
    )?;
    let row = stmt
        .query_row(params![id], folder_row_from_stmt)
        .optional()?;
    Ok(row)
}

/// All folder rows (for building a tree in memory).
pub fn list_all_folders(conn: &Connection) -> AppResult<Vec<FolderRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, name, created_at FROM folders ORDER BY name",
    )?;
    let rows = stmt.query_map([], folder_row_from_stmt)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Path segment names from library root down to `folder_id` (inclusive).
pub fn folder_path_segments(conn: &Connection, folder_id: &str) -> AppResult<Vec<String>> {
    let mut segments: Vec<String> = Vec::new();
    let mut current: Option<String> = Some(folder_id.to_string());
    while let Some(fid) = current {
        let row = get_folder(conn, &fid)?
            .ok_or_else(|| AppError::BadInput(format!("folder not found: {fid}")))?;
        segments.push(row.name);
        current = row.parent_id;
    }
    segments.reverse();
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_schema() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='sheets'")
            .unwrap();
        let exists: String = stmt.query_row([], |r| r.get(0)).unwrap();
        assert_eq!(exists, "sheets");
    }

    #[test]
    fn insert_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        migrate(&conn, dir.path()).unwrap();
        let row = SheetRow {
            id: "id1".into(),
            display_title: "Test".into(),
            kind: "text".into(),
            local_rel_path: "library/content/id1/content.txt".into(),
            local_content_hash: "abc".into(),
            remote_path: Some("sheets/id1.txt".into()),
            remote_blob_sha: None,
            last_local_modified_at: Utc::now().to_rfc3339(),
            last_synced_at: None,
            folder_id: None,
            artist: None,
        };
        insert_sheet(&conn, &row).unwrap();
        let all = list_sheets(&conn, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "id1");
        let found = list_sheets(&conn, Some("Te")).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn migration_v2_sets_user_version_and_creates_folders_table() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.sqlite3");
        let conn = Connection::open(&db_path).unwrap();
        init_schema(&conn).unwrap();
        migrate(&conn, dir.path()).unwrap();
        let v: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 2);
        let n: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='folders'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        migrate(&conn, dir.path()).unwrap();
        let v2: i32 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v2, 2);
    }

    #[test]
    fn folder_path_segments_walks_ancestors() {
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        migrate(&conn, dir.path()).unwrap();
        let root_id = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
        let child_id = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
        let now = Utc::now().to_rfc3339();
        insert_folder(
            &conn,
            &FolderRow {
                id: root_id.into(),
                parent_id: None,
                name: "Pop".into(),
                created_at: now.clone(),
            },
        )
        .unwrap();
        insert_folder(
            &conn,
            &FolderRow {
                id: child_id.into(),
                parent_id: Some(root_id.into()),
                name: "Songs".into(),
                created_at: now,
            },
        )
        .unwrap();
        let segs = folder_path_segments(&conn, child_id).unwrap();
        assert_eq!(segs, vec!["Pop".to_string(), "Songs".to_string()]);
    }

    #[test]
    fn list_sheets_filtered_matches_tag_substring() {
        let dir = tempfile::tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        migrate(&conn, dir.path()).unwrap();
        let row = SheetRow {
            id: "s1".into(),
            display_title: "A".into(),
            kind: "text".into(),
            local_rel_path: "library/content/x.txt".into(),
            local_content_hash: "h".into(),
            remote_path: None,
            remote_blob_sha: None,
            last_local_modified_at: Utc::now().to_rfc3339(),
            last_synced_at: None,
            folder_id: None,
            artist: None,
        };
        insert_sheet(&conn, &row).unwrap();
        replace_sheet_tags(&conn, "s1", &[String::from("Classic Rock")]).unwrap();
        let hit = list_sheets_filtered(&conn, None, None, Some("rock")).unwrap();
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].id, "s1");
        let miss = list_sheets_filtered(&conn, None, None, Some("jazz")).unwrap();
        assert!(miss.is_empty());
    }

    #[test]
    fn migration_moves_legacy_paths_under_content() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path();
        let id = "11111111-2222-3333-4444-555555555555";
        let rel = format!("library/{id}/content.txt");
        std::fs::create_dir_all(data.join("library").join(id)).unwrap();
        std::fs::write(data.join(&rel), b"e|0|0").unwrap();

        let db_path = data.join("index.sqlite3");
        let conn = Connection::open(&db_path).unwrap();
        init_schema(&conn).unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"INSERT INTO sheets (
                id, display_title, kind, local_rel_path, local_content_hash,
                remote_path, remote_blob_sha, last_local_modified_at, last_synced_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                id,
                "T",
                "text",
                &rel,
                "abc",
                None::<String>,
                None::<String>,
                now,
                None::<String>,
            ],
        )
        .unwrap();
        migrate(&conn, data).unwrap();

        let new_path = data.join("library/content").join(id).join("content.txt");
        assert!(new_path.is_file());
        let got = get_sheet(&conn, id).unwrap().expect("row");
        assert_eq!(
            got.local_rel_path,
            format!("library/content/{id}/content.txt")
        );
    }
}
