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

const sheets = ref<SheetMeta[]>([]);
const error = ref<string | null>(null);

async function refresh() {
  error.value = null;
  try {
    sheets.value = await invoke<SheetMeta[]>("list_sheets", { query: null });
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
    <p class="muted">谱库（后端已接通，导入 UI 在后续任务）</p>
    <p v-if="error" class="err">{{ error }}</p>
    <p v-else-if="sheets.length === 0">暂无谱子。将用「导入」按钮从本机添加。</p>
    <ul v-else class="list">
      <li v-for="s in sheets" :key="s.id">
        <strong>{{ s.display_title }}</strong>
        <span class="muted"> · {{ s.kind }}</span>
      </li>
    </ul>
    <button type="button" class="btn" @click="refresh">刷新列表</button>
  </main>
</template>

<style scoped>
.muted {
  color: #666;
  font-size: 0.95rem;
}
.err {
  color: #b00020;
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
  margin-top: 1rem;
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
