# Shell / Theme / Window UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the [壳层与主题 UI 规格](../specs/2026-05-02-shell-theme-ui-design.md): default maximized window, **light-paper** design tokens, unified app chrome (no stacked duplicate nav), responsive toolbars, pin (**always-on-top**) with persistence, optional fullscreen toggle, and `data-theme` ready for future presets.

**Architecture:** Global CSS variables live in a single imported stylesheet; `#app` carries `data-theme`. **App.vue** owns one chrome row (brand, primary nav, pin, optional fullscreen, settings) and a flex `main` slot for `RouterView`. **UI preferences** (`always_on_top`, `theme_id`) persist in the app data directory as JSON via new Tauri commands, mirroring `github_settings.json`. Frontend applies `getCurrentWindow().setAlwaysOnTop` after load and on window resume/focus when prefs say pinned. **No Vitest** in this repo yet — verification is `vue-tsc`/`npm run build`, `cargo build`, and manual Tauri checks (see header note).

**Tech Stack:** Tauri 2, Vue 3, Vue Router 4, TypeScript, Vite 6, scoped CSS + global token file, `@tauri-apps/api/window`.

**Verification note (no placeholder):** This codebase does **not** ship a frontend unit-test runner. Tasks use **compile checks** and **manual QA** instead of "write failing test" steps. If you add Vitest later, fold pin/state helpers into `@vue/test-utils` tests in a follow-up plan.

---

## File map (create / modify)

| File | Responsibility |
|------|------------------|
| `docs/superpowers/specs/2026-05-02-shell-theme-ui-design.md` | Normative spec (read-only for implementer). |
| `src/styles/light-paper.css` | **Create** — `:root` / `[data-theme="light-paper"]` CSS custom properties + base `body` / link colors consumed by chrome and views. |
| `src/main.ts` | **Modify** — `import "./styles/light-paper.css"` before mount; set `document.documentElement.dataset.theme` from default or future prefs. |
| `src/App.vue` | **Modify** — Replace ad-hoc `.nav` with token-based **App chrome** (flex row + `main`); wire pin + (optional) fullscreen; remove duplicate entry points where views re-link settings. |
| `src/types/uiPrefs.ts` | **Create** — TS mirror of JSON shape `{ always_on_top: boolean; theme_id: string }` with defaults. |
| `src/views/LibraryView.vue` | **Modify** — Toolbar: use `var(--…)` for backgrounds/borders; add **narrow breakpoint** (e.g. `max-width: 42rem` for *main* column only — use container or media query) to move secondary actions into a **「⋯」** disclosure (`<details>` or small popover) so nothing is unreachable. |
| `src/views/SettingsView.vue` | **Modify** — Drop redundant top links if chrome already has 设置; keep page content; use tokens for surfaces. |
| `src/components/SheetReaderPanel.vue` | **Modify** — `.bar`, `.tab`, `.pdf-wrap` colors → tokens; **narrow**: collapse **字号/行距** (and PDF pager if needed) under **⋯** in `.bar` when `max-width` under chosen breakpoint (e.g. 36rem on the reader panel — use `@media` on `.reader`). |
| `src-tauri/tauri.conf.json` | **Modify** — `app.windows[0].maximized: true` (keep `width`/`height` as restore size). |
| `src-tauri/capabilities/default.json` | **Modify** — Add `core:window:allow-set-always-on-top`, `core:window:allow-is-always-on-top`; if implementing fullscreen button: `core:window:allow-set-fullscreen`, `core:window:allow-is-fullscreen`. |
| `src-tauri/src/ui_prefs.rs` | **Create** — Load/save `ui_prefs.json` under `AppPaths::data_dir` (same pattern as `settings.rs` + `GitHubSettings`). |
| `src-tauri/src/commands/ui_prefs.rs` | **Create** — `get_ui_prefs`, `set_ui_prefs` (partial update OK: merge JSON). |
| `src-tauri/src/lib.rs` | **Modify** — Register new invoke handlers. |
| `src-tauri/src/commands/mod.rs` | **Modify** — `pub mod ui_prefs;` if you use a `commands` module file; else add mod in `lib.rs` — follow existing `commands::folders` pattern. |

---

### Task 1: Default maximized window

**Files:**

- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Add `maximized` to main window**

Edit the `windows[0]` object to include maximized flag (exact JSON):

```json
"windows": [
  {
    "label": "main",
    "title": "Guitar Sheet",
    "width": 1100,
    "height": 720,
    "maximized": true
  }
]
```

Use at least `1100×720` as fallback restore size (optional tweak from `800×600` so first un-maximize is usable). If your schema rejects unknown keys, match [Tauri 2 window config](https://v2.tauri.app/reference/config/#windowconfig).

- [ ] **Step 2: Verify**

Run: `cd /Users/blxie/workspace/guitar-sheet && npm run tauri build` **or** `npm run tauri dev` and confirm the window opens **maximized**.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat(window): open main window maximized by default"
```

---

### Task 2: Tauri capabilities for window controls

**Files:**

- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Extend permissions array**

Append (preserve existing entries):

```json
"core:window:allow-set-always-on-top",
"core:window:allow-is-always-on-top"
```

If Task 6 adds fullscreen toggle, also add:

```json
"core:window:allow-set-fullscreen",
"core:window:allow-is-fullscreen"
```

- [ ] **Step 2: Verify**

Run: `cd /Users/blxie/workspace/guitar-sheet/src-tauri && cargo build`

Expected: **success** (schema validates capabilities).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/capabilities/default.json
git commit -m "chore(tauri): allow always-on-top window APIs"
```

---

### Task 3: Persist `ui_prefs.json` (Rust)

**Files:**

- Create: `src-tauri/src/ui_prefs.rs`
- Create: `src-tauri/src/commands/ui_prefs.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/mod.rs` (if present; else inline `mod` in `lib.rs`)

- [ ] **Step 1: Define struct and file IO in `ui_prefs.rs`**

```rust
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::paths::AppPaths;

const FILE_NAME: &str = "ui_prefs.json";

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
    let v: UiPrefs = serde_json::from_str(&s).map_err(AppError::Json)?;
    Ok(v)
}

pub fn save(paths: &AppPaths, prefs: &UiPrefs) -> AppResult<()> {
    let p = paths.data_dir.join(FILE_NAME);
    let body = serde_json::to_string_pretty(prefs).map_err(AppError::Json)?;
    std::fs::write(&p, body).map_err(AppError::Io)?;
    Ok(())
}
```

- [ ] **Step 2: Commands `get_ui_prefs` / `set_ui_prefs`**

`get_ui_prefs` returns `UiPrefs` using `state.paths`.

`set_ui_prefs` accepts partial JSON: read `load`, merge fields present in incoming value (use `serde_json::Value` + manual merge, or `#[serde(default)]` on an `UiPrefsPatch` struct with `Option<bool>` / `Option<String>`), then `save`.

Register both in `invoke_handler!`.

- [ ] **Step 3: Verify**

Run: `cd /Users/blxie/workspace/guitar-sheet/src-tauri && cargo build`

Expected: **success**.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/ui_prefs.rs src-tauri/src/commands/ui_prefs.rs src-tauri/src/lib.rs src-tauri/src/commands/mod.rs
git commit -m "feat: persist UI prefs (always-on-top, theme id) in ui_prefs.json"
```

---

### Task 4: Global **light-paper** tokens

**Files:**

- Create: `src/styles/light-paper.css`
- Modify: `src/main.ts`

- [ ] **Step 1: Add token file** (minimal set — extend as you migrate views)

```css
:root,
[data-theme="light-paper"] {
  --gs-bg-app: #f4f4f5;
  --gs-bg-surface: #ffffff;
  --gs-bg-muted: #fafafa;
  --gs-border: #e5e5e7;
  --gs-text: #111827;
  --gs-text-muted: #6b7280;
  --gs-link: #2563eb;
  --gs-link-hover: #1d4ed8;
  --gs-primary-border: #2563eb;
  --gs-primary-bg: #eff6ff;
  --gs-danger: #b00020;
  --gs-success: #0a5;
  --gs-radius-sm: 6px;
  --gs-radius-md: 8px;
  --gs-shadow-sm: 0 1px 2px rgb(0 0 0 / 0.05);
}

body {
  color: var(--gs-text);
  background: var(--gs-bg-app);
}
```

- [ ] **Step 2: Import in `main.ts`**

```ts
import "./styles/light-paper.css";
```

After `createApp`, before or after `mount`, set:

```ts
document.documentElement.dataset.theme = "light-paper";
```

(Later: read from `get_ui_prefs` once Task 5 wires invoke.)

- [ ] **Step 3: Verify**

Run: `npm run build`

Expected: **PASS** (no TS errors).

- [ ] **Step 4: Commit**

```bash
git add src/styles/light-paper.css src/main.ts
git commit -m "feat(ui): add light-paper CSS tokens and data-theme default"
```

---

### Task 5: App chrome + pin + apply persisted always-on-top

**Files:**

- Create: `src/types/uiPrefs.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: TS type**

```ts
export interface UiPrefs {
  alwaysOnTop: boolean;
  themeId: string;
}

export const defaultUiPrefs = (): UiPrefs => ({
  alwaysOnTop: false,
  themeId: "light-paper",
});
```

Use `invoke<UiPrefs>("get_ui_prefs")` — map Rust `camelCase` to these names (align serde on both sides).

- [ ] **Step 2: `App.vue` script**

- On `onMounted`: `invoke("get_ui_prefs")` → set `document.documentElement.dataset.theme = prefs.themeId`, `await getCurrentWindow().setAlwaysOnTop(prefs.alwaysOnTop)`.
- Pin button toggles local ref, calls `setAlwaysOnTop`, then `invoke("set_ui_prefs", { patch: { alwaysOnTop: next } })` (shape matches your Rust command).
- Subscribe to window events via `@tauri-apps/api/event` + `getCurrentWindow().listen` for **`TauriEvent.WINDOW_RESUMED`** or **`WINDOW_FOCUS`** (import from `event` package) to **re-apply** `setAlwaysOnTop(prefs.alwaysOnTop)` when user returns to the app (covers hide/show edge per spec §6.2).
- On `setAlwaysOnTop` rejection: set a small `ref` banner string `"无法固定窗口（部分 Linux 桌面不支持）"` under chrome.

- [ ] **Step 3: `App.vue` template**

Single chrome row:

- Left: `RouterLink` brand to `/`, optional subtle route label (e.g. "谱库" when `route.path === '/'`).
- Right: **pin** `<button type="button" :aria-pressed="pinned" :title="…">` (use inline SVG thumbtack or Unicode `📌` only if acceptable — prefer SVG), then `RouterLink` to `/settings`.
- `main.shell-body`: `<RouterView />` only; keep `flex:1; min-height:0` from current `#shell` pattern.

Remove duplicate **设置** links from child views in Task 7.

- [ ] **Step 4: Verify**

Run: `npm run build`

Manual: launch app, toggle pin, restart app — state restored; pin affects z-order vs another window.

- [ ] **Step 5: Commit**

```bash
git add src/App.vue src/types/uiPrefs.ts
git commit -m "feat(ui): app chrome with pin and persisted always-on-top"
```

---

### Task 6 (optional): Fullscreen toggle in chrome

**Files:**

- Modify: `src/App.vue`
- Ensure capabilities from Task 2 include set/is fullscreen.

- [ ] **Step 1: Add button** next to pin calling `getCurrentWindow().setFullscreen(!await isFullscreen())` (use `isFullscreen()` from API).

- [ ] **Step 2: Verify manually** — toggle does not break Router; exit fullscreen restores.

- [ ] **Step 3: Commit**

```bash
git add src/App.vue src-tauri/capabilities/default.json
git commit -m "feat(window): optional fullscreen toggle in chrome"
```

---

### Task 7: Migrate Library + Settings to tokens + overflow on narrow

**Files:**

- Modify: `src/views/LibraryView.vue`
- Modify: `src/views/SettingsView.vue`

- [ ] **Step 1: Library toolbar**

Replace hardcoded hex colors in **scoped** styles with `var(--gs-…)` for sidebar, borders, primary button, muted text. Add a **details/summary** or button-activated menu: below `max-width: 42rem` on viewport (or `@container` if you add container-type on `.main`), hide **同步 / 刷新** in row and show **「更多 ▾」** containing them.

- [ ] **Step 2: Settings**

Remove top `RouterLink` duplicates to settings if chrome already links there; keep `RouterLink to="/"` as **返回谱库** if chrome does not show back — **Decision:** chrome always shows brand → `/`, so Settings body can use a simple heading without duplicate nav row.

- [ ] **Step 3: Verify**

`npm run build`; manually resize window < 400px width — all library actions reachable.

- [ ] **Step 4: Commit**

```bash
git add src/views/LibraryView.vue src/views/SettingsView.vue
git commit -m "style(ui): token-based library/settings + narrow overflow toolbar"
```

---

### Task 8: Reader panel chrome + controls responsive

**Files:**

- Modify: `src/components/SheetReaderPanel.vue`

- [ ] **Step 1: Tokenize** `.bar`, `.tab`, `.reader-body`, `.pdf-controls` backgrounds/borders/text.

- [ ] **Step 2: Narrow media query** e.g. `@media (max-width: 36rem) { … }` inside scoped block: hide `.controls` row for text; add `<details class="reader-overflow">` in `.bar` with summary **⋯** containing duplicated range inputs **or** links that focus controls (simplest: move the two `<label>` ranges inside `<details>` in narrow mode using CSS only is hard — **acceptable approach:** duplicate controls in `details` for small screens only, hide the original `.controls` with `display:none` under media query).

- [ ] **Step 3: PDF** — under same narrow query, move pager into `details` or stack vertically.

- [ ] **Step 4: Verify**

`npm run build`; open a text sheet, shrink panel — font/line sliders still usable.

- [ ] **Step 5: Commit**

```bash
git add src/components/SheetReaderPanel.vue
git commit -m "style(reader): light-paper tokens and narrow overflow for controls"
```

---

### Task 9: Wire `theme_id` from prefs to DOM (if not done in Task 5)

**Files:**

- Modify: `src/App.vue` or `src/main.ts`

- [ ] **Step 1:** After `get_ui_prefs`, set `document.documentElement.dataset.theme = prefs.themeId` (validate allowed values; fallback `light-paper`).

- [ ] **Step 2: Commit**

```bash
git add src/App.vue
git commit -m "feat(ui): apply theme id from ui_prefs to data-theme"
```

---

## Spec coverage (self-review)

| Spec section | Task(s) |
|--------------|---------|
| §3 默认最大化 | Task 1 |
| §3 可选全屏 | Task 6 |
| §4.1 单一 chrome | Task 5, 7 |
| §4.2 主内容槽 flex | Already partially in `App.vue` — Task 5 ensures chrome + `main` |
| §4.3 响应式谱库 | Task 7 |
| §5 light-paper + `data-theme` | Tasks 4, 9 |
| §6 图钉 + 持久化 + 边界 | Tasks 2, 3, 5 |
| §7 分期主题扩展 | Task 4/9 reserve `theme_id` only (no importer) |

**Placeholder scan:** None intentional; all commands and file names are concrete.

**Type consistency:** Rust `camelCase` serde ↔ TS `alwaysOnTop` / `themeId` — use identical JSON keys in `invoke` payloads.

---

**Plan complete and saved to** `docs/superpowers/plans/2026-05-02-shell-theme-ui.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration. **REQUIRED SUB-SKILL:** superpowers:subagent-driven-development.

2. **Inline Execution** — Execute tasks in this session using executing-plans with batch checkpoints. **REQUIRED SUB-SKILL:** superpowers:executing-plans.

**Which approach do you want?**
