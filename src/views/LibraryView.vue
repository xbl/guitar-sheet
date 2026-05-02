<script setup lang="ts">
import { onMounted, ref } from "vue";
import { RouterLink } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ConflictEntry, SheetMeta, SyncOutcome } from "../types/sheet";

const sheets = ref<SheetMeta[]>([]);
const error = ref<string | null>(null);
const syncMsg = ref<string | null>(null);
const conflicts = ref<ConflictEntry[]>([]);

async function refresh() {
  error.value = null;
  try {
    sheets.value = await invoke<SheetMeta[]>("list_sheets", {
      query: null,
      folderId: null,
      tagSubstring: null,
    });
  } catch (e) {
    error.value = String(e);
  }
}

async function pickImport() {
  error.value = null;
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Tab / PDF / Image",
          extensions: ["txt", "md", "pdf", "png", "jpg", "jpeg", "webp"],
        },
      ],
    });
    if (selected === null) return;
    const path = typeof selected === "string" ? selected : selected[0];
    await invoke("import_sheet", {
      sourcePath: path,
      maybeTitle: null,
      folderId: null,
    });
    await refresh();
    syncMsg.value = "已导入";
  } catch (e) {
    error.value = String(e);
  }
}

async function remove(id: string, title: string) {
  if (!confirm(`删除「${title}」？本地文件会一并删除。`)) return;
  error.value = null;
  try {
    await invoke("delete_sheet", { id });
    await refresh();
  } catch (e) {
    error.value = String(e);
  }
}

async function syncGitHub() {
  syncMsg.value = null;
  conflicts.value = [];
  error.value = null;
  try {
    const out = await invoke<SyncOutcome>("sync_pull_push");
    conflicts.value = out.conflicts;
    if (out.ok) {
      syncMsg.value = `同步完成：拉取 ${out.pulled}，推送 ${out.pushed}`;
      await refresh();
    } else {
      syncMsg.value = `存在 ${out.conflicts.length} 个冲突，请在下方选择处理方式后再点「同步」。`;
    }
  } catch (e) {
    error.value = String(e);
  }
}

async function resolveConflict(
  id: string,
  choice: "keep_local" | "take_remote" | "save_copy",
) {
  error.value = null;
  try {
    await invoke("resolve_sheet_conflict", { sheetId: id, choice });
    syncMsg.value = "冲突已处理";
    conflicts.value = conflicts.value.filter((c) => c.sheet_id !== id);
    await refresh();
  } catch (e) {
    error.value = String(e);
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="page">
    <header class="toolbar">
      <h1>谱库</h1>
      <div class="actions">
        <button type="button" @click="pickImport">导入谱子</button>
        <button type="button" class="primary" @click="syncGitHub">与 GitHub 同步</button>
        <button type="button" @click="refresh">刷新</button>
        <RouterLink class="link" to="/settings">设置</RouterLink>
      </div>
    </header>

    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="syncMsg" class="ok">{{ syncMsg }}</p>

    <p v-if="!error && sheets.length === 0" class="muted">暂无谱子，点击「导入谱子」从本机选择文件。</p>
    <ul v-else class="list">
      <li v-for="s in sheets" :key="s.id">
        <RouterLink class="title" :to="`/sheet/${s.id}`">{{ s.display_title }}</RouterLink>
        <span class="meta">{{ s.kind }}</span>
        <div class="row-actions">
          <RouterLink :to="`/sheet/${s.id}`">打开</RouterLink>
          <button type="button" class="linkish" @click="remove(s.id, s.display_title)">删除</button>
        </div>
      </li>
    </ul>

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
  </div>
</template>

<style scoped>
.page {
  max-width: 48rem;
  margin: 0 auto;
  padding: 1rem 1.25rem 2rem;
}
.toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1rem;
}
.toolbar h1 {
  margin: 0;
  font-size: 1.35rem;
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
}
.actions button.primary {
  font-weight: 600;
}
.link,
.actions a {
  color: #2563eb;
  text-decoration: none;
  padding: 0.35rem 0.5rem;
}
.err {
  color: #b00020;
}
.ok {
  color: #0a5;
}
.muted {
  color: #666;
}
.list {
  list-style: none;
  padding: 0;
  margin: 0;
}
.list li {
  border-bottom: 1px solid #e5e5e5;
  padding: 0.75rem 0;
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.35rem 1rem;
  align-items: start;
}
.title {
  font-weight: 600;
  color: #111;
  text-decoration: none;
}
.title:hover {
  text-decoration: underline;
}
.meta {
  color: #666;
  font-size: 0.85rem;
}
.row-actions {
  grid-column: 1 / -1;
  display: flex;
  gap: 1rem;
}
.linkish {
  background: none;
  border: none;
  color: #b00020;
  cursor: pointer;
  padding: 0;
  font: inherit;
  text-decoration: underline;
}
.conflicts {
  margin-top: 2rem;
}
.card {
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1rem;
}
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.75rem;
  word-break: break-all;
  color: #444;
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
  border-radius: 6px;
  border: 1px solid #ccc;
  background: #fafafa;
}
button.primary {
  border-color: #2563eb;
  background: #eff6ff;
}
</style>
