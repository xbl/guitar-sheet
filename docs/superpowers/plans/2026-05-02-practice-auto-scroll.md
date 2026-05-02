# Practice Auto-Scroll + Metronome Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Vue composables and UI for metronome (BPM, mute) plus discrete 1–20 scroll-speed levels driving vertical auto-scroll in a scrollable sheet container, with bottom-stop and localStorage persistence, matching [练习模式：节拍器 + 谱面自动滚动](../specs/2026-05-02-practice-auto-scroll-design.md).

**Architecture:** Pure TypeScript modules map scroll level → pixels-per-second and detect scroll bottom; `useAutoScroll` runs a `requestAnimationFrame` loop updating `scrollTop`; `useMetronome` uses `AudioContext` for short oscillator clicks on an interval derived from BPM (`setInterval` period `60/bpm` seconds), `muted` skips scheduling audio only. `PracticeToolbar.vue` binds controls; a thin `ReaderView.vue` shell provides `overflow-y: auto` + placeholder content until the MVP reader replaces internals. **Explicitly not in this slice:** pausing when the window is hidden (spec allows omitting in v1).

**Tech Stack:** Vue 3 (`<script setup>`), TypeScript, Vite 6, Vitest 3 + `happy-dom` for unit tests, Web Audio API (browser).

---

## File map (create / modify)

| Path | Responsibility |
|------|----------------|
| `package.json` | Add `vitest`, `@vitejs/plugin-vue` already present; add `happy-dom`, test script. |
| `vite.config.ts` | Merge Vitest config (`test.environment`, `include`). |
| `src/practice/constants.ts` | BPM / level ranges, `localStorage` keys, default values. |
| `src/practice/scrollSpeed.ts` | `clampScrollLevel`, `scrollLevelToPixelsPerSecond`. |
| `src/practice/scrollGeometry.ts` | `maxScrollTop`, `isAtScrollBottom`, `applyScrollDelta`. |
| `src/practice/practicePreferences.ts` | Read/write BPM, level, mute from `localStorage` with JSON-safe typing. |
| `src/composables/useAutoScroll.ts` | rAF loop, `onStoppedAtBottom` callback. |
| `src/composables/useMetronome.ts` | `AudioContext`, scheduling, `resume()` on play. |
| `src/components/practice/PracticeToolbar.vue` | Play/pause, BPM number input, mute checkbox, 1–20 range input. |
| `src/views/ReaderView.vue` | Scroll container `ref`, long placeholder sheet, mounts toolbar + wires composables. |
| `src/App.vue` | Replace demo with `<ReaderView />` (or keep minimal wrapper). |

---

### Task 1: Vitest + happy-dom wiring

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`

- [ ] **Step 1: Install dev dependencies**

Run:

```bash
cd /Users/blxie/workspace/guitar-sheet
npm install -D vitest@^3 happy-dom@^15
```

- [ ] **Step 2: Add npm scripts**

In `package.json`, inside `"scripts"`, add:

```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 3: Extend `vite.config.ts`**

Replace file content with (preserve existing `host` / `server` / `watch` options):

```typescript
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    environment: "happy-dom",
    include: ["src/**/*.test.ts"],
    globals: false,
  },
}));
```

- [ ] **Step 4: Run Vitest (smoke)**

Run: `npm run test`

Expected: `No test files found` exit 0 or Vitest completes with 0 tests (until Task 2).

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json vite.config.ts
git commit -m "chore: add vitest and happy-dom for frontend unit tests"
```

---

### Task 2: Constants + scroll speed mapping + tests

**Files:**
- Create: `src/practice/constants.ts`
- Create: `src/practice/scrollSpeed.ts`
- Create: `src/practice/scrollSpeed.test.ts`

- [ ] **Step 1: Write failing tests**

Create `src/practice/scrollSpeed.test.ts`:

```typescript
import { describe, expect, it } from "vitest";
import { clampScrollLevel, scrollLevelToPixelsPerSecond } from "./scrollSpeed";

describe("clampScrollLevel", () => {
  it("clamps to 1–20 and rounds", () => {
    expect(clampScrollLevel(0)).toBe(1);
    expect(clampScrollLevel(21)).toBe(20);
    expect(clampScrollLevel(7.6)).toBe(8);
  });
});

describe("scrollLevelToPixelsPerSecond", () => {
  it("is monotonic and maps 1→10 and 20→200 px/s", () => {
    expect(scrollLevelToPixelsPerSecond(1)).toBeCloseTo(10, 5);
    expect(scrollLevelToPixelsPerSecond(20)).toBeCloseTo(200, 5);
    expect(scrollLevelToPixelsPerSecond(10)).toBeGreaterThan(
      scrollLevelToPixelsPerSecond(9),
    );
  });
});
```

Run: `npm run test`

Expected: FAIL — modules missing.

- [ ] **Step 2: Implement**

Create `src/practice/constants.ts`:

```typescript
export const SCROLL_LEVEL_MIN = 1;
export const SCROLL_LEVEL_MAX = 20;
export const SCROLL_LEVEL_DEFAULT = 10;

export const BPM_MIN = 40;
export const BPM_MAX = 240;
export const BPM_DEFAULT = 120;

export const STORAGE_KEY_BPM = "guitar-sheet.practice.bpm";
export const STORAGE_KEY_SCROLL_LEVEL = "guitar-sheet.practice.scrollLevel";
export const STORAGE_KEY_METRONOME_MUTED = "guitar-sheet.practice.metronomeMuted";

/** Linear map: level 1 → 10 px/s, level 20 → 200 px/s */
export const SCROLL_PX_PER_SEC_AT_MIN = 10;
export const SCROLL_PX_PER_SEC_AT_MAX = 200;
```

Create `src/practice/scrollSpeed.ts`:

```typescript
import {
  SCROLL_LEVEL_MAX,
  SCROLL_LEVEL_MIN,
  SCROLL_PX_PER_SEC_AT_MAX,
  SCROLL_PX_PER_SEC_AT_MIN,
} from "./constants";

export function clampScrollLevel(raw: number): number {
  const rounded = Math.round(raw);
  return Math.min(SCROLL_LEVEL_MAX, Math.max(SCROLL_LEVEL_MIN, rounded));
}

export function scrollLevelToPixelsPerSecond(level: number): number {
  const lv = clampScrollLevel(level);
  const t =
    (lv - SCROLL_LEVEL_MIN) / (SCROLL_LEVEL_MAX - SCROLL_LEVEL_MIN);
  return SCROLL_PX_PER_SEC_AT_MIN + t * (SCROLL_PX_PER_SEC_AT_MAX - SCROLL_PX_PER_SEC_AT_MIN);
}
```

Run: `npm run test`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/practice/constants.ts src/practice/scrollSpeed.ts src/practice/scrollSpeed.test.ts
git commit -m "feat(practice): add scroll level to px/s mapping with tests"
```

---

### Task 3: Scroll geometry helpers + tests

**Files:**
- Create: `src/practice/scrollGeometry.ts`
- Create: `src/practice/scrollGeometry.test.ts`

- [ ] **Step 1: Write failing tests**

Create `src/practice/scrollGeometry.test.ts`:

```typescript
import { describe, expect, it } from "vitest";
import {
  applyScrollDelta,
  isAtScrollBottom,
  maxScrollTop,
} from "./scrollGeometry";

describe("maxScrollTop", () => {
  it("returns 0 when content fits", () => {
    expect(maxScrollTop(100, 120)).toBe(0);
  });
  it("returns overflow when content taller than viewport", () => {
    expect(maxScrollTop(500, 100)).toBe(400);
  });
});

describe("isAtScrollBottom", () => {
  it("returns true within epsilon of bottom", () => {
    expect(isAtScrollBottom(398, 500, 100, 3)).toBe(true);
  });
  it("returns false when not near bottom", () => {
    expect(isAtScrollBottom(0, 500, 100)).toBe(false);
  });
});

describe("applyScrollDelta", () => {
  it("clamps to max scroll top", () => {
    const r = applyScrollDelta({ scrollTop: 390, scrollHeight: 500, clientHeight: 100, deltaY: 50 });
    expect(r.nextScrollTop).toBe(400);
    expect(r.reachedBottom).toBe(true);
  });
});
```

Run: `npm run test` → FAIL until implementation.

- [ ] **Step 2: Implement**

Create `src/practice/scrollGeometry.ts`:

```typescript
const DEFAULT_EPS_PX = 2;

export function maxScrollTop(scrollHeight: number, clientHeight: number): number {
  return Math.max(0, scrollHeight - clientHeight);
}

export function isAtScrollBottom(
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
  epsilonPx: number = DEFAULT_EPS_PX,
): boolean {
  const max = maxScrollTop(scrollHeight, clientHeight);
  return scrollTop >= max - epsilonPx;
}

export function applyScrollDelta(params: {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
  deltaY: number;
  epsilonPx?: number;
}): { nextScrollTop: number; reachedBottom: boolean } {
  const max = maxScrollTop(params.scrollHeight, params.clientHeight);
  const next = Math.min(max, params.scrollTop + params.deltaY);
  const eps = params.epsilonPx ?? DEFAULT_EPS_PX;
  const reachedBottom = next >= max - eps;
  return { nextScrollTop: next, reachedBottom };
}
```

Run: `npm run test` → PASS.

- [ ] **Step 3: Commit**

```bash
git add src/practice/scrollGeometry.ts src/practice/scrollGeometry.test.ts
git commit -m "feat(practice): add scroll geometry helpers with tests"
```

---

### Task 4: practicePreferences (localStorage)

**Files:**
- Create: `src/practice/practicePreferences.ts`
- Create: `src/practice/practicePreferences.test.ts`

- [ ] **Step 1: Write tests with stubbed storage**

Create `src/practice/practicePreferences.test.ts`:

```typescript
import { afterEach, describe, expect, it } from "vitest";
import {
  loadPracticePreferences,
  savePracticePreferences,
} from "./practicePreferences";

function mockStorage() {
  const m = new Map<string, string>();
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => {
      m.set(k, v);
    },
  } as Storage;
}

describe("practicePreferences", () => {
  afterEach(() => {
    localStorage.clear();
  });

  it("round-trips defaults when empty", () => {
    const s = mockStorage();
    const a = loadPracticePreferences(s);
    expect(a.bpm).toBe(120);
    savePracticePreferences(s, { ...a, bpm: 90 });
    const b = loadPracticePreferences(s);
    expect(b.bpm).toBe(90);
  });
});
```

Implement `src/practice/practicePreferences.ts`:

```typescript
import {
  BPM_DEFAULT,
  BPM_MAX,
  BPM_MIN,
  SCROLL_LEVEL_DEFAULT,
  STORAGE_KEY_BPM,
  STORAGE_KEY_METRONOME_MUTED,
  STORAGE_KEY_SCROLL_LEVEL,
} from "./constants";
import { clampScrollLevel } from "./scrollSpeed";

export type PracticePreferences = {
  bpm: number;
  scrollLevel: number;
  metronomeMuted: boolean;
};

function clampBpm(raw: number): number {
  const r = Math.round(raw);
  return Math.min(BPM_MAX, Math.max(BPM_MIN, r));
}

export function loadPracticePreferences(storage: Storage = localStorage): PracticePreferences {
  let bpm = BPM_DEFAULT;
  let scrollLevel = SCROLL_LEVEL_DEFAULT;
  let metronomeMuted = false;

  const rawBpm = storage.getItem(STORAGE_KEY_BPM);
  if (rawBpm != null) {
    const n = Number(rawBpm);
    if (!Number.isNaN(n)) bpm = clampBpm(n);
  }

  const rawLv = storage.getItem(STORAGE_KEY_SCROLL_LEVEL);
  if (rawLv != null) {
    const n = Number(rawLv);
    if (!Number.isNaN(n)) scrollLevel = clampScrollLevel(n);
  }

  const rawMute = storage.getItem(STORAGE_KEY_METRONOME_MUTED);
  if (rawMute === "1" || rawMute === "true") metronomeMuted = true;
  if (rawMute === "0" || rawMute === "false") metronomeMuted = false;

  return { bpm, scrollLevel, metronomeMuted };
}

export function savePracticePreferences(
  storage: Storage,
  prefs: PracticePreferences,
): void {
  storage.setItem(STORAGE_KEY_BPM, String(clampBpm(prefs.bpm)));
  storage.setItem(STORAGE_KEY_SCROLL_LEVEL, String(clampScrollLevel(prefs.scrollLevel)));
  storage.setItem(STORAGE_KEY_METRONOME_MUTED, prefs.metronomeMuted ? "1" : "0");
}
```

Run: `npm run test` → PASS (export test file after adding impl).

- [ ] **Step 2: Commit**

```bash
git add src/practice/practicePreferences.ts src/practice/practicePreferences.test.ts
git commit -m "feat(practice): persist BPM, scroll level, and metronome mute"
```

---

### Task 5: `useAutoScroll` composable

**Files:**
- Create: `src/composables/useAutoScroll.ts`

- [ ] **Step 1: Implement composable** (no Vitest for DOM timing here; covered indirectly via geometry tests)

Create `src/composables/useAutoScroll.ts`:

```typescript
import { onScopeDispose, watch, type Ref } from "vue";
import { scrollLevelToPixelsPerSecond } from "../practice/scrollSpeed";
import { applyScrollDelta } from "../practice/scrollGeometry";

export function useAutoScroll(options: {
  scrollParentRef: Ref<HTMLElement | null>;
  isPlaying: Ref<boolean>;
  scrollLevel: Ref<number>;
  onStoppedAtBottom?: () => void;
}) {
  const { scrollParentRef, isPlaying, scrollLevel, onStoppedAtBottom } = options;
  let rafId: number | null = null;
  let lastTs: number | null = null;

  function tick(now: number) {
    const el = scrollParentRef.value;
    if (!el || !isPlaying.value) {
      lastTs = null;
      return;
    }
    if (lastTs == null) lastTs = now;
    const dtMs = now - lastTs;
    lastTs = now;

    const pxPerSec = scrollLevelToPixelsPerSecond(scrollLevel.value);
    const deltaY = (pxPerSec * dtMs) / 1000;

    const { nextScrollTop, reachedBottom } = applyScrollDelta({
      scrollTop: el.scrollTop,
      scrollHeight: el.scrollHeight,
      clientHeight: el.clientHeight,
      deltaY,
    });

    el.scrollTop = nextScrollTop;

    if (reachedBottom) {
      isPlaying.value = false;
      lastTs = null;
      onStoppedAtBottom?.();
      return;
    }

    rafId = requestAnimationFrame(tick);
  }

  function stopLoop() {
    if (rafId != null) cancelAnimationFrame(rafId);
    rafId = null;
    lastTs = null;
  }

  watch(
    isPlaying,
    (playing) => {
      stopLoop();
      if (playing) {
        lastTs = null;
        rafId = requestAnimationFrame(tick);
      }
    },
    { flush: "sync" },
  );

  watch(scrollLevel, () => {
    if (isPlaying.value) {
      lastTs = null;
    }
  });

  onScopeDispose(stopLoop);

  return { stopLoop };
}
```

- [ ] **Step 2: Run typecheck**

Run: `npm run build`

Expected: `vue-tsc --noEmit && vite build` passes.

- [ ] **Step 3: Commit**

```bash
git add src/composables/useAutoScroll.ts
git commit -m "feat(practice): add useAutoScroll with rAF and bottom stop"
```

---

### Task 6: `useMetronome` composable (Web Audio)

**Files:**
- Create: `src/composables/useMetronome.ts`

- [ ] **Step 1: Implement**

Create `src/composables/useMetronome.ts`:

```typescript
import { onScopeDispose, watch, type Ref } from "vue";

/** One short click per beat; `muted` skips audio only (interval still runs). */
export function useMetronome(options: {
  bpm: Ref<number>;
  muted: Ref<boolean>;
  isPlaying: Ref<boolean>;
  onAudioUnavailable?: (reason: string) => void;
}) {
  const { bpm, muted, isPlaying, onAudioUnavailable } = options;
  let ctx: AudioContext | null = null;
  let intervalId: ReturnType<typeof setInterval> | null = null;

  function ensureContext(): AudioContext {
    if (!ctx) ctx = new AudioContext();
    return ctx;
  }

  function playClick() {
    const c = ensureContext();
    if (muted.value) return;

    const when = c.currentTime + 0.02;
    const osc = c.createOscillator();
    const g = c.createGain();
    osc.type = "sine";
    osc.frequency.value = 1000;
    osc.connect(g);
    g.connect(c.destination);

    g.gain.setValueAtTime(0.0001, when);
    g.gain.exponentialRampToValueAtTime(0.25, when + 0.005);
    g.gain.exponentialRampToValueAtTime(0.0001, when + 0.06);

    osc.start(when);
    osc.stop(when + 0.1);
  }

  function stopScheduling() {
    if (intervalId != null) {
      clearInterval(intervalId);
      intervalId = null;
    }
  }

  function restartScheduling() {
    stopScheduling();
    if (!isPlaying.value) return;

    const c = ensureContext();
    void c.resume().catch(() => {
      onAudioUnavailable?.("AudioContext could not resume");
    });

    const periodMs = (60 / Math.max(1, bpm.value)) * 1000;
    playClick();
    intervalId = setInterval(() => {
      if (!isPlaying.value) return;
      playClick();
    }, periodMs);
  }

  watch(
    [isPlaying, bpm, muted],
    () => {
      stopScheduling();
      if (isPlaying.value) restartScheduling();
    },
    { flush: "post" },
  );

  onScopeDispose(stopScheduling);

  return {
    async resumeIfNeeded() {
      const c = ensureContext();
      if (c.state === "suspended") await c.resume();
    },
  };
}
```

- [ ] **Step 2: Run typecheck**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src/composables/useMetronome.ts
git commit -m "feat(practice): add Web Audio metronome with mute"
```

---

### Task 7: `PracticeToolbar.vue` + `ReaderView.vue` shell

**Files:**
- Create: `src/components/practice/PracticeToolbar.vue`
- Create: `src/views/ReaderView.vue`
- Modify: `src/App.vue`

- [ ] **Step 1: Add toolbar**

Create `src/components/practice/PracticeToolbar.vue`:

```vue
<script setup lang="ts">
import { BPM_MAX, BPM_MIN, SCROLL_LEVEL_MAX, SCROLL_LEVEL_MIN } from "../../practice/constants";

defineProps<{
  isPlaying: boolean;
  bpm: number;
  scrollLevel: number;
  metronomeMuted: boolean;
  audioWarning?: string;
}>();

const emit = defineEmits<{
  togglePlay: [];
  "update:bpm": [number];
  "update:scrollLevel": [number];
  "update:metronomeMuted": [boolean];
}>();
</script>

<template>
  <div class="practice-toolbar" role="toolbar" aria-label="练习">
    <button type="button" @click="emit('togglePlay')">
      {{ isPlaying ? "暂停" : "播放" }}
    </button>
    <label>
      BPM
      <input
        type="number"
        :min="BPM_MIN"
        :max="BPM_MAX"
        :value="bpm"
        @input="
          emit('update:bpm', Number(($event.target as HTMLInputElement).value))
        "
      />
    </label>
    <label>
      <input
        type="checkbox"
        :checked="metronomeMuted"
        @change="
          emit(
            'update:metronomeMuted',
            ($event.target as HTMLInputElement).checked,
          )
        "
      />
      节拍器静音
    </label>
    <label>
      滚动
      <input
        type="range"
        :min="SCROLL_LEVEL_MIN"
        :max="SCROLL_LEVEL_MAX"
        step="1"
        :value="scrollLevel"
        @input="
          emit(
            'update:scrollLevel',
            Number(($event.target as HTMLInputElement).value),
          )
        "
      />
      {{ scrollLevel }}
    </label>
    <p v-if="audioWarning" class="warn">{{ audioWarning }}</p>
  </div>
</template>

<style scoped>
.practice-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
  padding: 0.5rem;
  border-bottom: 1px solid #ccc;
}
.warn {
  color: #a40;
  margin: 0;
  flex-basis: 100%;
}
</style>
```

- [ ] **Step 2: Add reader shell**

Create `src/views/ReaderView.vue`:

```vue
<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import PracticeToolbar from "../components/practice/PracticeToolbar.vue";
import { useAutoScroll } from "../composables/useAutoScroll";
import { useMetronome } from "../composables/useMetronome";
import {
  loadPracticePreferences,
  savePracticePreferences,
} from "../practice/practicePreferences";

const placeholderLines = Array.from(
  { length: 120 },
  (_, i) => `小节占位 ${i + 1} — e|--2--3--|`,
).join("\n");

const scrollEl = ref<HTMLElement | null>(null);
const isPlaying = ref(false);
const bpm = ref(120);
const scrollLevel = ref(10);
const metronomeMuted = ref(false);
const audioWarning = ref("");

onMounted(() => {
  const p = loadPracticePreferences();
  bpm.value = p.bpm;
  scrollLevel.value = p.scrollLevel;
  metronomeMuted.value = p.metronomeMuted;
});

watch([bpm, scrollLevel, metronomeMuted], () => {
  savePracticePreferences(localStorage, {
    bpm: bpm.value,
    scrollLevel: scrollLevel.value,
    metronomeMuted: metronomeMuted.value,
  });
});

useAutoScroll({
  scrollParentRef: scrollEl,
  isPlaying,
  scrollLevel,
});

const { resumeIfNeeded } = useMetronome({
  bpm,
  muted: metronomeMuted,
  isPlaying,
  onAudioUnavailable: (r) => {
    audioWarning.value = r;
  },
});

async function onTogglePlay() {
  if (!isPlaying.value) {
    await resumeIfNeeded();
  }
  isPlaying.value = !isPlaying.value;
}
</script>

<template>
  <div class="reader">
    <PracticeToolbar
      :is-playing="isPlaying"
      :bpm="bpm"
      :scroll-level="scrollLevel"
      :metronome-muted="metronomeMuted"
      :audio-warning="audioWarning"
      @toggle-play="onTogglePlay"
      @update:bpm="(v) => (bpm.value = v)"
      @update:scroll-level="(v) => (scrollLevel.value = v)"
      @update:metronome-muted="(v) => (metronomeMuted.value = v)"
    />
    <div ref="scrollEl" class="sheet-scroll" tabindex="0">
      <pre class="sheet-placeholder">{{ placeholderLines }}</pre>
    </div>
  </div>
</template>

<style scoped>
.reader {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.sheet-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
  background: #fafafa;
}
.sheet-placeholder {
  margin: 0;
  font-family: ui-monospace, monospace;
  white-space: pre-wrap;
}
</style>
```

- [ ] **Step 3: Point `App.vue` at reader**

Replace `src/App.vue` template root content with:

```vue
<script setup lang="ts">
import ReaderView from "./views/ReaderView.vue";
</script>

<template>
  <ReaderView />
</template>

<style>
html,
body,
#app {
  margin: 0;
  height: 100%;
}
</style>
```

Remove unused `greet` demo imports and styles from `App.vue` that are no longer referenced.

- [ ] **Step 4: Run dev smoke**

Run: `npm run dev`

Manually: play → scroll moves + click sounds; mute → scroll continues; slide level → speed changes; reach bottom → stops.

- [ ] **Step 5: Run tests + build**

Run: `npm run test && npm run build`

Expected: all tests pass; build passes.

- [ ] **Step 6: Commit**

```bash
git add src/components/practice/PracticeToolbar.vue src/views/ReaderView.vue src/App.vue
git commit -m "feat(practice): wire toolbar and reader shell with auto-scroll"
```

---

### Self-review (plan vs spec)

**Spec coverage**

| 规格条目 | 对应任务 |
|----------|----------|
| 播放/暂停、触底停播 | Task 5、Task 7 |
| 1–20 档滚动 + 滑杆 | Task 2、Task 7 |
| BPM + 静音不关滚动 | Task 6、Task 7 |
| localStorage 持久化 | Task 4、Task 7 |
| Web Audio + suspended resume | Task 6（`resumeIfNeeded` 在播放前调用） |
| 窗口隐藏暂停 | **未实现**（规格允许 v1 省略；已在架构段声明） |

**Placeholder scan:** 无 `TBD` / 空实现步骤。

**类型一致性：** `scrollLevel`、`bpm`、`metronomeMuted` 在 `PracticePreferences` 与 `ReaderView` 中同名；`useMetronome` / `useAutoScroll` 共用 `isPlaying`。

**后续（非本计划范围）：** 语音/哼唱定位并驱动滚谱见 [练习模式规格 §8](../specs/2026-05-02-practice-auto-scroll-design.md)（需未来独立计划与专项规格）。

---

### Execution handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-02-practice-auto-scroll.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**

- **Subagent-Driven:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`.
- **Inline:** REQUIRED SUB-SKILL: `superpowers:executing-plans`.
