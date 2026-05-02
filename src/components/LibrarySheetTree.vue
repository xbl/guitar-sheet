<script setup lang="ts">
import LibrarySheetTree from "./LibrarySheetTree.vue"
import type { LibraryTreeRow } from "../utils/libraryTree"

defineProps<{
  rows: LibraryTreeRow[]
  selectedSheetId: string | null
  /** Folder chosen for「在此创建」/ 导入目标. */
  contextFolderId: string | null
}>()

defineEmits<{
  selectSheet: [id: string]
  selectFolder: [id: string]
}>()
</script>

<template>
  <ul class="tree-list">
    <template v-for="row in rows" :key="row.kind === 'folder' ? `f-${row.id}` : `s-${row.id}`">
      <li v-if="row.kind === 'folder'" class="tree-item folder">
        <button
          type="button"
          class="folder-btn"
          :class="{ context: contextFolderId === row.id }"
          :title="'在此文件夹下新建 / 导入'"
          @click="$emit('selectFolder', row.id)"
        >
          {{ row.name }}
        </button>
        <LibrarySheetTree
          v-if="row.children.length"
          :rows="row.children"
          :selected-sheet-id="selectedSheetId"
          :context-folder-id="contextFolderId"
          @select-sheet="$emit('selectSheet', $event)"
          @select-folder="$emit('selectFolder', $event)"
        />
      </li>
      <li v-else class="tree-item sheet">
        <button
          type="button"
          class="sheet-btn"
          :class="{ active: selectedSheetId === row.id }"
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
  padding: 0 0 0 0.65rem;
  border-left: 1px solid #e4e4e4;
}
.tree-item.folder {
  margin: 0.2rem 0;
}
.tree-item.sheet {
  margin: 0.12rem 0;
}
.folder-btn {
  display: block;
  width: 100%;
  text-align: left;
  border: none;
  background: none;
  padding: 0.3rem 0.35rem;
  border-radius: 4px;
  font: inherit;
  font-weight: 600;
  font-size: 0.88rem;
  color: #333;
  cursor: pointer;
}
.folder-btn:hover {
  background: #eee;
}
.folder-btn.context {
  background: #e3f2fd;
  color: #0d47a1;
}
.sheet-btn {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.35rem;
  width: 100%;
  text-align: left;
  border: none;
  background: none;
  padding: 0.28rem 0.35rem;
  border-radius: 4px;
  font: inherit;
  font-size: 0.86rem;
  cursor: pointer;
  color: #222;
}
.sheet-btn:hover {
  background: #f5f5f5;
}
.sheet-btn.active {
  background: #e8f0fe;
  color: #143c9e;
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
  font-size: 0.72rem;
  color: #888;
  text-transform: lowercase;
}
</style>
