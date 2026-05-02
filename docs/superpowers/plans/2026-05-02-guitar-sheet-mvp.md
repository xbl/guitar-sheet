# Guitar Sheet MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a macOS Tauri + Vue app where beginners import text/PDF/image sheets, browse them locally, optionally sync each file to a user-owned GitHub repo via REST with explicit conflict resolution (no silent overwrite).

**Architecture:** Vue 3 UI calls Tauri commands. Rust owns SQLite index, hashed content for conflict detection, filesystem copies under the app data dir, and a small GitHub REST client. PAT lives in the OS keychain (`keyring` crate). PDF/image/text rendering stays in the WebView (Vue + `pdfjs-dist` for PDF).

**Tech Stack:** Tauri 2, Vue 3, TypeScript, Vite, `rusqlite` (bundled SQLite), `sha2`, `reqwest` (rustls), `serde`/`serde_json`, `keyring`, `uuid`, `chrono`, `thiserror`; Vue side `vue-router`, `pdfjs-dist` (worker build per Vite docs).

---

## File map (create / modify)

| Path | Responsibility |
|------|----------------|
| `src-tauri/Cargo.toml` | Add crates + features (`rusqlite/bundled`, `reqwest/rustls-tls`, `sha2`, `uuid`, `chrono`, `thiserror`, `base64`, `percent-encoding` as needed for GitHub API). |
| `src-tauri/capabilities/default.json` | Grant `dialog:default`, `fs:default` scoped to app data, and any `sql` or custom permissions if introduced (Tauri 2 ACL — use documented scope patterns for `AppLocalData`). |
| `src-tauri/tauri.conf.json` | Add window `label: "main"` if missing; keep `beforeDevCommand` / `frontendDist` unchanged unless build breaks. |
| `src-tauri/src/lib.rs` | Register modules, `manage` shared `AppState` (`Mutex` around `Connection` + paths), register all commands, remove demo `greet` when superseded. |
| `src-tauri/src/error.rs` | `AppError` enum + `Into<String>` / `Serialize` for frontend. |
| `src-tauri/src/paths.rs` | Resolve `library_root`, `db_path` under `tauri::path::BaseDirectory::AppLocalData`. |
| `src-tauri/src/db.rs` | Open SQLite, schema create, CRUD for `sheets` table matching spec §5.1. |
| `src-tauri/src/hash.rs` | `sha256_file(path) -> Result<String, AppError>` + unit tests. |
| `src-tauri/src/conflict.rs` | Pure functions: `classify_row(local_hash, stored_remote_sha, remote_sha, local_dirty)` → enum `PullDecision` / `Conflict`. |
| `src-tauri/src/settings.rs` | Non-secret fields: `owner`, `repo`, `branch`, `prefix` (default `""` or `sheets`); load/save JSON next to DB; **no token in JSON**. |
| `src-tauri/src/secrets.rs` | `token_get` / `token_set` / `token_clear` using `keyring` entry `service="com.blxie.tauri-app.github"`, user `pat`. |
| `src-tauri/src/github/client.rs` | Minimal REST: `get_tree`, `get_blob`, `put_contents` (create/update with SHA), `delete_file` optional for rename flows. |
| `src-tauri/src/github/mod.rs` | Re-export + URL helpers (`repo_base`). |
| `src-tauri/src/sync.rs` | Orchestrate pull-then-push pass, emit structured `SyncReport` including `conflicts: Vec<ConflictItem>` requiring UI resolution. |
| `src-tauri/src/commands/mod.rs` | Thin `#[tauri::command]` wrappers only. |
| `src/main.ts` | Add `vue-router` bootstrap. |
| `src/router/index.ts` | Routes: `/`, `/sheet/:id`, `/settings`. |
| `src/views/LibraryView.vue` | List + search + import button + delete. |
| `src/views/ReaderView.vue` | Branch on `kind`: text / pdf / image subcomponents or inline. |
| `src/views/SettingsView.vue` | GitHub fields + test connection + save + clear token. |
| `src/components/ConflictModal.vue` | Renders one conflict at a time or a queue; calls `resolve_conflict`. |
| `src/styles.css` | Global typography baseline for readability. |
| `README.md` | Add PAT scope guidance + GitHub file size note (100MB limit) per spec §6.2 / §8. |

---

### Task 1: Tauri ACL + window label + remove demo surface

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/lib.rs` (temporary keep `greet` until Task 4 removes it)

- [ ] **Step 1: Add main window label**

Edit `src-tauri/tauri.conf.json` under `app.windows[0]`:

```json
"label": "main",
"title": "Guitar Sheet"
```

- [ ] **Step 2: Expand default capability for dialog + scoped fs**

Replace `src-tauri/capabilities/default.json` with:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Main window capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "fs:default",
    "fs:allow-app-read",
    "fs:allow-app-write",
    "fs:allow-app-read-recursive",
    "fs:allow-app-write-recursive"
  ]
}
```

If the project’s generated schema rejects `fs:allow-app-*` names, follow the **exact** identifiers shown in `src-tauri/gen/schemas/desktop-schema.json` for recursive AppLocalData access and substitute accordingly (do not leave ambiguous placeholders—pick the working identifiers from the generated schema file on disk).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "chore(tauri): label main window and extend fs/dialog permissions"
```

---

### Task 2: Dependencies + error + paths modules

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/paths.rs`

- [ ] **Step 1: Append dependencies**

In `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
rusqlite = { version = "0.32", features = ["bundled"] }
sha2 = "0.10"
hex = "0.4"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
keyring = "3"
base64 = "0.22"
```

Run:

```bash
cd /Users/blxie/workspace/guitar-sheet/src-tauri && cargo check
```

Expected: downloads crates; may fail until new modules are wired—proceed to Step 2.

- [ ] **Step 2: Create `error.rs`**

Create `src-tauri/src/error.rs`:

```rust
use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("sql: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http: {0}")]
    Http(String),
    #[error("bad input: {0}")]
    BadInput(String),
    #[error("keyring: {0}")]
    Keyring(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct UserFacingError {
    pub code: &'static str,
    pub message: String,
}

impl From<AppError> for UserFacingError {
    fn from(value: AppError) -> Self {
        UserFacingError {
            code: "app_error",
            message: value.to_string(),
        }
    }
}
```

- [ ] **Step 3: Create `paths.rs`**

Create `src-tauri/src/paths.rs`:

```rust
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
        let resolver = app.path();
        let data_dir = resolver
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
```

- [ ] **Step 4: Register modules in `lib.rs`**

At top of `src-tauri/src/lib.rs`:

```rust
mod error;
mod paths;
```

- [ ] **Step 5: `cargo check`**

Run:

```bash
cd /Users/blxie/workspace/guitar-sheet/src-tauri && cargo check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/error.rs src-tauri/src/paths.rs src-tauri/src/lib.rs
git commit -m "feat(core): add errors, paths, and backend dependencies"
```

---

### Task 3: SQLite schema + repository API (TDD)

**Files:**
- Create: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing unit test for schema**

Create `src-tauri/src/db.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

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
}
```

Add `init_schema` stub returning `Ok(())` to make compile fail on missing table—actually test expects table; implement `init_schema` in Step 3.

- [ ] **Step 2: Run test (expect FAIL)**

```bash
cd /Users/blxie/workspace/guitar-sheet/src-tauri && cargo test creates_schema -- --nocapture
```

Expected: FAIL until `init_schema` creates `sheets`.

- [ ] **Step 3: Implement schema + helpers**

In `src-tauri/src/db.rs` implement:

```sql
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
```

Expose functions:

```rust
pub fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> { /* exec DDL */ }

pub struct SheetRow { /* fields */ }

pub fn insert_sheet(conn: &Connection, row: &SheetRow) -> Result<(), rusqlite::Error> { /* */ }
pub fn list_sheets(conn: &Connection, query: Option<&str>) -> Result<Vec<SheetRow>, rusqlite::Error> { /* */ }
pub fn get_sheet(conn: &Connection, id: &str) -> Result<Option<SheetRow>, rusqlite::Error> { /* */ }
pub fn update_hashes_and_remote(
    conn: &Connection,
    id: &str,
    local_hash: &str,
    remote_sha: Option<&str>,
    synced_at_rfc3339: Option<&str>,
) -> Result<(), rusqlite::Error> { /* */ }
pub fn delete_sheet(conn: &Connection, id: &str) -> Result<(), rusqlite::Error> { /* */ }
```

- [ ] **Step 4: Run tests**

```bash
cd /Users/blxie/workspace/guitar-sheet/src-tauri && cargo test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat(db): add sheets schema and repository helpers"
```

---

### Task 4: Content hashing utility (TDD)

**Files:**
- Create: `src-tauri/src/hash.rs`
- Modify: `src-tauri/src/lib.rs` (`mod hash;`)

- [ ] **Step 1: Failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn hashes_file() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"abc").unwrap();
        let h = sha256_file(f.path()).unwrap();
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
```

Add `tempfile` to `[dev-dependencies]` in `Cargo.toml` for tests only.

- [ ] **Step 2: Implement `sha256_file`**

Use `sha2::{Sha256, Digest}` streaming read 8KiB buffers.

- [ ] **Step 3: `cargo test` PASS + commit**

```bash
git add src-tauri/src/hash.rs src-tauri/Cargo.toml src-tauri/src/lib.rs
git commit -m "feat(core): sha256 helper for sync/conflict detection"
```

---

### Task 5: AppState + import/list/delete commands

**Files:**
- Create: `src-tauri/src/state.rs`
- Create: `src-tauri/src/commands/sheets.rs`
- Modify: `src-tauri/src/lib.rs`

**Ordering note:** `import_sheet` must compute `remote_path` using the same prefix rules as `settings.rs`. Implement **Task 6 (`settings.rs` + default prefix normalization)** *before* wiring `remote_path`, or temporarily hardcode `remote_path = format!("sheets/{id}{ext}")` in Task 5 and replace with settings-aware builder in Task 8—pick one approach in commits, do not leave both behaviors active.

- [ ] **Step 1: Define `AppState`**

`src-tauri/src/state.rs`:

```rust
use std::sync::Mutex;
use rusqlite::Connection;
use crate::paths::AppPaths;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub paths: AppPaths,
}
```

- [ ] **Step 2: Open DB on startup**

In `run()` before `Builder`:

```rust
let paths = paths::AppPaths::resolve(&app.handle())?;
paths.ensure_dirs()?;
let conn = Connection::open(&paths.db_path)?;
db::init_schema(&conn)?;
let state = state::AppState {
    conn: Mutex::new(conn),
    paths,
};
```

Register `.manage(state)`.

Register plugins:

```rust
.plugin(tauri_plugin_dialog::init())
.plugin(tauri_plugin_fs::init())
```

- [ ] **Step 3: Implement import**

`#[tauri::command]` `import_sheet(source_path: String, maybe_title: Option<String>) -> SheetMeta`

Algorithm:

1. Validate extension → map to `kind`.  
2. `id = Uuid::new_v4()`; `dest_dir = library_dir.join(&id)`; `create_dir_all`.  
3. Preserve extension: `dest = dest_dir.join(format!("content{}", ext))`; `std::fs::copy`.  
4. `hash = sha256_file(&dest)`; `modified = chrono::Utc::now().to_rfc3339()`.  
5. Compute `remote_path` using normalized `prefix` from `settings::load` (Task 6) as `format!("{}{}{}", prefix, id, ext)` where `prefix` ends with `/` (default `sheets/`).  
6. Insert DB row (`remote_blob_sha` NULL).  
7. Return serializable `SheetMeta`.

- [ ] **Step 4: Commands `list_sheets`, `get_sheet`, `delete_sheet`, `rename_sheet_title`**

Wire serde types shared with frontend.

- [ ] **Step 5: Remove `greet` from handler list**

- [ ] **Step 6: Manual smoke**

```bash
cd /Users/blxie/workspace/guitar-sheet && npm run tauri dev
```

Invoke from temporary Vue button or `invoke` in console—confirm list updates.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src
git commit -m "feat(sheets): sqlite-backed import and library commands"
```

---

### Task 6: Settings JSON + keyring PAT

**Files:**
- Create: `src-tauri/src/settings.rs`
- Create: `src-tauri/src/secrets.rs`
- Create: `src-tauri/src/commands/settings.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: `settings.json` shape**

```json
{
  "owner": "octocat",
  "repo": "hello-world",
  "branch": "main",
  "prefix": "guitar-sheet/"
}
```

Functions: `load(app: &AppHandle) -> Settings`, `save(...)`.

- [ ] **Step 2: Keyring**

`secrets.rs` wraps `Entry::new("GuitarSheet/GitHub", "pat")?` (adjust to match `keyring` 3 API exactly per docs) with `set_password`, `get_password`, `delete_credential`.

- [ ] **Step 3: Commands**

`get_github_settings`, `set_github_settings`, `github_pat_status` (boolean), `clear_github_pat`.

- [ ] **Step 4: Commit**

```bash
git commit -am "feat(settings): persist repo config and PAT in keychain"
```

---

### Task 7: GitHub REST client (integration-tested with mock server optional)

**Files:**
- Create: `src-tauri/src/github/client.rs`
- Create: `src-tauri/src/github/mod.rs`
- Modify: `src-tauri/src/lib.rs`

Implement functions (all return `AppResult`):

```rust
pub async fn get_contents_sha(
    client: &reqwest::Client,
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
) -> AppResult<Option<(String, String)>> // (sha, base64 content) or None if 404

pub async fn put_contents(
    client: &reqwest::Client,
    token: &str,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    message: &str,
    content_bytes: &[u8],
    previous_sha: Option<&str>,
) -> AppResult<String> // new sha
```

Use `https://api.github.com/repos/{owner}/{repo}/contents/{path}` with `ref` query for GET. Encode body per GitHub JSON schema.

- [ ] **Step 1: Unit test base64 length limits** (small fixture string).

- [ ] **Step 2: `cargo test` + commit**

```bash
git commit -am "feat(github): REST contents get/put helpers"
```

---

### Task 8: Sync engine + conflict surface to UI

**Files:**
- Create: `src-tauri/src/sync.rs`
- Create: `src-tauri/src/commands/sync.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Pure classifier tests (`conflict.rs`)**

Implement function:

```rust
pub enum SyncClassify {
    PullFastForward { remote_sha: String },
    PushFastForward,
    Conflict { remote_sha: String, local_hash: String },
    RemoteMissing,
}

pub fn classify(
    local_hash: &str,
    last_pushed_remote_sha: Option<&str>,
    remote_sha_opt: Option<&str>,
) -> SyncClassify { /* per spec §6 */ }
```

Tests:

```rust
assert!(matches!(classify("A", Some("R1"), Some("R2")), SyncClassify::Conflict { .. }));
```

- [ ] **Step 2: `sync_pull_push` command**

Pseudo-steps inside Rust:

1. Load token + settings; error if incomplete.  
2. For each DB row with `remote_path` set: GET remote; classify.  
3. Collect `conflicts` into `Vec` returned to UI **without mutation** when any conflict exists OR process sequentially—**MVP simpler**: stop entire sync on first conflict, return payload; user resolves via `resolve_conflict_choice`.  

Given spec wants per-file B, implement **queue**: return `Vec<ConflictItem>`.

4. For non-conflict rows: apply updates + DB commits transactionally.

5. Push phase: for rows where local hash differs from `remote_blob_sha`, PUT contents.

- [ ] **Step 3: `resolve_conflict` command**

Parameters: `sheet_id`, `choice` enum `KeepLocal | TakeRemote | SaveCopy`.

Implement `SaveCopy` by duplicating file to new UUID directory + new `remote_path` + DB insert.

- [ ] **Step 4: Commit**

```bash
git commit -am "feat(sync): pull/push with explicit conflict resolution"
```

---

### Task 9: Vue router + Library + Reader + Settings skeleton

**Files:**
- Modify: `package.json` (add `vue-router`, `pdfjs-dist`)
- Create: `src/router/index.ts`
- Modify: `src/main.ts`
- Replace: `src/App.vue` as shell with `<router-view />`
- Create: `src/views/LibraryView.vue`
- Create: `src/views/ReaderView.vue`
- Create: `src/views/SettingsView.vue`

- [ ] **Step 1: Install deps**

```bash
cd /Users/blxie/workspace/guitar-sheet && npm install vue-router@4 pdfjs-dist@4
```

- [ ] **Step 2: Router code**

`src/router/index.ts`:

```ts
import { createRouter, createWebHistory } from "vue-router";
import LibraryView from "../views/LibraryView.vue";
import ReaderView from "../views/ReaderView.vue";
import SettingsView from "../views/SettingsView.vue";

export const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: "/", name: "library", component: LibraryView },
    { path: "/sheet/:id", name: "reader", component: ReaderView, props: true },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});
```

`src/main.ts` mounts with `app.use(router)`.

- [ ] **Step 3: Library view**

Use `@tauri-apps/api/core` `invoke("list_sheets")`; file pick via `@tauri-apps/plugin-dialog` `open` from frontend (requires dialog permission) **or** Rust command wrapping dialog—pick one pattern and stay consistent; plan recommends **frontend dialog** for simpler path strings.

After pick: `invoke("import_sheet", { sourcePath: selected })`.

- [ ] **Step 4: Reader view**

- `text`: `<pre>` with CSS vars for font size / line-height sliders.  
- `image`: `<img :src="assetUrl">` where `assetUrl` is `convertFileSrc` from `@tauri-apps/api/core` pointing to copied file path returned by `get_sheet`.  
- `pdf`: use `pdfjs-dist` `getDocument({ url: convertFileSrc(...) })` with worker configured in `vite.config.ts` per Vite + pdf.js guidance.

- [ ] **Step 5: Run web + desktop**

```bash
npm run dev
npm run tauri dev
```

Expected: navigate between views without errors.

- [ ] **Step 6: Commit**

```bash
git add src package.json package-lock.json vite.config.ts
git commit -m "feat(ui): library, reader, settings shell with router"
```

---

### Task 10: Vite config for PDF worker + security notes

**Files:**
- Modify: `vite.config.ts`

Add worker alias/copy per `pdfjs-dist` ESM worker file location; ensure `optimizeDeps.include` contains `pdfjs-dist`.

Concrete snippet (adjust paths to match installed `pdfjs-dist` version files that exist in `node_modules`):

```ts
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import path from "node:path";

export default defineConfig(() => ({
  plugins: [vue()],
  resolve: {
    alias: {
      // Example — verify actual worker .mjs path after npm install:
      pdfjs: path.resolve(__dirname, "node_modules/pdfjs-dist/build/pdf.mjs"),
    },
  },
  optimizeDeps: {
    include: ["pdfjs-dist"],
  },
  // ...keep existing Tauri server config from template
}));
```

After editing, run `npm run build` and fix until PASS.

- [ ] **Commit**

```bash
git commit -am "chore(vite): wire pdfjs-dist worker for reader"
```

---

### Task 11: Conflict modal UX wired to commands

**Files:**
- Create: `src/components/ConflictModal.vue`
- Modify: `src/views/LibraryView.vue` (or dedicated `SyncPanel.vue`)

Flow:

1. User clicks **Sync** → `invoke("sync_pull_push")` returns `{ conflicts: [...] }`.  
2. If empty `{ ok: true }` toast.  
3. Else open modal with first conflict; buttons call `resolve_conflict` sequentially until list empty, then optionally auto-run sync again or instruct user to press sync once more—**pick deterministic approach in code comments** (recommended: auto rerun sync until clean, max 10 iterations guard).

- [ ] **Commit**

```bash
git commit -am "feat(ui): conflict resolution modal for github sync"
```

---

### Task 12: README PAT guidance + manual QA script

**Files:**
- Modify: `README.md`

Add section **GitHub 同步** listing:

- Classic PAT scopes (`repo` for private).  
- File size limit 100MB.  
- Token rotation steps.

Add **Manual QA checklist** mirroring spec §9.

- [ ] **Commit**

```bash
git commit -am "docs: github sync notes and manual QA checklist"
```

---

## Plan self-review (against `docs/superpowers/specs/2026-05-02-guitar-sheet-design.md`)

**Spec coverage mapping**

| Spec section | Plan tasks |
|--------------|-----------|
| §4.1 导入类型 | Task 5 + Task 9 reader branches |
| §4.2 谱库 | Task 5 + Task 9 Library |
| §4.3 阅读视图 | Task 9–10 |
| §4.4 GitHub 设置/同步 | Tasks 6–8 + 11 |
| §5 数据模型 | Task 3 + Task 5 paths (`library/{id}/content.ext`) |
| §6 冲突 B | Task 8 classifier + Task 11 UI |
| §7 架构 | Tasks 2–8 Rust, 9–11 Vue |
| §8 错误处理 | Task 2 `AppError` + command `map_err` to `UserFacingError` |
| §9 测试 | Tasks 3–4 unit tests; mock GitHub optional follow-up |

**Placeholder scan:** none intentionally added; any `fs:allow-*` permission names must be resolved against generated schema (called out explicitly in Task 1).

**Type consistency:** Frontend DTO names (`SheetMeta`, `ConflictItem`, `ConflictChoice`) must match serde on Rust commands—define shared TypeScript interfaces in `src/types/sheet.ts` in Task 9 and keep field names identical.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-02-guitar-sheet-mvp.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
