<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface SheetMeta {
  id: string;
  display_title: string;
  kind: string;
  local_rel_path: string;
  absolute_path: string;
  local_content_hash: string;
  remote_path: string | null;
}

interface ConflictEntry {
  sheet_id: string;
  display_title: string;
  remote_path: string;
  remote_sha: string;
  local_disk_hash: string;
  last_known_remote_sha: string | null;
}

interface SyncOutcome {
  ok: boolean;
  conflicts: ConflictEntry[];
  pulled: number;
  pushed: number;
}

const sheets = ref<SheetMeta[]>([]);
const error = ref<string | null>(null);
const syncMsg = ref<string | null>(null);
const conflicts = ref<ConflictEntry[]>([]);

async function refresh() {
  error.value = null;
  try {
    sheets.value = await invoke<SheetMeta[]>("list_sheets", { query: null });
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
      syncMsg.value = `存在 ${out.conflicts.length} 个冲突，请在下方选择处理方式后再同步。`;
    }
  } catch (e) {
    error.value = String(e);
  }
}

async function resolve(id: string, choice: "keep_local" | "take_remote" | "save_copy") {
  error.value = null;
  try {
    await invoke("resolve_sheet_conflict", { sheet_id: id, choice });
    syncMsg.value = `已处理：${choice}`;
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
  <main class="container">
    <h1>Guitar Sheet</h1>
    <p class="muted">谱库 · GitHub 同步（需先在系统外配置 Token / owner / repo）</p>
    <p v-if="error" class="err">{{ error }}</p>
    <p v-if="syncMsg" class="sync">{{ syncMsg }}</p>
    <div class="row-actions">
      <button type="button" class="btn" @click="refresh">刷新列表</button>
      <button type="button" class="btn primary" @click="syncGitHub">与 GitHub 同步</button>
    </div>
    <p v-if="!error && sheets.length === 0" class="muted">暂无谱子。</p>
    <ul v-else-if="sheets.length > 0" class="list">
      <li v-for="s in sheets" :key="s.id">
        <strong>{{ s.display_title }}</strong>
        <span class="muted"> · {{ s.kind }}</span>
      </li>
    </ul>

    <section v-if="conflicts.length > 0" class="conflicts">
      <h2>冲突</h2>
      <article v-for="c in conflicts" :key="c.sheet_id" class="card">
        <h3>{{ c.display_title }}</h3>
        <p class="muted mono">{{ c.remote_path }}</p>
        <div class="btns">
          <button type="button" @click="resolve(c.sheet_id, 'keep_local')">保留本地</button>
          <button type="button" @click="resolve(c.sheet_id, 'take_remote')">使用远端</button>
          <button type="button" @click="resolve(c.sheet_id, 'save_copy')">另存为副本</button>
        </div>
      </article>
    </section>
  </main>
</template>

<style scoped>
.muted {
  color: #666;
  font-size: 0.95rem;
}
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.8rem;
  word-break: break-all;
}
.err {
  color: #b00020;
}
.sync {
  color: #0a5;
  margin-top: 0.5rem;
}
.row-actions {
  display: flex;
  gap: 0.75rem;
  justify-content: center;
  flex-wrap: wrap;
  margin-top: 1rem;
}
.list {
  text-align: left;
  max-width: 36rem;
  margin: 1rem auto;
  padding: 0;
  list-style: none;
}
.list li {
  padding: 0.5rem 0;
  border-bottom: 1px solid #eee;
}
.btn {
  padding: 0.4rem 0.9rem;
}
.btn.primary {
  font-weight: 600;
}
.conflicts {
  margin-top: 2rem;
  text-align: left;
  max-width: 40rem;
  margin-left: auto;
  margin-right: auto;
}
.card {
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1rem;
}
.btns {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
</style>

<style>
:root {
  font-family: system-ui, sans-serif;
}
.container {
  padding: 2rem;
  text-align: center;
}
</style>
