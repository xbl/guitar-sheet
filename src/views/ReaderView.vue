<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readTextFile } from "@tauri-apps/plugin-fs";
import * as pdfjsLib from "pdfjs-dist";
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import type { SheetMeta } from "../types/sheet";

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

const props = defineProps<{ id: string }>();

const router = useRouter();
const error = ref<string | null>(null);
const meta = ref<SheetMeta | null>(null);
const textBody = ref("");
const fontPx = ref(16);
const lineHeight = ref(1.6);
const imgSrc = ref("");

const pdfPage = ref(1);
const pdfTotal = ref(0);
const canvasEl = ref<HTMLCanvasElement | null>(null);
let pdfDoc: import("pdfjs-dist").PDFDocumentProxy | null = null;
let renderTask: { cancel: () => void } | null = null;

async function load() {
  error.value = null;
  textBody.value = "";
  imgSrc.value = "";
  pdfDoc = null;
  pdfTotal.value = 0;
  pdfPage.value = 1;
  try {
    const m = await invoke<SheetMeta>("get_sheet", { id: props.id });
    meta.value = m;
    const path = m.absolute_path;
    if (m.kind === "text") {
      textBody.value = await readTextFile(path);
    } else if (m.kind === "image") {
      imgSrc.value = convertFileSrc(path);
    } else if (m.kind === "pdf") {
      const loadingTask = pdfjsLib.getDocument({
        url: convertFileSrc(path),
      });
      pdfDoc = await loadingTask.promise;
      pdfTotal.value = pdfDoc.numPages;
      pdfPage.value = 1;
      await renderPdfPage();
    }
  } catch (e) {
    error.value = String(e);
    meta.value = null;
  }
}

async function renderPdfPage() {
  if (!pdfDoc || !canvasEl.value) return;
  const page = await pdfDoc.getPage(pdfPage.value);
  const viewport = page.getViewport({ scale: 1.25 });
  const canvas = canvasEl.value;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  if (renderTask) {
    try {
      renderTask.cancel();
    } catch {
      /* ignore */
    }
  }
  canvas.width = viewport.width;
  canvas.height = viewport.height;
  const task = page.render({ canvasContext: ctx, viewport });
  renderTask = task;
  await task.promise;
}

function prevPdf() {
  if (pdfPage.value > 1) {
    pdfPage.value -= 1;
    void renderPdfPage();
  }
}

function nextPdf() {
  if (pdfDoc && pdfPage.value < pdfDoc.numPages) {
    pdfPage.value += 1;
    void renderPdfPage();
  }
}

onMounted(() => {
  void load();
});

watch(
  () => props.id,
  () => {
    void load();
  },
);
</script>

<template>
  <div class="reader">
    <header class="bar">
      <button type="button" class="back" @click="router.push('/')">← 谱库</button>
      <h1 v-if="meta">{{ meta.display_title }}</h1>
    </header>

    <p v-if="error" class="err">{{ error }}</p>

    <template v-else-if="meta">
      <section v-if="meta.kind === 'text'" class="text-wrap">
        <div class="controls">
          <label>字号 <input v-model.number="fontPx" type="range" min="12" max="32" /></label>
          <label>行距 <input v-model.number="lineHeight" type="range" min="1.2" max="2.4" step="0.05" /></label>
        </div>
        <pre
          class="tab"
          :style="{ fontSize: fontPx + 'px', lineHeight: String(lineHeight) }"
        >{{ textBody }}</pre>
      </section>

      <section v-else-if="meta.kind === 'image'" class="img-wrap">
        <img v-if="imgSrc" :src="imgSrc" alt="sheet" />
      </section>

      <section v-else-if="meta.kind === 'pdf'" class="pdf-wrap">
        <div class="pdf-controls">
          <button type="button" :disabled="pdfPage <= 1" @click="prevPdf">上一页</button>
          <span>{{ pdfPage }} / {{ pdfTotal || "…" }}</span>
          <button type="button" :disabled="!pdfDoc || pdfPage >= pdfTotal" @click="nextPdf">下一页</button>
        </div>
        <canvas ref="canvasEl" class="pdf-canvas" />
      </section>
    </template>
  </div>
</template>

<style scoped>
.reader {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}
.bar {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #e5e5e5;
  position: sticky;
  top: 0;
  background: #fff;
  z-index: 2;
}
.bar h1 {
  margin: 0;
  font-size: 1.1rem;
  flex: 1;
}
.back {
  border: none;
  background: none;
  cursor: pointer;
  color: #2563eb;
  font-size: 1rem;
}
.err {
  color: #b00020;
  padding: 1rem;
}
.text-wrap {
  padding: 1rem;
  flex: 1;
}
.controls {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  margin-bottom: 0.75rem;
  font-size: 0.9rem;
}
.tab {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: #fafafa;
  border: 1px solid #eee;
  border-radius: 8px;
  padding: 1rem;
  overflow: auto;
}
.img-wrap {
  flex: 1;
  padding: 1rem;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  overflow: auto;
}
.img-wrap img {
  max-width: 100%;
  height: auto;
}
.pdf-wrap {
  flex: 1;
  padding: 1rem;
  overflow: auto;
}
.pdf-controls {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}
.pdf-canvas {
  display: block;
  margin: 0 auto;
  max-width: 100%;
  height: auto;
  border: 1px solid #ddd;
}
button {
  cursor: pointer;
  padding: 0.35rem 0.65rem;
}
</style>
