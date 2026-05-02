<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue"
import { invoke } from "@tauri-apps/api/core"
import { open } from "@tauri-apps/plugin-dialog"
import LibrarySheetTree from "../components/LibrarySheetTree.vue"
import SheetReaderPanel from "../components/SheetReaderPanel.vue"
import type { FolderNode } from "../types/folder"
import { buildLibraryTree, type LibraryTreeRow } from "../utils/libraryTree"
import type { ConflictEntry, SheetMeta, SyncOutcome } from "../types/sheet"
import type { TreeDndPayload } from "../utils/treeDnD"
import { readTreeDragPayload } from "../utils/treeDnD"

const sheets = ref<SheetMeta[]>([])
const folderTree = ref<FolderNode[]>([])
const libraryRows = computed<LibraryTreeRow[]>(() =>
  buildLibraryTree(folderTree.value, sheets.value),
)

const selectedSheetId = ref<string | null>(null)
/** 导入 / 新建文件夹 的目标父文件夹（null = 谱库根） */
const contextFolderId = ref<string | null>(null)

const error = ref<string | null>(null)
const syncMsg = ref<string | null>(null)
const conflicts = ref<ConflictEntry[]>([])

const searchQuery = ref("")
const newFolderName = ref("")
/** `true` = 文件夹折叠（隐藏子项） */
const collapsedFolders = ref<Record<string, boolean>>({})

const creatingSheet = ref(false)
/** 新建曲谱：与子组件约定 id，载入完成后自动进入正文编辑 */
const pendingTextEditSheetId = ref<string | null>(null)

const createFolderParentId = computed(() => contextFolderId.value)

const importTargetFolderId = computed(() => contextFolderId.value)

/** 拖放高亮：文件夹 id */
const highlightDropFolderId = ref<string | null>(null)
const highlightDropRoot = ref(false)
/** 谱库拖放后强制重载阅读区（路径变化） */
const readerReloadNonce = ref(0)

function clearDropHighlight() {
  highlightDropFolderId.value = null
  highlightDropRoot.value = false
}

function onFolderDropHover(id: string | null) {
  highlightDropFolderId.value = id
  highlightDropRoot.value = false
}

function onRootDragOver(e: DragEvent) {
  e.preventDefault()
  highlightDropFolderId.value = null
  highlightDropRoot.value = true
}

async function onRootDrop(e: DragEvent) {
  const drag = readTreeDragPayload(e)
  clearDropHighlight()
  if (!drag) return
  error.value = null
  try {
    if (drag.kind === "sheet") {
      await invoke("move_sheet", { sheetId: drag.id, targetFolderId: null })
    } else {
      await invoke("move_folder", { folderId: drag.id, newParentId: null })
    }
    await refresh()
    readerReloadNonce.value++
  } catch (err) {
    error.value = String(err)
  }
}

async function onTreeMoveDrop(payload: {
  target: { kind: "folder"; folderId: string }
  drag: TreeDndPayload
}) {
  clearDropHighlight()
  const { target, drag } = payload
  error.value = null
  try {
    if (drag.kind === "sheet") {
      await invoke("move_sheet", {
        sheetId: drag.id,
        targetFolderId: target.folderId,
      })
    } else {
      if (drag.id === target.folderId) return
      await invoke("move_folder", {
        folderId: drag.id,
        newParentId: target.folderId,
      })
    }
    await refresh()
    readerReloadNonce.value++
  } catch (err) {
    error.value = String(err)
  }
}

async function loadTree() {
  try {
    folderTree.value = await invoke<FolderNode[]>("list_folder_tree")
  } catch (e) {
    error.value = String(e)
  }
}

async function refreshList() {
  error.value = null
  try {
    const q = searchQuery.value.trim() || null
    sheets.value = await invoke<SheetMeta[]>("list_sheets", {
      query: q,
      folderId: null,
      tagSubstring: null,
    })
  } catch (e) {
    error.value = String(e)
  }
}

async function refresh() {
  await loadTree()
  await refreshList()
}

function onSheetTitleRenamed(payload: { id: string; title: string }) {
  const idx = sheets.value.findIndex((s) => s.id === payload.id)
  if (idx >= 0) {
    sheets.value[idx] = {
      ...sheets.value[idx],
      display_title: payload.title,
    }
  }
}

function onSheetDeleted(id: string) {
  if (pendingTextEditSheetId.value === id) {
    pendingTextEditSheetId.value = null
  }
  if (selectedSheetId.value === id) {
    selectedSheetId.value = null
  }
  void refreshList()
}

async function createNewSheet() {
  if (creatingSheet.value) return
  error.value = null
  creatingSheet.value = true
  try {
    const meta = await invoke<SheetMeta>("create_text_sheet", {
      title: "未命名",
      folderId: importTargetFolderId.value,
      initialContent: null,
    })
    await refresh()
    pendingTextEditSheetId.value = meta.id
    selectedSheetId.value = meta.id
    syncMsg.value = "已新建文本曲谱，标题与正文已进入编辑。"
  } catch (e) {
    error.value = String(e)
  } finally {
    creatingSheet.value = false
  }
}

async function createFolder() {
  const name = newFolderName.value.trim()
  if (!name) return
  error.value = null
  try {
    await invoke("create_folder", {
      parentId: createFolderParentId.value,
      name,
    })
    newFolderName.value = ""
    await loadTree()
  } catch (e) {
    error.value = String(e)
  }
}

async function pickImport() {
  error.value = null
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Tab / PDF / Image",
          extensions: ["txt", "md", "pdf", "png", "jpg", "jpeg", "webp"],
        },
      ],
    })
    if (selected === null) return
    const path = typeof selected === "string" ? selected : selected[0]
    await invoke("import_sheet", {
      sourcePath: path,
      maybeTitle: null,
      folderId: importTargetFolderId.value,
    })
    await refreshList()
    syncMsg.value = "已导入"
  } catch (e) {
    error.value = String(e)
  }
}

async function syncGitHub() {
  syncMsg.value = null
  conflicts.value = []
  error.value = null
  try {
    const out = await invoke<SyncOutcome>("sync_pull_push")
    conflicts.value = out.conflicts
    if (out.ok) {
      syncMsg.value = `同步完成：拉取 ${out.pulled}，推送 ${out.pushed}`
      await refresh()
    } else {
      syncMsg.value = `存在 ${out.conflicts.length} 个冲突，请在下方选择处理方式后再点「同步」。`
    }
  } catch (e) {
    error.value = String(e)
  }
}

async function resolveConflict(
  id: string,
  choice: "keep_local" | "take_remote" | "save_copy",
) {
  error.value = null
  try {
    await invoke("resolve_sheet_conflict", { sheetId: id, choice })
    syncMsg.value = "冲突已处理"
    conflicts.value = conflicts.value.filter((c: ConflictEntry) => c.sheet_id !== id)
    await refreshList()
  } catch (e) {
    error.value = String(e)
  }
}

let searchDebounce: ReturnType<typeof setTimeout> | null = null
function onSearchInput() {
  if (searchDebounce) clearTimeout(searchDebounce)
  searchDebounce = setTimeout(() => {
    void refreshList()
  }, 300)
}

function onPendingTextEditConsumed() {
  pendingTextEditSheetId.value = null
}

function toggleFolderCollapse(id: string) {
  const cur = collapsedFolders.value[id] ?? false
  collapsedFolders.value = {
    ...collapsedFolders.value,
    [id]: !cur,
  }
}

watch(selectedSheetId, (id) => {
  if (pendingTextEditSheetId.value !== null && id !== pendingTextEditSheetId.value) {
    pendingTextEditSheetId.value = null
  }
})

onMounted(() => {
  void refresh()
  document.addEventListener("dragend", clearDropHighlight)
})

onUnmounted(() => {
  document.removeEventListener("dragend", clearDropHighlight)
})
</script>

<template>
  <div class="layout">
    <aside class="sidebar">
      <h2 class="side-title">谱库</h2>
      <label class="sidebar-search">
        <span class="sidebar-search-label">搜索</span>
        <input
          v-model="searchQuery"
          type="search"
          placeholder="标题 / 路径 / 标签…"
          enterkeyhint="search"
          @input="onSearchInput"
        />
      </label>
      <div class="ctx-row">
        <span class="ctx-label">目标文件夹</span>
        <button
          type="button"
          class="ctx-btn"
          :class="{ on: contextFolderId === null }"
          @click="contextFolderId = null"
        >
          根目录
        </button>
      </div>
      <p class="hint small">
        点击文件夹名：在此下新建 / 导入 / <strong>新建曲谱</strong>；点击曲谱：右侧打开。可将曲谱或文件夹<strong>拖到文件夹名上</strong>以移入。
      </p>
      <div
        class="tree-drop-root"
        :class="{ 'is-target': highlightDropRoot }"
        @dragover="onRootDragOver"
        @drop.prevent="onRootDrop"
      >
        拖到此处 → 移至谱库根目录（移出文件夹）
      </div>
      <div class="tree-scroll">
        <template v-if="libraryRows.length">
          <LibrarySheetTree
            :is-root="true"
            :rows="libraryRows"
            :selected-sheet-id="selectedSheetId"
            :context-folder-id="contextFolderId"
            :collapsed-folders="collapsedFolders"
            :highlight-drop-folder-id="highlightDropFolderId"
            @select-sheet="selectedSheetId = $event"
            @select-folder="contextFolderId = $event"
            @toggle-folder-collapse="toggleFolderCollapse"
            @folder-drop-hover="onFolderDropHover"
            @move-drop="onTreeMoveDrop"
          />
        </template>
        <p v-else class="muted small">暂无文件夹与曲谱。可先导入或创建文件夹。</p>
      </div>
      <div class="new-folder">
        <input
          v-model="newFolderName"
          type="text"
          placeholder="新文件夹名称"
          maxlength="80"
          @keydown.enter="createFolder"
        />
        <button type="button" @click="createFolder">创建</button>
      </div>
      <p class="hint small">创建位置取决于当前选中的文件夹（蓝色高亮）。未选中则在根目录创建。</p>
    </aside>

    <main class="main">
      <header class="toolbar">
        <div class="actions">
          <button type="button" :disabled="creatingSheet" @click="createNewSheet">
            {{ creatingSheet ? "创建中…" : "新建曲谱" }}
          </button>
          <button type="button" @click="pickImport">导入谱子</button>
          <button type="button" class="primary toolbar-sync-wide" @click="syncGitHub">
            与 GitHub 同步
          </button>
          <button type="button" class="toolbar-sync-wide" @click="refresh">刷新</button>
          <details class="toolbar-more-narrow">
            <summary>更多 ▾</summary>
            <div class="toolbar-more-body">
              <button type="button" class="primary" @click="syncGitHub">与 GitHub 同步</button>
              <button type="button" @click="refresh">刷新</button>
            </div>
          </details>
        </div>
      </header>

      <p v-if="error" class="err">{{ error }}</p>
      <p v-if="syncMsg" class="ok">{{ syncMsg }}</p>

      <div class="reader-host">
        <SheetReaderPanel
          :sheet-id="selectedSheetId"
          :pending-text-edit-for-sheet-id="pendingTextEditSheetId"
          :reload-nonce="readerReloadNonce"
          variant="embed"
          @deleted="onSheetDeleted"
          @pending-text-edit-consumed="onPendingTextEditConsumed"
          @title-renamed="onSheetTitleRenamed"
        />
      </div>

      <section v-if="conflicts.length" class="conflicts">
        <h2>冲突</h2>
        <article v-for="c in conflicts" :key="c.sheet_id" class="card">
          <h3>{{ c.display_title }}</h3>
          <p class="mono">{{ c.remote_path }}</p>
          <div class="btns">
            <button type="button" @click="resolveConflict(c.sheet_id, 'keep_local')">保留本地</button>
            <button type="button" @click="resolveConflict(c.sheet_id, 'take_remote')">使用远端</button>
            <button type="button" @click="resolveConflict(c.sheet_id, 'save_copy')">另存为副本</button>
          </div>
        </article>
      </section>
    </main>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  flex: 1;
  min-height: 0;
  width: 100%;
  overflow: hidden;
  align-items: stretch;
}
.sidebar {
  width: min(19rem, 38vw);
  flex-shrink: 0;
  padding: 0.75rem 0.65rem 1rem;
  border-right: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}
.side-title {
  margin: 0 0 0.35rem;
  font-size: 1rem;
  font-weight: 700;
  color: var(--gs-text);
}
.sidebar-search {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  margin-bottom: 0.55rem;
}
.sidebar-search-label {
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--gs-text-muted);
  letter-spacing: 0.03em;
}
.sidebar-search input {
  width: 100%;
  box-sizing: border-box;
  padding: 0.4rem 0.5rem;
  font-size: 0.85rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
  color: var(--gs-text);
}
.ctx-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-wrap: wrap;
  margin-bottom: 0.35rem;
}
.ctx-label {
  font-size: 0.75rem;
  color: var(--gs-text-muted);
}
.ctx-btn {
  font-size: 0.78rem;
  padding: 0.2rem 0.45rem;
  border-radius: 4px;
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  cursor: pointer;
}
.ctx-btn.on {
  border-color: var(--gs-primary-border);
  background: var(--gs-primary-bg);
  font-weight: 600;
}
.tree-drop-root {
  flex-shrink: 0;
  margin-bottom: 0.45rem;
  padding: 0.45rem 0.5rem;
  font-size: 0.72rem;
  line-height: 1.35;
  color: var(--gs-text-muted);
  border: 1px dashed var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
}
.tree-drop-root.is-target {
  border-color: var(--gs-primary-border);
  color: var(--gs-link);
  background: var(--gs-primary-bg);
}
.tree-scroll {
  flex: 1;
  overflow: auto;
  min-height: 8rem;
  margin: 0.5rem 0;
  padding: 0.35rem 0.25rem;
  padding-right: 0.15rem;
  border-radius: var(--gs-radius-md);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
}
.hint {
  margin: 0;
  color: var(--gs-text-muted);
  line-height: 1.35;
}
.small {
  font-size: 0.78rem;
}
.new-folder {
  margin-top: 0.5rem;
  display: flex;
  gap: 0.35rem;
}
.new-folder input {
  flex: 1;
  min-width: 0;
  padding: 0.35rem 0.45rem;
  font-size: 0.85rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
}
.new-folder button {
  flex-shrink: 0;
  padding: 0.35rem 0.5rem;
  font-size: 0.85rem;
  cursor: pointer;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
}
.main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.reader-host {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.toolbar {
  flex-shrink: 0;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  justify-content: flex-end;
}
.toolbar-more-narrow {
  display: none;
}
.toolbar-more-narrow summary {
  cursor: pointer;
  list-style: none;
  font-size: 0.88rem;
  padding: 0.35rem 0.5rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-muted);
  color: var(--gs-text);
}
.toolbar-more-narrow summary::-webkit-details-marker {
  display: none;
}
.toolbar-more-body {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  margin-top: 0.35rem;
  padding: 0.35rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
  min-width: 10rem;
}
.toolbar-more-body button {
  width: 100%;
}
@media (max-width: 42rem) {
  .toolbar-sync-wide {
    display: none !important;
  }
  .toolbar-more-narrow {
    display: block;
  }
}
.actions button.primary {
  font-weight: 600;
}
.actions a {
  color: var(--gs-link);
  text-decoration: none;
  padding: 0.35rem 0.5rem;
}
.err {
  color: var(--gs-danger);
  padding: 0 0.75rem;
  flex-shrink: 0;
}
.ok {
  color: var(--gs-success);
  padding: 0 0.75rem;
  flex-shrink: 0;
}
.muted {
  color: var(--gs-text-muted);
}
.conflicts {
  flex-shrink: 0;
  max-height: min(32vh, 20rem);
  overflow-y: auto;
  padding: 0.5rem 0.75rem 1rem;
  border-top: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
}
.card {
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-md);
  padding: 1rem;
  margin-bottom: 1rem;
}
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
  word-break: break-all;
  color: var(--gs-text-muted);
}
.btns {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
button {
  cursor: pointer;
  padding: 0.35rem 0.75rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
}
button.primary {
  border-color: var(--gs-primary-border);
  background: var(--gs-primary-bg);
}
button:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
</style>
