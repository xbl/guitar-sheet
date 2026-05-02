<script setup lang="ts">
import { TauriEvent } from "@tauri-apps/api/event"
import { invoke } from "@tauri-apps/api/core"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { computed, onMounted, onUnmounted, ref } from "vue"
import { RouterLink, RouterView, useRoute } from "vue-router"
import type { UiPrefs, UiPrefsPatch } from "./types/uiPrefs"

const route = useRoute()
const alwaysOnTop = ref(false)
const fullscreen = ref(false)
const pinError = ref<string | null>(null)

let unlistenFocus: (() => void) | null = null
let unlistenResume: (() => void) | null = null

const routeLabel = computed(() => {
  if (route.path === "/") return "谱库"
  if (route.path === "/settings") return "设置"
  if (route.path.startsWith("/sheet/")) return "阅读"
  return ""
})

async function applyAlwaysOnTopFromState() {
  const win = getCurrentWindow()
  try {
    await win.setAlwaysOnTop(alwaysOnTop.value)
    pinError.value = null
  } catch {
    if (alwaysOnTop.value) {
      pinError.value = "无法固定窗口（部分 Linux 桌面不支持）"
    }
  }
}

async function loadPrefs() {
  const prefs = await invoke<UiPrefs>("get_ui_prefs")
  document.documentElement.dataset.theme = prefs.themeId || "light-paper"
  alwaysOnTop.value = prefs.alwaysOnTop
  const win = getCurrentWindow()
  try {
    await win.setAlwaysOnTop(alwaysOnTop.value)
    pinError.value = null
  } catch {
    if (alwaysOnTop.value) {
      pinError.value = "无法固定窗口（部分 Linux 桌面不支持）"
    }
  }
  fullscreen.value = await win.isFullscreen()
}

async function togglePin() {
  const next = !alwaysOnTop.value
  const win = getCurrentWindow()
  try {
    await win.setAlwaysOnTop(next)
    pinError.value = null
    alwaysOnTop.value = next
    const patch: UiPrefsPatch = { alwaysOnTop: next }
    await invoke<UiPrefs>("set_ui_prefs", patch)
  } catch {
    pinError.value = "无法固定窗口（部分 Linux 桌面不支持）"
  }
}

async function toggleFullscreen() {
  const win = getCurrentWindow()
  const next = !(await win.isFullscreen())
  await win.setFullscreen(next)
  fullscreen.value = next
}

onMounted(async () => {
  document.documentElement.dataset.theme = "light-paper"
  try {
    await loadPrefs()
  } catch {
    /* e.g. open in plain browser */
  }
  const win = getCurrentWindow()
  try {
    unlistenFocus = await win.listen(TauriEvent.WINDOW_FOCUS, () => {
      void applyAlwaysOnTopFromState()
    })
    unlistenResume = await win.listen(TauriEvent.WINDOW_RESUMED, () => {
      void applyAlwaysOnTopFromState()
    })
  } catch {
    /* non-Tauri */
  }
})

onUnmounted(() => {
  unlistenFocus?.()
  unlistenResume?.()
})
</script>

<template>
  <div id="shell">
    <header class="chrome">
      <div class="chrome-left">
        <RouterLink to="/" class="brand">Guitar Sheet</RouterLink>
        <span v-if="routeLabel" class="route-label">{{ routeLabel }}</span>
      </div>
      <div class="chrome-right">
        <button
          type="button"
          class="icon-btn"
          :title="fullscreen ? '退出全屏' : '全屏'"
          :aria-pressed="fullscreen"
          @click="toggleFullscreen"
        >
          <span class="sr-only">{{ fullscreen ? "退出全屏" : "全屏" }}</span>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              fill="currentColor"
              d="M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z"
            />
          </svg>
        </button>
        <button
          type="button"
          class="icon-btn"
          :class="{ on: alwaysOnTop }"
          :aria-pressed="alwaysOnTop"
          :title="alwaysOnTop ? '取消固定在最前' : '固定在最前'"
          @click="togglePin"
        >
          <span class="sr-only">{{ alwaysOnTop ? "取消置顶" : "置顶" }}</span>
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <path
              fill="currentColor"
              d="M16 9V4h1c.55 0 1-.45 1-1s-.45-1-1-1H7c-.55 0-1 .45-1 1s.45 1 1 1h1v5c0 1.66-1.34 3-3 3v2h5.97v7l1 1 1-1v-7H19v-2c-1.66 0-3-1.34-3-3z"
            />
          </svg>
        </button>
        <RouterLink to="/settings" class="settings-link">设置</RouterLink>
      </div>
    </header>
    <p v-if="pinError" class="chrome-warn" role="status">{{ pinError }}</p>
    <div class="shell-content">
      <RouterView />
    </div>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
}
html,
body {
  height: 100%;
  margin: 0;
  overflow: hidden;
}
body {
  font-family:
    system-ui,
    -apple-system,
    sans-serif;
}
#shell {
  height: 100dvh;
  max-height: 100dvh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.shell-content {
  flex: 1;
  min-height: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.chrome {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.45rem 0.85rem;
  border-bottom: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  flex-shrink: 0;
  flex-wrap: wrap;
}
.chrome-left {
  display: flex;
  align-items: baseline;
  gap: 0.65rem;
  min-width: 0;
}
.chrome-right {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-shrink: 0;
}
.brand {
  font-weight: 700;
  color: var(--gs-text);
  text-decoration: none;
  font-size: 0.98rem;
}
.brand.router-link-active {
  color: var(--gs-text);
}
.route-label {
  font-size: 0.8rem;
  color: var(--gs-text-muted);
}
.settings-link {
  color: var(--gs-link);
  text-decoration: none;
  font-size: 0.9rem;
  padding: 0.35rem 0.5rem;
  border-radius: var(--gs-radius-sm);
}
.settings-link:hover {
  color: var(--gs-link-hover);
  background: var(--gs-bg-surface);
}
.settings-link.router-link-active {
  font-weight: 600;
}
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.25rem;
  height: 2.25rem;
  padding: 0;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
  color: var(--gs-text-muted);
  cursor: pointer;
}
.icon-btn:hover {
  color: var(--gs-text);
  border-color: var(--gs-text-muted);
}
.icon-btn.on {
  color: var(--gs-link);
  border-color: var(--gs-primary-border);
  background: var(--gs-primary-bg);
}
.icon-btn .icon {
  width: 1.15rem;
  height: 1.15rem;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
.chrome-warn {
  margin: 0;
  padding: 0.35rem 0.85rem;
  font-size: 0.82rem;
  color: var(--gs-danger);
  background: var(--gs-bg-surface);
  border-bottom: 1px solid var(--gs-border);
  flex-shrink: 0;
}
</style>
