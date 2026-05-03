<script setup lang="ts">
import LibrarySheetTree from "./LibrarySheetTree.vue"
import type { LibraryTreeRow } from "../utils/libraryTree"
import {
  clearLibraryPointerPayload,
  notifyLibraryPointerDrop,
  notifyLibraryPointerHover,
  setLibraryPointerPayload,
  type TreeDndPayload,
} from "../utils/treeDnD"

withDefaults(
  defineProps<{
    rows: LibraryTreeRow[]
    selectedSheetId: string | null
    contextFolderId: string | null
    collapsedFolders: Record<string, boolean>
    highlightDropFolderId: string | null
    isRoot?: boolean
  }>(),
  { isRoot: false },
)

const emit = defineEmits<{
  selectSheet: [id: string]
  selectFolder: [id: string]
  toggleFolderCollapse: [id: string]
  folderRename: [payload: { id: string; name: string }]
  createSubfolder: [parentId: string]
  deleteFolder: [payload: { id: string; name: string }]
}>()

const DRAG_THRESHOLD_PX = 6

/** 拖动结束后吞掉一次 click，避免误触选中 */
let suppressNextTreeClick = false

function sheetPayload(row: LibraryTreeRow & { kind: "sheet" }): TreeDndPayload {
  return { kind: "sheet", id: row.id }
}

function folderPayload(row: LibraryTreeRow & { kind: "folder" }): TreeDndPayload {
  return { kind: "folder", id: row.id }
}

function attachRowPointerDrag(payload: TreeDndPayload, e: MouseEvent) {
  if (e.button !== 0) return
  const sx = e.clientX
  const sy = e.clientY
  let moved = false

  const onMove = (ev: MouseEvent) => {
    if (!moved) {
      if (Math.hypot(ev.clientX - sx, ev.clientY - sy) > DRAG_THRESHOLD_PX) {
        moved = true
        setLibraryPointerPayload(payload)
        document.body.style.userSelect = "none"
      }
    }
    if (moved) {
      notifyLibraryPointerHover(ev.clientX, ev.clientY)
      ev.preventDefault()
    }
  }

  const onUp = (ev: MouseEvent) => {
    window.removeEventListener("mousemove", onMove)
    window.removeEventListener("mouseup", onUp)
    document.body.style.userSelect = ""
    if (moved) {
      suppressNextTreeClick = true
      void notifyLibraryPointerDrop(ev.clientX, ev.clientY)
    } else {
      clearLibraryPointerPayload()
    }
  }

  window.addEventListener("mousemove", onMove)
  window.addEventListener("mouseup", onUp)
}

function onSheetRowPointerDown(row: LibraryTreeRow & { kind: "sheet" }, e: MouseEvent) {
  attachRowPointerDrag(sheetPayload(row), e)
}

function onFolderRowPointerDown(row: LibraryTreeRow & { kind: "folder" }, e: MouseEvent) {
  attachRowPointerDrag(folderPayload(row), e)
}

function onSheetRowClick(row: LibraryTreeRow & { kind: "sheet" }, e: MouseEvent) {
  if (suppressNextTreeClick) {
    suppressNextTreeClick = false
    e.preventDefault()
    e.stopPropagation()
    return
  }
  emit("selectSheet", row.id)
}

function onFolderHitClick(row: LibraryTreeRow & { kind: "folder" }, e: MouseEvent) {
  if (suppressNextTreeClick) {
    suppressNextTreeClick = false
    e.preventDefault()
    e.stopPropagation()
    return
  }
  emit("selectFolder", row.id)
}

</script>

<template>
  <ul
    class="tree-list"
    :class="{ 'tree-list--root': isRoot }"
    role="tree"
  >
    <template v-for="row in rows" :key="row.kind === 'folder' ? `f-${row.id}` : `s-${row.id}`">
      <li
        v-if="row.kind === 'folder'"
        class="tree-node tree-node--folder"
        role="treeitem"
        :aria-expanded="row.children.length ? !collapsedFolders[row.id] : undefined"
      >
        <div
          class="folder-line"
          :data-folder-drop-id="row.id"
          :class="{
            'is-context': contextFolderId === row.id,
            'is-drop-target': highlightDropFolderId === row.id,
          }"
        >
          <button
            v-if="row.children.length"
            type="button"
            class="tree-chevron"
            draggable="false"
            :aria-label="collapsedFolders[row.id] ? '展开文件夹' : '折叠文件夹'"
            @click.stop="$emit('toggleFolderCollapse', row.id)"
          >
            <svg
              class="tree-chevron-svg"
              :class="{ open: !collapsedFolders[row.id] }"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M10 8l4 4-4 4"
                stroke="currentColor"
                stroke-width="2.2"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
          <span v-else class="tree-chevron tree-chevron--spacer" aria-hidden="true" />

          <button
            type="button"
            class="folder-icon-hit"
            draggable="false"
            title="选中为新建/导入目标"
            aria-label="选中文件夹"
            @click.stop="$emit('selectFolder', row.id)"
            @mousedown.stop
            @pointerdown.stop
          >
            <svg
              class="folder-svg"
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              aria-hidden="true"
            >
              <path
                d="M4 8a2 2 0 012-2h3.5l1.6 1.6a1 1 0 00.7.3H19a2 2 0 012 2v9a2 2 0 01-2 2H6a2 2 0 01-2-2V8z"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linejoin="round"
              />
            </svg>
          </button>

          <button
            type="button"
            class="folder-hit"
            :title="'拖动整行移动文件夹；或将曲谱拖入此行'"
            @mousedown="onFolderRowPointerDown(row, $event)"
            @click="onFolderHitClick(row, $event)"
          >
            <span class="folder-name">{{ row.name }}</span>
            <span class="folder-count">{{ row.sheetCount }}</span>
          </button>

          <div
            class="folder-actions"
            @click.stop
            @mousedown.stop
            @pointerdown.stop
          >
            <button
              type="button"
              class="icon-btn"
              draggable="false"
              title="重命名"
              aria-label="重命名文件夹"
              @click.stop="$emit('folderRename', { id: row.id, name: row.name })"
              @mousedown.stop
              @pointerdown.stop
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path
                  d="M4 20h4l9.5-9.5a2 2 0 000-2.83l-.67-.67a2 2 0 00-2.83 0L4 16v4z"
                  stroke="currentColor"
                  stroke-width="1.75"
                  stroke-linejoin="round"
                />
                <path d="M13 7l4 4" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" />
              </svg>
            </button>
            <button
              type="button"
              class="icon-btn"
              draggable="false"
              title="在此文件夹下新建子文件夹"
              aria-label="新建子文件夹"
              @click.stop="$emit('createSubfolder', row.id)"
              @mousedown.stop
              @pointerdown.stop
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path
                  d="M4 8a2 2 0 012-2h3.5l1.6 1.6a1 1 0 00.7.3H19a2 2 0 012 2v9a2 2 0 01-2 2H6a2 2 0 01-2-2V8z"
                  stroke="currentColor"
                  stroke-width="1.5"
                  stroke-linejoin="round"
                />
                <path
                  d="M12 11v6M9 14h6"
                  stroke="currentColor"
                  stroke-width="1.75"
                  stroke-linecap="round"
                />
              </svg>
            </button>
            <button
              type="button"
              class="icon-btn icon-btn--danger"
              draggable="false"
              title="删除空文件夹"
              aria-label="删除文件夹"
              @click.stop="$emit('deleteFolder', { id: row.id, name: row.name })"
              @mousedown.stop
              @pointerdown.stop
            >
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path
                  d="M4 7h16M10 11v6M14 11v6M6 7l1 14h10l1-14M9 7V5a1 1 0 011-1h4a1 1 0 011 1v2"
                  stroke="currentColor"
                  stroke-width="1.75"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </button>
          </div>
        </div>

        <LibrarySheetTree
          v-if="row.children.length && !collapsedFolders[row.id]"
          :rows="row.children"
          :selected-sheet-id="selectedSheetId"
          :context-folder-id="contextFolderId"
          :collapsed-folders="collapsedFolders"
          :highlight-drop-folder-id="highlightDropFolderId"
          @select-sheet="$emit('selectSheet', $event)"
          @select-folder="$emit('selectFolder', $event)"
          @toggle-folder-collapse="$emit('toggleFolderCollapse', $event)"
          @folder-rename="$emit('folderRename', $event)"
          @create-subfolder="$emit('createSubfolder', $event)"
          @delete-folder="$emit('deleteFolder', $event)"
        />
      </li>

      <li
        v-else
        class="tree-node tree-node--sheet"
        role="treeitem"
      >
        <button
          type="button"
          class="sheet-hit"
          :class="{ active: selectedSheetId === row.id }"
          @mousedown="onSheetRowPointerDown(row, $event)"
          @click="onSheetRowClick(row, $event)"
        >
          <span class="sheet-icon-wrap" aria-hidden="true">
            <svg
              class="sheet-svg"
              width="17"
              height="17"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M14 2H8a2 2 0 00-2 2v16a2 2 0 002 2h8a2 2 0 002-2V8l-6-6z"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linejoin="round"
              />
              <path d="M14 2v6h6" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
              <path d="M8 14h8M8 17h5" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" />
            </svg>
          </span>
          <span class="sheet-title">{{ row.title }}</span>
          <span class="sheet-kind">{{ row.sheetKind }}</span>
        </button>
      </li>
    </template>
  </ul>
</template>

<style scoped>
.tree-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.tree-list:not(.tree-list--root) {
  margin: 0.15rem 0 0.25rem;
  padding-left: 0.65rem;
  margin-left: 0.35rem;
  border-left: 1px solid color-mix(in srgb, var(--gs-border) 65%, transparent);
}
.tree-list--root > .tree-node:first-child {
  margin-top: 0;
}
.tree-node--folder {
  margin: 0.2rem 0;
}
.tree-node--sheet {
  margin: 0.08rem 0;
}

.folder-line {
  display: flex;
  align-items: center;
  gap: 0.2rem;
  min-height: 2.35rem;
  padding: 0.2rem 0.35rem 0.2rem 0.25rem;
  border-radius: var(--gs-radius-md);
  transition:
    background 0.12s,
    box-shadow 0.12s;
}
.folder-line.is-context {
  background: var(--gs-tree-row-active);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--gs-tree-accent) 28%, transparent);
}
.folder-line.is-context .folder-icon-hit {
  color: var(--gs-tree-accent);
}
.folder-line.is-context .folder-name {
  color: var(--gs-tree-accent);
  font-weight: 650;
}
.folder-line.is-drop-target {
  z-index: 1;
  outline: 2px solid var(--gs-tree-accent);
  outline-offset: 1px;
  background: color-mix(in srgb, var(--gs-tree-accent) 14%, var(--gs-bg-surface));
  box-shadow:
    0 0 0 2px color-mix(in srgb, var(--gs-tree-accent) 24%, transparent),
    inset 0 0 0 1px color-mix(in srgb, var(--gs-tree-accent) 22%, transparent);
  animation: gs-tree-folder-drop-pulse 1.1s ease-in-out infinite;
}
@media (prefers-reduced-motion: reduce) {
  .folder-line.is-drop-target {
    animation: none;
  }
}
@keyframes gs-tree-folder-drop-pulse {
  0%,
  100% {
    box-shadow:
      0 0 0 2px color-mix(in srgb, var(--gs-tree-accent) 24%, transparent),
      inset 0 0 0 1px color-mix(in srgb, var(--gs-tree-accent) 22%, transparent);
  }
  50% {
    box-shadow:
      0 0 0 4px color-mix(in srgb, var(--gs-tree-accent) 34%, transparent),
      inset 0 0 0 1px color-mix(in srgb, var(--gs-tree-accent) 32%, transparent);
  }
}

.tree-chevron {
  flex-shrink: 0;
  width: 1.5rem;
  height: 1.5rem;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--gs-radius-sm);
  background: transparent;
  color: var(--gs-tree-chevron);
  cursor: pointer;
}
.tree-chevron:hover {
  background: color-mix(in srgb, var(--gs-border) 35%, transparent);
  color: var(--gs-text-muted);
}
.tree-chevron-svg {
  display: block;
  transition: transform 0.18s ease;
  transform: rotate(0deg);
}
.tree-chevron-svg.open {
  transform: rotate(90deg);
}
.tree-chevron--spacer {
  width: 1.5rem;
  flex-shrink: 0;
}

.folder-icon-hit {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.15rem;
  margin: 0;
  border: none;
  border-radius: var(--gs-radius-sm);
  background: transparent;
  color: var(--gs-text-muted);
  cursor: pointer;
}
.folder-icon-hit:hover {
  background: color-mix(in srgb, var(--gs-border) 35%, transparent);
  color: var(--gs-text-muted);
}
.folder-line.is-context .folder-icon-hit:hover {
  background: color-mix(in srgb, var(--gs-tree-accent) 14%, transparent);
}
.folder-line:not(.is-context) .folder-svg {
  opacity: 0.92;
}

.folder-hit {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.35rem 0.45rem;
  margin: 0;
  border: none;
  border-radius: var(--gs-radius-sm);
  background: transparent;
  font: inherit;
  font-size: 0.875rem;
  color: var(--gs-text);
  cursor: grab;
  text-align: left;
}
.folder-hit:hover {
  background: color-mix(in srgb, var(--gs-bg-muted) 90%, white);
}
.folder-hit:active {
  cursor: grabbing;
}

.folder-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  letter-spacing: 0.01em;
}

.folder-count {
  flex-shrink: 0;
  font-size: 0.72rem;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  color: var(--gs-text-muted);
}
.folder-line.is-context .folder-count {
  color: color-mix(in srgb, var(--gs-tree-accent) 55%, var(--gs-text-muted));
}

.folder-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 0.05rem;
}
.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.65rem;
  height: 1.65rem;
  padding: 0;
  border: none;
  border-radius: var(--gs-radius-sm);
  background: transparent;
  color: var(--gs-text-muted);
  cursor: pointer;
}
.icon-btn:hover {
  background: color-mix(in srgb, var(--gs-border) 45%, transparent);
  color: var(--gs-text);
}
.icon-btn--danger:hover {
  background: color-mix(in srgb, var(--gs-danger) 12%, transparent);
  color: var(--gs-danger);
}

.sheet-hit {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  width: 100%;
  padding: 0.4rem 0.5rem 0.4rem 0.35rem;
  border: none;
  border-radius: var(--gs-radius-sm);
  background: transparent;
  font: inherit;
  font-size: 0.83rem;
  color: var(--gs-text);
  cursor: grab;
  text-align: left;
}
.sheet-hit:active {
  cursor: grabbing;
}
.sheet-hit:hover {
  background: var(--gs-bg-muted);
}
.sheet-hit.active {
  background: var(--gs-primary-bg);
  box-shadow: inset 0 0 0 1px var(--gs-primary-border);
  color: var(--gs-link);
  font-weight: 600;
}
.sheet-icon-wrap {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  color: var(--gs-text-muted);
  opacity: 0.9;
}
.sheet-hit.active .sheet-icon-wrap {
  color: var(--gs-link);
  opacity: 1;
}
.sheet-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sheet-kind {
  flex-shrink: 0;
  font-size: 0.68rem;
  font-weight: 500;
  color: var(--gs-text-muted);
  text-transform: lowercase;
  opacity: 0.85;
}
</style>
