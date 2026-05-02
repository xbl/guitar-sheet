use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{AppError, AppResult};

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
    })
}

pub fn insert_sheet(conn: &Connection, row: &SheetRow) -> AppResult<()> {
    conn.execute(
        r#"INSERT INTO sheets (
            id, display_title, kind, local_rel_path, local_content_hash,
            remote_path, remote_blob_sha, last_local_modified_at, last_synced_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
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
        ],
    )?;
    Ok(())
}

pub fn list_sheets(conn: &Connection, query: Option<&str>) -> AppResult<Vec<SheetRow>> {
    let mut out = Vec::new();
    if let Some(q) = query {
        let like = format!("%{q}%");
        let mut stmt = conn.prepare(
            r#"SELECT id, display_title, kind, local_rel_path, local_content_hash,
                      remote_path, remote_blob_sha, last_local_modified_at, last_synced_at
               FROM sheets
               WHERE display_title LIKE ?1 OR local_rel_path LIKE ?1
               ORDER BY last_local_modified_at DESC"#,
        )?;
        let rows = stmt.query_map(params![like], row_from_stmt)?;
        for r in rows {
            out.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(
            r#"SELECT id, display_title, kind, local_rel_path, local_content_hash,
                      remote_path, remote_blob_sha, last_local_modified_at, last_synced_at
               FROM sheets
               ORDER BY last_local_modified_at DESC"#,
        )?;
        let rows = stmt.query_map([], row_from_stmt)?;
        for r in rows {
            out.push(r?);
        }
    }
    Ok(out)
}

pub fn get_sheet(conn: &Connection, id: &str) -> AppResult<Option<SheetRow>> {
    let mut stmt = conn.prepare(
        r#"SELECT id, display_title, kind, local_rel_path, local_content_hash,
                  remote_path, remote_blob_sha, last_local_modified_at, last_synced_at
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

pub fn delete_sheet(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM sheets WHERE id = ?1", params![id])?;
    Ok(())
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
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let row = SheetRow {
            id: "id1".into(),
            display_title: "Test".into(),
            kind: "text".into(),
            local_rel_path: "library/id1/content.txt".into(),
            local_content_hash: "abc".into(),
            remote_path: Some("sheets/id1.txt".into()),
            remote_blob_sha: None,
            last_local_modified_at: Utc::now().to_rfc3339(),
            last_synced_at: None,
        };
        insert_sheet(&conn, &row).unwrap();
        let all = list_sheets(&conn, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "id1");
        let found = list_sheets(&conn, Some("Te")).unwrap();
        assert_eq!(found.len(), 1);
    }
}
