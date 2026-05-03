<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  reactive,
  ref,
} from "vue"
import { RouterLink } from "vue-router"
import { invoke } from "@tauri-apps/api/core"
import { open } from "@tauri-apps/plugin-dialog"
import LibrarySheetTree from "../components/LibrarySheetTree.vue"
import SheetReaderPanel from "../components/SheetReaderPanel.vue"
import type { FolderNode } from "../types/folder"
import { buildLibraryTree, type LibraryTreeRow } from "../utils/libraryTree"
import type { ConflictEntry, SheetMeta, SyncOutcome } from "../types/sheet"
import type { TreeDndPayload } from "../utils/treeDnD"
import {
  clearLibraryPointerPayload,
  libraryPointerDragPayload,
  registerLibraryPointerUi,
  takeLibraryPointerPayload,
} from "../utils/treeDnD"
import { confirmTwice } from "../utils/confirmTwice"
import { showToast } from "../utils/toast"

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

/** 阅读区上报：当前文本谱是否为「未命名 + 空正文」可自动删除 */
const readerEmptyDraftId = ref<string | null>(null)

function onEmptyDraftChange(id: string | null) {
  readerEmptyDraftId.value = id
}

async function abandonIfLeavingDraft(prevSheetId: string) {
  if (readerEmptyDraftId.value !== prevSheetId) return
  try {
    await invoke("delete_sheet", { id: prevSheetId })
    readerEmptyDraftId.value = null
    selectedSheetId.value = null
    await refreshList()
    readerReloadNonce.value++
  } catch (e) {
    showToast(String(e))
  }
}

async function onSelectSheet(id: string) {
  const prev = selectedSheetId.value
  if (prev && prev !== id) {
    await abandonIfLeavingDraft(prev)
  }
  selectedSheetId.value = id
}

async function onSelectFolder(fid: string) {
  const prev = selectedSheetId.value
  if (prev) {
    await abandonIfLeavingDraft(prev)
  }
  contextFolderId.value = fid
}

const createFolderParentId = computed(() => contextFolderId.value)

const importTargetFolderId = computed(() => contextFolderId.value)

/** 拖放高亮：文件夹 id */
const highlightDropFolderId = ref<string | null>(null)
const highlightDropRoot = ref(false)
/** 指针拖曳谱库行时，用于侧栏整体提示 */
const libraryDndActive = computed(() => libraryPointerDragPayload.value !== null)
/** 谱库拖放后强制重载阅读区（路径变化） */
const readerReloadNonce = ref(0)

const SIDEBAR_COLLAPSED_KEY = "guitar-sheet.librarySidebarCollapsed"
const sidebarCollapsed = ref(false)

function readSidebarCollapsed(): boolean {
  try {
    return localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === "1"
  } catch {
    return false
  }
}

function persistSidebarCollapsed() {
  try {
    localStorage.setItem(SIDEBAR_COLLAPSED_KEY, sidebarCollapsed.value ? "1" : "0")
  } catch {
    /* private mode / unavailable */
  }
}

function toggleSidebarCollapsed() {
  sidebarCollapsed.value = !sidebarCollapsed.value
  persistSidebarCollapsed()
}

/** 指针拖动悬停：与 HTML5 dragover 无关，仅靠 mousemove 坐标 */
function libraryPointerHover(clientX: number, clientY: number) {
  const el = document.elementFromPoint(clientX, clientY)
  if (!el) return
  const folderHost = el.closest("[data-folder-drop-id]")
  if (folderHost instanceof HTMLElement) {
    const fid = folderHost.getAttribute("data-folder-drop-id")
    if (fid) {
      highlightDropFolderId.value = fid
      highlightDropRoot.value = false
      return
    }
  }
  const rootHost = el.closest(".tree-drop-root")
  if (rootHost) {
    highlightDropFolderId.value = null
    highlightDropRoot.value = true
    return
  }
  if (el.closest(".tree-scroll")) {
    highlightDropFolderId.value = null
    highlightDropRoot.value = false
  }
}

async function libraryPointerDrop(clientX: number, clientY: number) {
  const el = document.elementFromPoint(clientX, clientY)
  if (!el) {
    takeLibraryPointerPayload()
    return
  }
  const folderHost = el.closest("[data-folder-drop-id]")
  if (folderHost instanceof HTMLElement) {
    const folderId = folderHost.getAttribute("data-folder-drop-id")
    if (folderId) {
      clearDropHighlight()
      const dragPayload = takeLibraryPointerPayload()
      if (!dragPayload) return
      if (dragPayload.kind === "folder" && dragPayload.id === folderId) return
      await onTreeMoveDrop({ target: { kind: "folder", folderId }, drag: dragPayload })
      return
    }
  }
  if (el.closest(".tree-drop-root")) {
    clearDropHighlight()
    const taken = takeLibraryPointerPayload()
    if (taken) await moveSheetOrFolderToRoot(taken)
    return
  }
  takeLibraryPointerPayload()
}

async function moveSheetOrFolderToRoot(drag: TreeDndPayload) {
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

function clearDropHighlight() {
  highlightDropFolderId.value = null
  highlightDropRoot.value = false
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

const folderLinePrompt = reactive({
  open: false,
  title: "",
  value: "",
})
const folderLinePromptBackdropRef = ref<HTMLElement | null>(null)
let folderLinePromptResolve: ((v: string | null) => void) | null = null

function openFolderLinePrompt(title: string, initial: string): Promise<string | null> {
  return new Promise((resolve) => {
    folderLinePromptResolve = resolve
    folderLinePrompt.title = title
    folderLinePrompt.value = initial
    folderLinePrompt.open = true
    void nextTick(() => {
      folderLinePromptBackdropRef.value?.focus()
      const inp = folderLinePromptBackdropRef.value?.querySelector(
        "input.lib-line-prompt-input",
      ) as HTMLInputElement | null
      inp?.focus()
      inp?.select()
    })
  })
}

function submitFolderLinePrompt() {
  const r = folderLinePromptResolve
  folderLinePromptResolve = null
  folderLinePrompt.open = false
  r?.(folderLinePrompt.value)
}

function cancelFolderLinePrompt() {
  const r = folderLinePromptResolve
  folderLinePromptResolve = null
  folderLinePrompt.open = false
  r?.(null)
}

async function onDeleteFolder(payload: { id: string; name: string }) {
  if (
    !(await confirmTwice(
      `删除文件夹「${payload.name}」？仅允许删除空文件夹（无子文件夹、无曲谱）。`,
      "再次确认：删除后不可恢复，确定删除吗？",
    ))
  ) {
    return
  }
  error.value = null
  try {
    await invoke("delete_folder", { folderId: payload.id })
    if (contextFolderId.value === payload.id) {
      contextFolderId.value = null
    }
    await refresh()
    readerReloadNonce.value++
  } catch (e) {
    showToast(String(e))
  }
}

async function onFolderRename(payload: { id: string; name: string }) {
  const next = await openFolderLinePrompt("文件夹名称", payload.name)
  if (next === null) return
  const t = next.trim()
  if (!t || t === payload.name) return
  error.value = null
  try {
    await invoke("rename_folder", { folderId: payload.id, newName: t })
    await refresh()
    readerReloadNonce.value++
  } catch (e) {
    error.value = String(e)
  }
}

async function onCreateSubfolder(rowFolderId: string) {
  /** 「当前文件夹」= 侧栏选中的上下文文件夹；未选中时在该行文件夹下创建 */
  const parentId = contextFolderId.value ?? rowFolderId
  const next = await openFolderLinePrompt("新建子文件夹名称", "")
  if (next === null) return
  const t = next.trim()
  if (!t) return
  error.value = null
  try {
    await invoke("create_folder", { parentId, name: t })
    await refresh()
  } catch (e) {
    error.value = String(e)
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
    selectedSheetId.value = meta.id
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

async function onDeleteSheet(payload: { id: string; title: string }) {
  if (
    !(await confirmTwice(
      `删除「${payload.title}」？本地文件会一并删除。`,
      "再次确认：删除后不可恢复，确定删除吗？",
    ))
  )
    return
  error.value = null
  try {
    await invoke("delete_sheet", { id: payload.id })
    if (selectedSheetId.value === payload.id) {
      selectedSheetId.value = null
    }
    readerEmptyDraftId.value = null
    await refresh()
    readerReloadNonce.value++
  } catch (e) {
    showToast(String(e))
  }
}

function toggleFolderCollapse(id: string) {
  const cur = collapsedFolders.value[id] ?? false
  collapsedFolders.value = {
    ...collapsedFolders.value,
    [id]: !cur,
  }
}

function onLibraryPointerCleanup() {
  clearDropHighlight()
  clearLibraryPointerPayload()
}

onMounted(() => {
  sidebarCollapsed.value = readSidebarCollapsed()
  void refresh()
  registerLibraryPointerUi(libraryPointerHover, libraryPointerDrop)
  document.addEventListener("dragend", onLibraryPointerCleanup)
})

onUnmounted(() => {
  registerLibraryPointerUi(null, null)
  document.removeEventListener("dragend", onLibraryPointerCleanup)
})
</script>

<template>
  <div class="layout" :class="{ 'layout--lib-dnd': libraryDndActive }">
    <aside class="sidebar" :class="{ 'is-collapsed': sidebarCollapsed }">
      <div class="sidebar-top">
        <nav class="side-nav" aria-label="主导航">
          <RouterLink
            to="/"
            class="side-nav-link"
            active-class="is-active"
            title="谱库"
          >
            <svg class="side-nav-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                fill="currentColor"
                d="M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z"
              />
            </svg>
            <span>谱库</span>
          </RouterLink>
          <RouterLink to="/settings" class="side-nav-link" title="设置">
            <svg class="side-nav-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                fill="currentColor"
                d="M19.14 12.94c.04-.31.06-.63.06-.94 0-.31-.02-.63-.06-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.488.488 0 0 0-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54a.484.484 0 0 0-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.04.31-.06.63-.06.94s.02.63.06.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"
              />
            </svg>
            <span>设置</span>
          </RouterLink>
        </nav>
        <div class="side-divider" role="presentation" />
        <div class="side-actions" aria-label="谱库操作">
          <button
            type="button"
            class="side-action"
            :disabled="creatingSheet"
            :title="creatingSheet ? '创建中…' : '新建曲谱'"
            @click="createNewSheet"
          >
            <svg class="side-action-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                fill="currentColor"
                d="M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.89 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm2 14h-3v3h-2v-3H8v-2h3v-3h2v3h3v2zm-3-7V3.5L18.5 9H13z"
              />
            </svg>
            <span>{{ creatingSheet ? "创建中…" : "新建曲谱" }}</span>
          </button>
          <button type="button" class="side-action" title="导入谱子" @click="pickImport">
            <svg class="side-action-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                fill="currentColor"
                d="M9 16h6v-6h4l-7-7-7 7h4zm-4 2h14v2H5z"
              />
            </svg>
            <span>导入谱子</span>
          </button>
          <button
            type="button"
            class="side-action side-action-primary"
            title="与 GitHub 同步"
            @click="syncGitHub"
          >
            <svg class="side-action-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                fill="currentColor"
                d="M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46C19.54 15.03 20 13.57 20 12c0-4.42-3.58-8-8-8zm0 14c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8L5.24 7.74C4.46 8.97 4 10.43 4 12c0 4.42 3.58 8 8 8v3l4-4-4-4v3z"
              />
            </svg>
            <span>与 GitHub 同步</span>
          </button>
          <button type="button" class="side-action" title="刷新列表" @click="refresh">
            <svg class="side-action-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path
                fill="currentColor"
                d="M17.65 6.35A7.958 7.958 0 0 0 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08A5.99 5.99 0 0 1 12 18c-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"
              />
            </svg>
            <span>刷新</span>
          </button>
        </div>
      </div>

      <div v-show="!sidebarCollapsed" class="sidebar-body">
        <div class="side-divider" role="presentation" />
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
      <div
        class="tree-drop-root"
        :class="{ 'is-target': highlightDropRoot }"
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
            @select-sheet="onSelectSheet"
            @select-folder="onSelectFolder"
            @toggle-folder-collapse="toggleFolderCollapse"
            @folder-rename="onFolderRename"
            @create-subfolder="onCreateSubfolder"
            @delete-folder="onDeleteFolder"
            @delete-sheet="onDeleteSheet"
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
        <button type="button" class="primary" @click="createFolder">创建</button>
      </div>
      </div>

      <div class="sidebar-footer">
        <button
          type="button"
          class="sidebar-collapse-toggle"
          :title="sidebarCollapsed ? '展开侧栏' : '收起侧栏'"
          :aria-label="sidebarCollapsed ? '展开侧栏' : '收起侧栏'"
          :aria-expanded="!sidebarCollapsed"
          @click="toggleSidebarCollapsed"
        >
          <svg
            v-if="sidebarCollapsed"
            class="sidebar-collapse-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              fill="currentColor"
              d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z"
            />
          </svg>
          <svg
            v-else
            class="sidebar-collapse-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path
              fill="currentColor"
              d="M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z"
            />
          </svg>
          <span class="sidebar-collapse-label">{{
            sidebarCollapsed ? "展开" : "收起"
          }}</span>
        </button>
      </div>
    </aside>

    <main class="main">
      <p v-if="error" class="err">{{ error }}</p>
      <p v-if="syncMsg" class="ok">{{ syncMsg }}</p>

      <div class="reader-host">
        <SheetReaderPanel
          :sheet-id="selectedSheetId"
          :reload-nonce="readerReloadNonce"
          variant="embed"
          @deleted="onSheetDeleted"
          @empty-draft-change="onEmptyDraftChange"
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

    <Teleport to="body">
      <div
        v-if="folderLinePrompt.open"
        ref="folderLinePromptBackdropRef"
        class="lib-line-prompt-backdrop"
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        @click.self="cancelFolderLinePrompt"
        @keydown.escape.prevent="cancelFolderLinePrompt"
      >
        <div class="lib-line-prompt" @click.stop>
          <p class="lib-line-prompt-title">{{ folderLinePrompt.title }}</p>
          <input
            v-model="folderLinePrompt.value"
            type="text"
            class="lib-line-prompt-input"
            maxlength="80"
            @keydown.enter.prevent="submitFolderLinePrompt"
          />
          <div class="lib-line-prompt-actions">
            <button
              type="button"
              class="lib-line-prompt-btn lib-line-prompt-cancel"
              @click="cancelFolderLinePrompt"
            >
              取消
            </button>
            <button
              type="button"
              class="lib-line-prompt-btn lib-line-prompt-primary"
              @click="submitFolderLinePrompt"
            >
              确定
            </button>
          </div>
        </div>
      </div>
    </Teleport>
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
.layout--lib-dnd {
  cursor: grabbing;
}
.sidebar {
  width: min(20rem, 42vw);
  min-width: min(20rem, 42vw);
  flex-shrink: 0;
  padding: 0.65rem 0.55rem 0.5rem;
  border-right: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  transition:
    width 0.2s ease,
    min-width 0.2s ease,
    padding 0.2s ease;
}
.sidebar.is-collapsed {
  width: 3.35rem;
  min-width: 3.35rem;
  padding-left: 0.35rem;
  padding-right: 0.35rem;
}
.sidebar-top {
  flex-shrink: 0;
}
.sidebar-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.sidebar-footer {
  flex-shrink: 0;
  margin-top: auto;
  padding-top: 0.4rem;
  border-top: 1px solid var(--gs-border);
}
.sidebar-collapse-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  width: 100%;
  box-sizing: border-box;
  padding: 0.4rem 0.45rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: var(--gs-text-muted);
  font-size: 0.8rem;
  cursor: pointer;
  transition:
    color 0.12s ease,
    border-color 0.12s ease,
    background 0.12s ease;
}
.sidebar-collapse-toggle:hover {
  color: var(--gs-text);
  border-color: var(--gs-text-muted);
  background: var(--gs-bg-muted);
}
.sidebar-collapse-icon {
  width: 1.1rem;
  height: 1.1rem;
  flex-shrink: 0;
}
.sidebar.is-collapsed .sidebar-collapse-label {
  display: none;
}
.sidebar.is-collapsed .sidebar-collapse-toggle {
  padding: 0.45rem 0.3rem;
}
.sidebar.is-collapsed .side-nav-link,
.sidebar.is-collapsed .side-action {
  justify-content: center;
  gap: 0;
  padding-left: 0.35rem;
  padding-right: 0.35rem;
}
.sidebar.is-collapsed .side-nav-link span,
.sidebar.is-collapsed .side-action span {
  display: none;
}
.side-nav {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}
.side-nav-link {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.42rem 0.5rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid transparent;
  font-size: 0.88rem;
  font-weight: 500;
  color: var(--gs-text-muted);
  text-decoration: none;
  transition:
    background 0.12s ease,
    color 0.12s ease;
}
.side-nav-link:hover {
  color: var(--gs-text);
  background: var(--gs-bg-surface);
}
.side-nav-link.is-active {
  color: var(--gs-link);
  background: var(--gs-primary-bg);
  border: 1px solid color-mix(in srgb, var(--gs-primary-border) 35%, transparent);
}
.side-nav-icon {
  width: 1.15rem;
  height: 1.15rem;
  flex-shrink: 0;
}
.side-divider {
  height: 1px;
  margin: 0.55rem 0;
  background: var(--gs-border);
  flex-shrink: 0;
}
.side-actions {
  display: flex;
  flex-direction: column;
  gap: 0.28rem;
  flex-shrink: 0;
}
.side-action {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  width: 100%;
  box-sizing: border-box;
  padding: 0.4rem 0.5rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: var(--gs-text);
  font-size: 0.85rem;
  text-align: left;
  cursor: pointer;
  transition:
    border-color 0.12s ease,
    background 0.12s ease;
}
.side-action:hover {
  border-color: var(--gs-text-muted);
  background: var(--gs-bg-muted);
}
.side-action:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}
.side-action-primary {
  font-weight: 600;
  border-color: var(--gs-primary-border);
  background: var(--gs-primary-bg);
  color: var(--gs-link);
}
.side-action-primary:hover {
  border-color: var(--gs-link);
  background: color-mix(in srgb, var(--gs-primary-bg) 85%, var(--gs-bg-surface));
}
.side-action-icon {
  width: 1.05rem;
  height: 1.05rem;
  flex-shrink: 0;
  opacity: 0.9;
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
.layout--lib-dnd .tree-scroll {
  box-shadow:
    inset 0 0 0 2px var(--gs-drop-zone-active-shadow),
    0 0 0 1px color-mix(in srgb, var(--gs-border) 55%, transparent);
  background: color-mix(in srgb, var(--gs-drop-zone-active-bg) 55%, var(--gs-bg-surface));
}
.layout--lib-dnd .tree-drop-root:not(.is-target) {
  border-color: color-mix(in srgb, var(--gs-primary-border) 45%, var(--gs-border));
  background: color-mix(in srgb, var(--gs-primary-bg) 35%, var(--gs-bg-surface));
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
  box-shadow:
    0 0 0 2px color-mix(in srgb, var(--gs-primary-border) 35%, transparent),
    inset 0 0 0 1px color-mix(in srgb, var(--gs-primary-border) 22%, transparent);
  animation: gs-tree-root-drop-pulse 1.1s ease-in-out infinite;
}
@media (prefers-reduced-motion: reduce) {
  .tree-drop-root.is-target {
    animation: none;
  }
}
@keyframes gs-tree-root-drop-pulse {
  0%,
  100% {
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--gs-primary-border) 35%, transparent),
      inset 0 0 0 1px color-mix(in srgb, var(--gs-primary-border) 22%, transparent);
  }
  50% {
    box-shadow:
      0 0 0 4px color-mix(in srgb, var(--gs-primary-border) 45%, transparent),
      inset 0 0 0 1px color-mix(in srgb, var(--gs-primary-border) 30%, transparent);
  }
}
.tree-scroll {
  flex: 1;
  overflow: auto;
  min-height: 6rem;
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
  box-sizing: border-box;
  padding: 0.4rem 0.5rem;
  font-size: 0.85rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
  color: inherit;
}
.new-folder button.primary {
  flex-shrink: 0;
  font-size: 0.85rem;
}
.main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--gs-bg-app);
}
.reader-host {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.err {
  color: var(--gs-danger);
  padding: 0.5rem 0.75rem 0;
  flex-shrink: 0;
}
.ok {
  color: var(--gs-success);
  padding: 0 0.75rem 0.35rem;
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

<style>
/* Teleport to body — keep unscoped so dialog styles apply */
.lib-line-prompt-backdrop {
  position: fixed;
  inset: 0;
  z-index: 12000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  background: color-mix(in srgb, var(--gs-text, #111) 38%, transparent);
}
.lib-line-prompt {
  width: min(22rem, 100%);
  padding: 1rem 1.1rem;
  border-radius: var(--gs-radius-md, 10px);
  border: 1px solid var(--gs-border, #ccc);
  background: var(--gs-bg-surface, #fff);
  box-shadow: var(--gs-shadow-sm, 0 4px 24px rgba(0, 0, 0, 0.12));
}
.lib-line-prompt-title {
  margin: 0 0 0.65rem;
  font-size: 0.92rem;
  font-weight: 600;
  color: var(--gs-text, #222);
}
.lib-line-prompt-input {
  width: 100%;
  box-sizing: border-box;
  padding: 0.4rem 0.5rem;
  font-size: 0.9rem;
  border: 1px solid var(--gs-border, #ccc);
  border-radius: var(--gs-radius-sm, 6px);
  background: var(--gs-bg-surface, #fff);
  color: inherit;
}
.lib-line-prompt-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.85rem;
}
.lib-line-prompt-btn {
  cursor: pointer;
  padding: 0.4rem 0.75rem;
  border-radius: var(--gs-radius-sm, 6px);
  border: 1px solid var(--gs-border, #ccc);
  font-size: 0.9rem;
}
.lib-line-prompt-cancel {
  background: var(--gs-bg-muted, #f0f0f0);
  color: var(--gs-text, #222);
}
.lib-line-prompt-primary {
  border-color: var(--gs-primary-border, #6b8cff);
  background: var(--gs-primary-bg, #eef3ff);
  color: var(--gs-text, #222);
  font-weight: 600;
}
</style>
