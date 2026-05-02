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
        <div
          class="folder-line"
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

          <span class="folder-icon-wrap" aria-hidden="true">
            <svg
              class="folder-svg"
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
            >
              <path
                d="M4 8a2 2 0 012-2h3.5l1.6 1.6a1 1 0 00.7.3H19a2 2 0 012 2v9a2 2 0 01-2 2H6a2 2 0 01-2-2V8z"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linejoin="round"
              />
            </svg>
          </span>

          <button
            type="button"
            class="folder-hit"
            draggable="true"
            :title="'拖动文件夹；或将曲谱拖到此移入'"
            @dragstart="onFolderDragStart(row, $event)"
            @click="$emit('selectFolder', row.id)"
            @dragover="onFolderDragOver(row.id, $event)"
            @drop="onFolderDrop(row.id, $event)"
          >
            <span class="folder-name">{{ row.name }}</span>
            <span class="folder-count">{{ row.sheetCount }}</span>
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
          class="sheet-hit"
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
.folder-line.is-context .folder-icon-wrap {
  color: var(--gs-tree-accent);
}
.folder-line.is-context .folder-name {
  color: var(--gs-tree-accent);
  font-weight: 650;
}
.folder-line.is-drop-target {
  outline: 2px dashed var(--gs-tree-accent-muted);
  outline-offset: 1px;
  background: color-mix(in srgb, var(--gs-tree-row-active) 85%, white);
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

.folder-icon-wrap {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--gs-text-muted);
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

.sheet-hit {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.4rem;
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
