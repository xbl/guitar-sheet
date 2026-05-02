<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface GitHubSettings {
  owner: string;
  repo: string;
  branch: string;
  prefix: string;
}

const settings = ref<GitHubSettings>({
  owner: "",
  repo: "",
  branch: "main",
  prefix: "",
});
const pat = ref("");
const patConfigured = ref(false);
const msg = ref<string | null>(null);
const err = ref<string | null>(null);

async function load() {
  err.value = null;
  try {
    settings.value = await invoke<GitHubSettings>("get_github_settings");
    patConfigured.value = await invoke<boolean>("github_pat_configured");
  } catch (e) {
    err.value = String(e);
  }
}

async function saveSettings() {
  err.value = null;
  msg.value = null;
  try {
    await invoke("set_github_settings", { value: settings.value });
    msg.value = "仓库设置已保存";
  } catch (e) {
    err.value = String(e);
  }
}

async function savePat() {
  err.value = null;
  msg.value = null;
  if (!pat.value.trim()) {
    err.value = "请输入 Token";
    return;
  }
  try {
    await invoke("set_github_pat", { token: pat.value.trim() });
    pat.value = "";
    patConfigured.value = true;
    msg.value = "Token 已保存到系统钥匙串";
  } catch (e) {
    err.value = String(e);
  }
}

async function clearPat() {
  err.value = null;
  msg.value = null;
  try {
    await invoke("clear_github_pat");
    patConfigured.value = false;
    msg.value = "已清除 Token";
  } catch (e) {
    err.value = String(e);
  }
}

async function testConn() {
  err.value = null;
  msg.value = null;
  try {
    const login = await invoke<string>("test_github_connection");
    msg.value = `连接成功，用户：${login}`;
  } catch (e) {
    err.value = String(e);
  }
}

onMounted(() => {
  void load();
});
</script>

<template>
  <div class="page">
    <h1 class="page-title">设置</h1>

    <p v-if="err" class="err">{{ err }}</p>
    <p v-if="msg" class="ok">{{ msg }}</p>

    <section class="card">
      <h2>GitHub 仓库</h2>
      <label>Owner <input v-model="settings.owner" type="text" autocomplete="off" /></label>
      <label>Repo <input v-model="settings.repo" type="text" autocomplete="off" /></label>
      <label>分支 <input v-model="settings.branch" type="text" /></label>
      <label
        >远端子目录前缀（可选，留空则默认 <code>sheets/</code>）
        <input v-model="settings.prefix" type="text" placeholder="例如 guitar-sheet/"
      /></label>
      <button type="button" class="primary" @click="saveSettings">保存仓库设置</button>
    </section>

    <section class="card">
      <h2>Personal Access Token</h2>
      <p class="hint">
        使用最小权限（私有库常用 <code>repo</code>）。Token 保存在本机钥匙串，不会写进谱子文件。
      </p>
      <p>当前状态：<strong>{{ patConfigured ? "已配置" : "未配置" }}</strong></p>
      <label class="block">
        <span>新 Token（保存后会清空输入框）</span>
        <input v-model="pat" type="password" autocomplete="off" />
      </label>
      <div class="row">
        <button type="button" class="primary" @click="savePat">保存 Token</button>
        <button type="button" @click="clearPat">清除 Token</button>
        <button type="button" @click="testConn">测试连接</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.page {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  max-width: 36rem;
  margin: 0 auto;
  width: 100%;
  padding: 1rem 1.25rem 2rem;
}
.page-title {
  margin: 0 0 1rem;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--gs-text);
}
.card {
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-md);
  padding: 1rem 1.25rem;
  margin-bottom: 1.25rem;
  background: var(--gs-bg-surface);
}
.card h2 {
  margin: 0 0 0.75rem;
  font-size: 1rem;
}
label {
  display: block;
  margin-bottom: 0.65rem;
  font-size: 0.9rem;
}
label.block span {
  display: block;
  margin-bottom: 0.25rem;
}
input[type="text"],
input[type="password"] {
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  margin-top: 0.2rem;
  padding: 0.4rem 0.5rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
}
.row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
button {
  cursor: pointer;
  padding: 0.4rem 0.75rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
}
button.primary {
  border-color: var(--gs-primary-border);
  background: var(--gs-primary-bg);
  font-weight: 600;
}
.err {
  color: var(--gs-danger);
}
.ok {
  color: var(--gs-success);
}
.hint {
  font-size: 0.85rem;
  color: var(--gs-text-muted);
}
code {
  font-size: 0.85em;
}
</style>
