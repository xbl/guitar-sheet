# Library folders, metadata & drag-and-drop — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Evolve the existing flat `library/{uuid}/` layout into a **nested folder tree on disk** under `library/content/`, with SQLite tables for `folders`, `tags`, `sheet_tags`, extended `sheets` (`folder_id`, `artist`), full-text style search over title/artist/tags, Tauri commands for import/move/create/edit flows, Finder drop support, and **GitHub `remote_path` rebuilt from folder segments + file name** per [谱库规格](../specs/2026-05-02-library-folders-metadata-design.md).

**Architecture:** Keep Vue → Tauri commands. Rust owns migrations (`PRAGMA user_version`), path sanitization, transactional **fs move + DB updates** for drag operations, and a single helper `remote_relative_path(sheet_id)` used by `import_sheet`, `create_text_sheet`, `move_sheet`, and post-move updates. Frontend: sidebar tree + “全部” list sharing one filter composable; text reader gains edit/save for `kind === 'text'`.

**Tech Stack:** Same as [MVP plan](./2026-05-02-guitar-sheet-mvp.md) (Tauri 2, Vue 3, rusqlite, sha2). Add **Vue draggable** or **HTML5 DnD API** (pick one in Task 10 — default to native DnD to avoid new deps); `@tauri-apps/plugin-fs` already available for app data reads if needed.

**Prerequisite:** MVP implementation through working `import_sheet` / `sync_pull_push` / `LibraryView` (current repo baseline). This plan **replaces** flat-only storage assumptions.

---

## Spec coverage (plan ↔ spec)

| Spec section | Addressed in |
|--------------|--------------|
| §3 磁盘目录 = 树 | Tasks 2, 4, 6, 7 |
| §4 `folders` / `tags` / `sheets` 扩展 | Tasks 1, 3, 5 |
| §5 `remote_path` | Tasks 4, 7 |
| §6.1 双入口 | Task 10 |
| §6.2 拖拽 | Tasks 6, 10 |
| §6.3 检索 | Task 5 |
| §6.4 新建文本谱 + 编辑 | Tasks 4, 9, 11 |
| §7 错误处理 | Assertions in Tasks 4, 6, 11 |

---

## File map (create / modify)

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/db.rs` | Schema v2 + migrations; `FolderRow`, `TagRow`; CRUD; `list_sheets_filtered`; transactions with fs helpers |
| `src-tauri/src/sanitize.rs` | **New:** segment sanitization for folder/file names (no `..`, control chars) |
| `src-tauri/src/library_paths.rs` | **New:** resolve `library/content` root, join folder chain → `PathBuf`, storage file naming |
| `src-tauri/src/commands/sheets.rs` | Extended import (folder_id, drop path), `create_text_sheet`, `move_sheet_to_folder`, `update_sheet_meta`, `save_text_sheet` |
| `src-tauri/src/commands/folders.rs` | **New:** `list_folder_tree`, `create_folder`, `rename_folder`, `move_folder`, `delete_folder` (empty only or cascade — match task) |
| `src-tauri/src/commands/tags.rs` | **New:** `set_sheet_tags`, `list_all_tags` |
| `src-tauri/src/commands/mod.rs` | Register new commands |
| `src-tauri/src/lib.rs` | Register modules |
| `src-tauri/src/sync_github.rs` | Use updated `remote_path` from DB; ensure move clears stale remote if spec requires (optional push deletes — document in Task 8) |
| `src/types/sheet.ts` | `folder_id`, `artist`, `tags: string[]` on meta |
| `src/types/folder.ts` | **New:** tree node type |
| `src/views/LibraryView.vue` | Sidebar tree, “全部”, search, DnD, Finder drop |
| `src/components/FolderTree.vue` | **New:** recursive tree + emits |
| `src/components/NewSheetDialog.vue` | **New:** title + optional artist/tags |
| `src/views/ReaderView.vue` | Text edit mode + save invoking `save_text_sheet` |
| `docs/superpowers/specs/2026-05-02-library-folders-metadata-design.md` | No edit unless spec drift found |

---

### Task 1: SQLite schema v2 + migration from flat `library/{uuid}/`

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs` (call `db::migrate` after `init_schema` / open connection)

**Constants:** Set `pub const SCHEMA_VERSION: i32 = 2;`

- [ ] **Step 1: Add migration runner**

After `init_schema`, open connection path from `AppState` already opens DB — add function `migrate(conn: &Connection) -> AppResult<()>` at bottom of `db.rs`:

```rust
pub fn migrate(conn: &Connection, data_dir: &std::path::Path) -> AppResult<()> {
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

fn migrate_to_v2(conn: &Connection, data_dir: &std::path::Path) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS folders (
          id TEXT PRIMARY KEY,
          parent_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
          name TEXT NOT NULL,
          created_at TEXT NOT NULL,
          UNIQUE(parent_id, name)
        );
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

        ALTER TABLE sheets ADD COLUMN folder_id TEXT REFERENCES folders(id);
        ALTER TABLE sheets ADD COLUMN artist TEXT;
        "#,
    )?;
    Ok(())
}
```

Note: SQLite `ALTER TABLE … ADD` may error if column exists on re-run — guard with `pragma table_info` in production migration or rely on `user_version` only running once.

- [ ] **Step 2: One-shot filesystem migration for existing sheets**

Still inside `migrate_to_v2`, after SQL, select all `sheets` and for each row where `local_rel_path` matches old pattern `library/<uuid>/…`:

```rust
// Pseudocode in plan — implement fully in code:
// let content_root = paths.library_dir.join("content");
// fs::create_dir_all(&content_root)?;
// for each row: if path is library/{uuid}/file -> fs::rename(data_dir.join(old), data_dir.join(new))
// new local_rel_path = library/content/{uuid}/{filename}
```

Wire `migrate_to_v2` to receive `data_dir: &Path` from caller: change signature to `migrate(conn: &Connection, data_dir: &Path) -> AppResult<()>` and call from `lib.rs` / state init with `paths.data_dir`.

Implement rename loop with `std::fs::rename`; on failure return `AppError::Io` and **do not bump user_version** (wrap migration in one transaction for DB only; fs is best-effort documented — if rename fails, abort and log).

- [ ] **Step 3: Unit test in-memory DB migration**

In `db.rs` `#[cfg(test)]`, test `migrate_to_v2` on empty `sheets` creates tables; test insert folder + sheet with `folder_id`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat(db): schema v2 folders/tags + migrate hook"
```

---

### Task 2: `sanitize` + `library_paths` helpers

**Files:**
- Create: `src-tauri/src/sanitize.rs`
- Create: `src-tauri/src/library_paths.rs`
- Modify: `src-tauri/src/lib.rs` (`mod sanitize; mod library_paths;`)

- [ ] **Step 1: `sanitize.rs`**

```rust
pub fn sanitize_segment(name: &str) -> Result<String, crate::error::AppError> {
    let t = name.trim();
    if t.is_empty() || t.contains('/') || t.contains('\\') || t == ".." {
        return Err(crate::error::AppError::BadInput("invalid name".into()));
    }
    if t.chars().any(|c| c.is_control()) {
        return Err(crate::error::AppError::BadInput("invalid characters".into()));
    }
    Ok(t.to_string())
}
```

Add `#[cfg(test)]` with cases: `"../x"` err, `"  pop  "` ok → `"pop"`.

- [ ] **Step 2: `library_paths.rs`**

```rust
use std::path::{Path, PathBuf};

pub fn content_root(library_dir: &Path) -> PathBuf {
    library_dir.join("content")
}

/// Relative path under data_dir for a sheet file at root of content (folder_id None).
pub fn rel_path_content_file(rel_under_content: &str) -> String {
    format!("library/content/{}", rel_under_content.trim_start_matches('/'))
}
```

Add function `folder_disk_path(library_dir: &Path, segments: &[String]) -> PathBuf` joining `content_root` + segments.

- [ ] **Step 3: `cargo test` in `src-tauri`**

Run: `cd src-tauri && cargo test sanitize`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/sanitize.rs src-tauri/src/library_paths.rs src-tauri/src/lib.rs
git commit -m "feat: sanitize path segments and library/content helpers"
```

---

### Task 3: Folder CRUD commands + list tree

**Files:**
- Create: `src-tauri/src/commands/folders.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/db.rs` (queries: `insert_folder`, `list_child_folders`, `get_folder_path_segments`)

- [ ] **Step 1: DB helpers for folder chain**

Implement `pub fn folder_path_segments(conn: &Connection, folder_id: &str) -> AppResult<Vec<String>>` walking `parent_id` until NULL, returning segments **root → leaf** (folder `name` values).

- [ ] **Step 2: Commands**

`#[tauri::command] pub fn create_folder(parent_id: Option<String>, name: String) -> Result<FolderDto, String>`  
- Sanitize `name`, insert row, `fs::create_dir_all` under `content_root` + segments.

`list_folder_tree` returns nested JSON: `{ id, name, children: [...] }`.

- [ ] **Step 3: Register in `commands/mod.rs` and `lib.rs` invoke_handler**

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/commands/folders.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: folder CRUD and tree listing"
```

---

### Task 4: Refactor `import_sheet` — target folder + `library/content/…` layout + `remote_path`

**Files:**
- Modify: `src-tauri/src/commands/sheets.rs`
- Modify: `src-tauri/src/db.rs` if insert signature changes

**Behavior:**
- New signature: `import_sheet(source_path, maybe_title, folder_id: Option<String>)`.
- Storage path: `library/content/{relative}` where `relative` = if `folder_id` Some → join `folder_path_segments` + unique filename from stem/uuid; if None → `library/content/{uuid_stem}.ext` rule (pick: `{sanitize(title)}_{uuid8}.txt` to avoid collisions — implement `unique_filename(conn, folder_id, base)`).
- `remote_path`: `normalized_prefix() + "/" + same relative as GitHub path` (use `/` separator — GitHub API expects forward slashes).
- Remove old `library/{uuid}/` layout for **new** imports (migrated rows already under `content/` from Task 1).

- [ ] **Step 1: Implement `compute_remote_path`** in `library_paths.rs` or `settings` helper

- [ ] **Step 2: Replace body of `import_sheet`** following spec §3–§5

- [ ] **Step 3: Manual test** — import into nested folder from dialog selecting folder id `None` first

- [ ] **Step 4: Commit**

```bash
git commit -am "feat(import): folder target and content-relative paths"
```

---

### Task 5: Tags + search (`list_sheets` extended)

**Files:**
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/commands/sheets.rs`

- Implement `list_sheets(query, folder_id_filter: Option<String>, tag_filter: Option<String>)` with SQL `LEFT JOIN` / `EXISTS` for tag substring.

- [ ] **Commit:** `feat: sheet list filters for folder and tags`

---

### Task 6: `move_sheet` + `move_folder` (fs + DB transaction)

**Files:**
- Modify: `src-tauri/src/commands/folders.rs` / `sheets.rs`
- Modify: `src-tauri/src/db.rs`

**Rules:** `std::fs::rename` for file move; folder move = `rename` directory subtree; update all affected `sheets.local_rel_path` and `folder_id` / `folders.parent_id` in one SQLite transaction after fs succeeds.

- [ ] **Commit:** `feat: move sheet and folder with filesystem mirror`

---

### Task 7: Align `sync_github` with new `remote_path`

**Files:**
- Modify: `src-tauri/src/sync_github.rs`
- Modify: `src-tauri/src/commands/sheets.rs` (ensure every mutation updates `remote_path` when prefix rules change)

After moves, old remote blob may remain — document in **README** (troubleshooting): on next successful push of new path, optional delete of old path via GitHub API (YAGNI: orphan remotes may linger; **must** update `remote_path` column so sync pushes to correct path).

- [ ] **Commit:** `fix(sync): use folder-based remote_path from DB`

---

### Task 8: Document GitHub orphan paths (spec debt)

**Files:**
- Modify: `README.md` — add optional short “Sync / moved files” note (orphan blobs on GitHub may remain until manual delete).

Skip if you accept orphan remotes for v1.

---

### Task 9: `create_text_sheet` + `save_text_sheet`

**Files:**
- Modify: `src-tauri/src/commands/sheets.rs`
- Modify: `src-tauri/src/db.rs`

Commands:

```rust
#[tauri::command]
pub fn create_text_sheet(
    folder_id: Option<String>,
    display_title: String,
    maybe_artist: Option<String>,
    extension: String, // "txt" | "md"
) -> Result<SheetMeta, String>;

#[tauri::command]
pub fn save_text_sheet(id: String, body_utf8: String) -> Result<SheetMeta, String>;
```

`create_text_sheet`: validate non-empty trimmed title; write empty file UTF-8; hash; insert row `kind=text`.

`save_text_sheet`: write file at `data_dir.join(local_rel_path)`, recompute hash, `UPDATE sheets`.

- [ ] **Commit:** `feat: create and save text sheets`

---

### Task 10: Vue — `FolderTree`, Library layout, DnD, Finder drop

**Files:**
- Create: `src/components/FolderTree.vue`
- Modify: `src/views/LibraryView.vue`
- Modify: `package.json` only if adding a DnD dependency

**UI:**
- Left: `FolderTree` + top item「全部」sets `selectedFolderId = null` and list uses flat `list_sheets` with no folder filter.
- Main: toolbar「导入」「新建曲谱」; search input bound to `query`.
- HTML5 DnD: `draggable` on row; `@drop` on folder nodes calling `move_sheet` / `move_folder` with target id.
- Finder drop: `@dragover.prevent` + `@drop.prevent` on drop zone — read `event.dataTransfer.files[0].path` is **not available in web**; use Tauri **drag-drop** or `dialog` only on macOS — **use `@tauri-apps/api` webview drag** — check Tauri 2 file drop plugin: if unavailable, **fallback** document picker with “current folder” — mark **Step** as: enable `tauri-plugin-fs` file drop on window from [Tauri docs](https://v2.tauri.app/plugin/file-drop/) or use `on_drop` event in Rust to get paths and forward to frontend via event — implement **Rust-side `on_file_drop` listener** passing paths to Vue for `import_sheet` with `folder_id`.

Concrete approach for Step 10c:

1. In `lib.rs` `.on_window_event` or plugin: when files dropped, emit `files-dropped` with paths array + optional target folder from focused UI (store `pending_drop_folder_id` in frontend state set on `dragenter` folder row).

If too heavy: **MVP subset** — Finder drop **only** onto app window (imports to root); folder-targeted import **only** via button — document in commit message; user may iterate.

- [ ] **Commit:** `feat(ui): folder sidebar, all-sheets view, drag-move`

---

### Task 11: Reader text edit mode

**Files:**
- Modify: `src/views/ReaderView.vue`

- When `kind === 'text'`, show toggle「编辑」; `<textarea>` or `contenteditable` with monospace CSS; Save button calls `save_text_sheet`; disable while saving.

- [ ] **Commit:** `feat(reader): edit and save text tabs`

---

### Task 12: TypeScript types + `invoke` wiring audit

**Files:**
- Modify: `src/types/sheet.ts`
- Run: `npm run build` at repo root

Expected: typecheck passes.

- [ ] **Commit:** `chore: sync TS types for library metadata`

---

## Plan self-review

| Check | Result |
|-------|--------|
| Spec §6.4 应用内编辑 | Task 11 |
| §6.2 访达拖入 | Task 10 (Rust drop or deferred subset) |
| No `TBD` in steps | Orphan remote cleanup deferred explicitly |
| Type names consistent | `SheetMeta` extended in Task 12 |

**Gap:** Finder → **specific folder** drop may require Rust event + focused folder id — if shipped without it, update spec §6.2 with “第一期仅拖入窗口根目录” or implement full drop in same release.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-02-library-folders-metadata.md`.

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks.

**2. Inline Execution** — run tasks in this session using executing-plans with checkpoints.

Which approach do you want?
