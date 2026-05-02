<script setup lang="ts">
import LibrarySheetTree from "./LibrarySheetTree.vue"
import type { LibraryTreeRow } from "../utils/libraryTree"
import {
  readTreeDragPayload,
  setTreeDragPayload,
  type TreeDndPayload,
} from "../utils/treeDnD"

withDefaults(
  defineProps<{
    rows: LibraryTreeRow[]
    selectedSheetId: string | null
    contextFolderId: string | null
    collapsedFolders: Record<string, boolean>
    /** Drop target highlight while dragging over a folder row */
    highlightDropFolderId: string | null
    isRoot?: boolean
  }>(),
  { isRoot: false },
)

const emit = defineEmits<{
  selectSheet: [id: string]
  selectFolder: [id: string]
  toggleFolderCollapse: [id: string]
  folderDropHover: [id: string | null]
  moveDrop: [
    payload: {
      target: { kind: "folder"; folderId: string }
      drag: TreeDndPayload
    },
  ]
}>()

function onSheetDragStart(row: LibraryTreeRow & { kind: "sheet" }, e: DragEvent) {
  setTreeDragPayload(e, { kind: "sheet", id: row.id })
}

function onFolderDragStart(row: LibraryTreeRow & { kind: "folder" }, e: DragEvent) {
  setTreeDragPayload(e, { kind: "folder", id: row.id })
}

function onFolderDragOver(folderId: string, e: DragEvent) {
  e.preventDefault()
  emit("folderDropHover", folderId)
}

function onFolderDrop(folderId: string, e: DragEvent) {
  e.preventDefault()
  const drag = readTreeDragPayload(e)
  if (!drag) return
  if (drag.kind === "folder" && drag.id === folderId) return
  emit("moveDrop", { target: { kind: "folder", folderId }, drag })
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
        <div class="folder-row">
          <button
            v-if="row.children.length"
            type="button"
            class="folder-fold"
            draggable="false"
            :aria-label="collapsedFolders[row.id] ? '展开文件夹' : '折叠文件夹'"
            @click.stop="$emit('toggleFolderCollapse', row.id)"
          >
            <span class="folder-fold-icon" aria-hidden="true">{{
              collapsedFolders[row.id] ? "▸" : "▾"
            }}</span>
          </button>
          <span v-else class="folder-fold folder-fold--spacer" aria-hidden="true" />

          <button
            type="button"
            class="folder-btn"
            :class="{
              context: contextFolderId === row.id,
              'is-drop-target': highlightDropFolderId === row.id,
            }"
            draggable="true"
            :title="'拖动文件夹；或将曲谱拖到此释放以移入'"
            @dragstart="onFolderDragStart(row, $event)"
            @click="$emit('selectFolder', row.id)"
            @dragover="onFolderDragOver(row.id, $event)"
            @drop="onFolderDrop(row.id, $event)"
          >
            <span class="folder-name">{{ row.name }}</span>
          </button>
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
          @folder-drop-hover="$emit('folderDropHover', $event)"
          @move-drop="$emit('moveDrop', $event)"
        />
      </li>

      <li
        v-else
        class="tree-node tree-node--sheet"
        role="treeitem"
      >
        <button
          type="button"
          class="sheet-btn"
          :class="{ active: selectedSheetId === row.id }"
          draggable="true"
          @dragstart="onSheetDragStart(row, $event)"
          @click="$emit('selectSheet', row.id)"
        >
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
  padding: 0 0 0 0.5rem;
  border-left: 2px solid var(--gs-border);
}
.tree-list--root {
  padding-left: 0;
  border-left: none;
}
.tree-node {
  margin: 0;
}
.tree-node--folder {
  padding: 0.15rem 0;
}
.tree-node--sheet {
  padding: 0.06rem 0;
}
.folder-row {
  display: flex;
  align-items: stretch;
  gap: 0.15rem;
  border-radius: var(--gs-radius-sm);
}
.folder-row:focus-within {
  outline: 1px solid var(--gs-primary-border);
  outline-offset: 1px;
}
.folder-fold {
  flex-shrink: 0;
  width: 1.35rem;
  padding: 0;
  border: 1px solid transparent;
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-muted);
  color: var(--gs-text-muted);
  cursor: pointer;
  font-size: 0.65rem;
  line-height: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.folder-fold:hover {
  border-color: var(--gs-border);
  color: var(--gs-text);
  background: var(--gs-bg-surface);
}
.folder-fold--spacer {
  visibility: hidden;
  pointer-events: none;
}
.folder-fold-icon {
  display: block;
  transform: translateY(0.05em);
}
.folder-btn {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  text-align: left;
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  padding: 0.35rem 0.45rem;
  border-radius: var(--gs-radius-sm);
  font: inherit;
  font-weight: 600;
  font-size: 0.86rem;
  color: var(--gs-text);
  cursor: grab;
  box-shadow: var(--gs-shadow-sm);
}
.folder-btn:active {
  cursor: grabbing;
}
.folder-btn:hover {
  border-color: var(--gs-primary-border);
  background: var(--gs-bg-muted);
}
.folder-btn.context {
  border-color: var(--gs-primary-border);
  background: var(--gs-primary-bg);
  color: var(--gs-link);
}
.folder-btn.is-drop-target {
  outline: 2px dashed var(--gs-primary-border);
  outline-offset: 1px;
  background: var(--gs-primary-bg);
}
.folder-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sheet-btn {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.35rem;
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  background: transparent;
  padding: 0.32rem 0.4rem 0.32rem 1.5rem;
  border-radius: var(--gs-radius-sm);
  font: inherit;
  font-size: 0.84rem;
  cursor: grab;
  color: var(--gs-text);
}
.sheet-btn:active {
  cursor: grabbing;
}
.sheet-btn:hover {
  background: var(--gs-bg-muted);
  border-color: var(--gs-border);
}
.sheet-btn.active {
  background: var(--gs-primary-bg);
  border-color: var(--gs-primary-border);
  color: var(--gs-link);
  font-weight: 600;
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
  font-size: 0.7rem;
  color: var(--gs-text-muted);
  text-transform: lowercase;
}
</style>
