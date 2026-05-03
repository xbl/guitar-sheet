# 前端主题与组件规范

本文档描述 **guitar-sheet** 当前前端实现中的主题（设计令牌）、壳层布局约定与 Vue 组件实践，供后续功能与改版对齐。若实现与本文冲突，以仓库代码为准。

---

## 1. 主题系统

### 1.1 激活方式

- 在 **`document.documentElement`（即 `<html>`）** 上设置 **`data-theme`**，取值为主题 id。
- 全局样式通过 **`[data-theme="<id>"]`** 选择器覆盖 CSS 自定义属性（变量）。
- `main.ts` 按顺序引入：`light-paper.css`，再 `dark-app.css`；后引入的文件在**同优先级**下覆盖同名变量（当 `data-theme` 匹配对应块时）。

### 1.2 内置主题 id

| id | 说明 | 样式文件 |
|----|------|----------|
| `light-paper` | 浅色纸张风 | `src/styles/light-paper.css` |
| `dark-app` | 深色生产力风（默认） | `src/styles/dark-app.css` |

- 持久化：`get_ui_prefs` / `set_ui_prefs` 中的 `themeId`（JSON 为 camelCase，与 Rust serde 一致）。
- 服务端校验：`src-tauri/src/ui_prefs.rs` 的 `normalize_theme_id` 仅接受上述两种 id；其它值会归一为 **`dark-app`**。
- 应用入口：`App.vue` 在挂载后 `invoke("get_ui_prefs")`，将 `prefs.themeId` 写到 `document.documentElement.dataset.theme`；失败或非 Tauri 环境时有前端回退（默认使用深色令牌）。

### 1.3 设计令牌（CSS 变量）

所有界面色与圆角应优先使用 **`--gs-*`** 变量，避免在组件内写死十六进制色（和弦图等特殊标注除外需在两种主题下可读）。

#### 布局与表面

| 变量 | 用途 |
|------|------|
| `--gs-bg-app` | 应用主背景（主内容区底色） |
| `--gs-bg-surface` | 卡片、输入、侧栏内嵌面板 |
| `--gs-bg-muted` | 壳顶栏、侧栏背景等弱一层表面 |
| `--gs-border` | 边框、分割线 |

#### 文本与交互

| 变量 | 用途 |
|------|------|
| `--gs-text` | 主文案 |
| `--gs-text-muted` | 次要说明、弱化标签 |
| `--gs-link` / `--gs-link-hover` | 链接色 |
| `--gs-primary-border` / `--gs-primary-bg` | 主操作强调（描边 + 浅底） |
| `--gs-danger` | 错误 |
| `--gs-success` | 成功提示 |

#### 形状与阴影

| 变量 | 用途 |
|------|------|
| `--gs-radius-sm` | 小圆角（按钮、输入），当前 **6px** |
| `--gs-radius-md` | 中圆角（卡片、树容器），当前 **8px** |
| `--gs-shadow-sm` | 轻阴影 |

#### 和弦图（谱面阅读）

| 变量 | 用途 |
|------|------|
| `--gs-chord` | 和弦符号强调色 |
| `--gs-chord-fret` | 品位线 |
| `--gs-chord-string` | 弦线 |
| `--gs-chord-dot` | 按弦点 |

#### 谱库树（文件夹 / 展开）

| 变量 | 用途 |
|------|------|
| `--gs-tree-accent` | 文件夹强调色 |
| `--gs-tree-accent-muted` | 弱化强调 |
| `--gs-tree-row-active` | 当前上下文 / 选中行背景 |
| `--gs-tree-chevron` | 折叠箭头 |

浅色主题中树强调偏橙；深色主题中偏蓝，与主导航强调一致。

### 1.4 扩展新主题的步骤

1. 新增 `src/styles/<theme-id>.css`，使用选择器 **`[data-theme="<theme-id>"] { ... }`** 定义完整 `--gs-*` 集合（与现有主题键名保持一致）。
2. 在 `main.ts` 中 **在 `light-paper.css` 之后** 引入新文件。
3. 在 **`ui_prefs.rs`** 的 `normalize_theme_id` 和（若需要）`default_theme` 中加入新 id。
4. 用 `set_ui_prefs` / 手动编辑数据目录下 `ui_prefs.json` 验证切换。

---

## 2. 壳层与页面结构

### 2.1 根节点

- Vue 挂载在 **`#app`**（见 `index.html`）。
- 根组件为 **`App.vue`** 中的 **`#shell`**：纵向 flex，包含顶栏 **`header.chrome`** 与 **`div.shell-content`**（内为 `<RouterView />`）。

### 2.2 顶栏（chrome）

- **左侧**：品牌链向 `/`，当前路由说明文案（`routeLabel`）。
- **右侧**：置顶（`alwaysOnTop`，状态持久化到 `ui_prefs`）。不使用独立「设置」链（设置入口在谱库侧栏）。
- 顶栏样式使用 **`--gs-bg-muted`**、**`--gs-border`**；图标按钮类名 **`icon-btn`**，激活态结合 **`--gs-primary-*`**。

### 2.3 路由视图

- **`/`**：`LibraryView` — 左侧栏 + 主阅读区。
- **`/settings`**：`SettingsView` — 表单与卡片布局。
- **`/sheet/:id`**：`ReaderView` — 独立阅读页。

---

## 3. 组件与代码约定

### 3.1 技术栈

- **Vue 3**，组合式 API，**`<script setup lang="ts">`**。
- 路由：**vue-router**，页面级视图放在 **`src/views/`**，可复用块放在 **`src/components/`**（含子目录如 `chords/`、`practice/`）。

### 3.2 样式

- **优先使用 scoped CSS**（`<style scoped>`），主题色一律 **`var(--gs-...)`**。
- 全局基础链接触色由 **`light-paper.css`** 中 `a` / `a:hover` 提供；组件内链接如需与壳层一致，保持使用语义化变量。
- 布局优先 **flex**；需要占满剩余高度时使用 **`min-height: 0`** 防止 flex 子项溢出撑破布局（谱库、阅读器已采用此模式）。

### 3.3 Tauri 调用

- 使用 **`@tauri-apps/api/core`** 的 **`invoke`**，类型与 Rust command 对齐。
- 与偏好相关的 TypeScript 类型见 **`src/types/uiPrefs.ts`**，与后端 JSON 字段命名保持一致（camelCase）。

### 3.4 谱库页（LibraryView）侧栏模式（参考）

以下为当前侧栏重复出现的 UI 形态，新增侧栏控件建议对齐：

- **`side-nav`**：`RouterLink` + 图标（**`side-nav-icon`**）+ 文案；当前页使用 **`active-class="is-active"`**。
- **`side-action`**：全宽按钮，左侧 **`side-action-icon`**；主操作可加 **`side-action-primary`**。
- **收起态**：根节点 **`sidebar`** 使用修饰类 **`is-collapsed`**；收起时隐藏正文区、仅保留图标轨与底栏切换按钮；持久化键 **`localStorage`**：`guitar-sheet.librarySidebarCollapsed`。

### 3.5 可访问性（简要）

- 纯图标按钮需提供 **`title`**、**`aria-label`** 或 **`sr-only`** 文案（参见 `App.vue` 置顶按钮）。
- 折叠侧栏按钮使用 **`aria-expanded`** 与清晰 **`aria-label`**。

---

## 4. 相关文件索引

| 区域 | 路径 |
|------|------|
| 浅色令牌 | `src/styles/light-paper.css` |
| 深色令牌 | `src/styles/dark-app.css` |
| 入口与样式引入 | `src/main.ts` |
| 壳层 | `src/App.vue` |
| 偏好类型 | `src/types/uiPrefs.ts` |
| 主题 id 校验与默认值 | `src-tauri/src/ui_prefs.rs` |
| 谱库树 | `src/components/LibrarySheetTree.vue` |
| 阅读面板 | `src/components/SheetReaderPanel.vue` |

---

## 5. 与规格文档的关系

产品层壳层与主题的设想见 **`docs/superpowers/specs/2026-05-02-shell-theme-ui-design.md`** 及对应计划。本文档侧重 **当前仓库已实现** 的约定；若规格与实现不一致，改版时应同步更新本文档。
