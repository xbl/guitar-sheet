<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, watch } from "vue"
import { useRouter } from "vue-router"
import { invoke } from "@tauri-apps/api/core"
import { convertFileSrc } from "@tauri-apps/api/core"
import { readFile, readTextFile } from "@tauri-apps/plugin-fs"
import * as pdfjsLib from "pdfjs-dist"
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url"
import type { SheetMeta } from "../types/sheet"

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl

const props = withDefaults(
  defineProps<{
    sheetId: string | null
    variant?: "embed" | "page"
    /**
     * 若与当前载入的曲谱 id 相同，本次载入完成后自动进入标题与正文编辑（用于新建后打开）。
     * 处理后会触发 `pendingTextEditConsumed` 以便父级清空。
     */
    pendingTextEditForSheetId?: string | null
  }>(),
  { variant: "embed", pendingTextEditForSheetId: null },
)

const emit = defineEmits<{
  deleted: [id: string]
  pendingTextEditConsumed: []
}>()

const router = useRouter()
const error = ref<string | null>(null)
const meta = ref<SheetMeta | null>(null)
const textBody = ref("")
const textBaseline = ref("")
const textDraft = ref("")
const editingText = ref(false)
const savingText = ref(false)

const titleEditing = ref(false)
const titleDraft = ref("")

const fontPx = ref(16)
const lineHeight = ref(1.6)
const imgSrc = ref("")
const textAreaRef = ref<HTMLTextAreaElement | null>(null)
const titleInputRef = ref<HTMLInputElement | null>(null)
/** Revoked when switching sheets or unmount; separate from imgSrc string when using blob URLs. */
let imageBlobUrl: string | null = null

type TextPreviewSeg =
  | { type: "text"; content: string }
  | { type: "img"; file: string }

function parseImageLine(line: string): string | null {
  const token = line.match(/^\s*\{\{IMG:([^}]+)\}\}\s*$/)
  if (token) return token[1]!.trim()
  const md = line.match(/^\s*!\[[^\]]*\]\(([^)]+)\)\s*$/)
  if (md) return md[1]!.trim()
  return null
}

function buildTextPreviewSegments(body: string): TextPreviewSeg[] {
  const lines = body.split("\n")
  const out: TextPreviewSeg[] = []
  const textBuf: string[] = []
  const flush = () => {
    if (textBuf.length) {
      out.push({ type: "text", content: textBuf.join("\n") })
      textBuf.length = 0
    }
  }
  for (const line of lines) {
    const file = parseImageLine(line)
    if (file) {
      flush()
      out.push({ type: "img", file })
    } else {
      textBuf.push(line)
    }
  }
  flush()
  return out
}

function siblingFilePath(tabAbs: string, file: string): string {
  const cleaned = file.replace(/^[/\\]+/, "")
  const name = cleaned.split(/[/\\]/).pop() || cleaned
  const i = Math.max(tabAbs.lastIndexOf("/"), tabAbs.lastIndexOf("\\"))
  const dir = i >= 0 ? tabAbs.slice(0, i) : tabAbs
  const sep = tabAbs.includes("\\") ? "\\" : "/"
  return `${dir}${sep}${name}`
}

const textPreviewSegments = computed(() =>
  meta.value?.kind === "text" ? buildTextPreviewSegments(textBody.value) : [],
)

const previewUrls = ref<Record<string, string>>({})
let previewGen = 0

function revokePreviewBlobUrls() {
  for (const url of Object.values(previewUrls.value) as string[]) {
    if (url.startsWith("blob:")) URL.revokeObjectURL(url)
  }
  previewUrls.value = {}
}

async function refreshTextPreviewImages() {
  const gen = ++previewGen
  revokePreviewBlobUrls()
  if (editingText.value || meta.value?.kind !== "text" || !meta.value.absolute_path) return

  const tabPath = meta.value.absolute_path
  const files = [
    ...new Set(
      buildTextPreviewSegments(textBody.value)
        .filter((s): s is { type: "img"; file: string } => s.type === "img")
        .map((s) => s.file),
    ),
  ]
  const next: Record<string, string> = {}
  for (const file of files) {
    if (gen !== previewGen) return
    try {
      const abs = siblingFilePath(tabPath, file)
      const bytes = await readFile(abs)
      const blob = new Blob([bytes], { type: guessImageMime(abs) })
      next[file] = URL.createObjectURL(blob)
    } catch {
      /* missing or unreadable */
    }
  }
  if (gen !== previewGen) {
    for (const u of Object.values(next)) URL.revokeObjectURL(u)
    return
  }
  previewUrls.value = next
}

function revokeImageBlobUrl() {
  if (imageBlobUrl) {
    URL.revokeObjectURL(imageBlobUrl)
    imageBlobUrl = null
  }
}

function guessImageMime(fsPath: string): string {
  const ext = fsPath.split(/[/\\]/).pop()?.split(".").pop()?.toLowerCase()
  switch (ext) {
    case "png":
      return "image/png"
    case "jpg":
    case "jpeg":
      return "image/jpeg"
    case "webp":
      return "image/webp"
    case "gif":
      return "image/gif"
    default:
      return "application/octet-stream"
  }
}

async function loadImageSrc(fsPath: string) {
  revokeImageBlobUrl()
  imgSrc.value = ""
  try {
    const bytes = await readFile(fsPath)
    const blob = new Blob([bytes], { type: guessImageMime(fsPath) })
    imageBlobUrl = URL.createObjectURL(blob)
    imgSrc.value = imageBlobUrl
  } catch {
    imgSrc.value = convertFileSrc(fsPath)
  }
}

const pdfPage = ref(1)
const pdfTotal = ref(0)
const canvasEl = ref<HTMLCanvasElement | null>(null)
let pdfDoc: import("pdfjs-dist").PDFDocumentProxy | null = null
let renderTask: { cancel: () => void } | null = null

const textDirty = computed(
  () => meta.value?.kind === "text" && editingText.value && textDraft.value !== textBaseline.value,
)

/** 编辑中文本仅空白时不能保存（不写入空正文）。 */
const textSaveEmpty = computed(
  () => meta.value?.kind === "text" && editingText.value && textDraft.value.trim() === "",
)

async function load() {
  error.value = null
  textBody.value = ""
  textBaseline.value = ""
  textDraft.value = ""
  editingText.value = false
  titleEditing.value = false
  revokePreviewBlobUrls()
  revokeImageBlobUrl()
  imgSrc.value = ""
  pdfDoc = null
  pdfTotal.value = 0
  pdfPage.value = 1
  meta.value = null

  const id = props.sheetId
  if (!id) return

  try {
    const m = await invoke<SheetMeta>("get_sheet", { id })
    meta.value = m
    const path = m.absolute_path
    if (m.kind === "text") {
      textBody.value = await readTextFile(path)
      textBaseline.value = textBody.value
      textDraft.value = textBody.value
      if (
        props.pendingTextEditForSheetId &&
        props.pendingTextEditForSheetId === m.id
      ) {
        titleDraft.value = m.display_title
        titleEditing.value = true
        editingText.value = true
        emit("pendingTextEditConsumed")
        await nextTick()
        titleInputRef.value?.focus()
        titleInputRef.value?.select()
      }
    } else if (m.kind === "image") {
      await loadImageSrc(path)
    } else if (m.kind === "pdf") {
      const loadingTask = pdfjsLib.getDocument({
        url: convertFileSrc(path),
      })
      pdfDoc = await loadingTask.promise
      pdfTotal.value = pdfDoc.numPages
      pdfPage.value = 1
      await nextTick()
      await renderPdfPage()
    }
  } catch (e) {
    error.value = String(e)
    meta.value = null
  }
}

async function saveTextBody() {
  const id = props.sheetId
  if (!id || !meta.value || meta.value.kind !== "text") return
  if (textDraft.value.trim() === "") return
  savingText.value = true
  error.value = null
  try {
    const updated = await invoke<SheetMeta>("save_text_sheet", {
      id,
      content: textDraft.value,
    })
    meta.value = updated
    textBody.value = textDraft.value
    textBaseline.value = textDraft.value
  } catch (e) {
    error.value = String(e)
  } finally {
    savingText.value = false
  }
}

function toggleTextEdit() {
  if (editingText.value && textDirty.value) {
    if (!confirm("正文已修改，退出编辑将丢失未保存的更改。确定退出？")) {
      return
    }
    textDraft.value = textBaseline.value
  }
  editingText.value = !editingText.value
  if (editingText.value) {
    textDraft.value = textBaseline.value
  }
}

function startTitleEdit() {
  if (!meta.value) return
  titleDraft.value = meta.value.display_title
  titleEditing.value = true
}

function cancelTitleEdit() {
  titleEditing.value = false
}

async function saveTitle() {
  const id = props.sheetId
  if (!id || !meta.value) return
  const t = titleDraft.value.trim()
  if (!t) {
    error.value = "标题不能为空。"
    return
  }
  error.value = null
  try {
    await invoke("rename_sheet_title", { id, title: t })
    meta.value = { ...meta.value, display_title: t }
    titleEditing.value = false
  } catch (e) {
    error.value = String(e)
  }
}

async function removeSheet() {
  const id = props.sheetId
  if (!id || !meta.value) return
  if (!confirm(`删除「${meta.value.display_title}」？本地文件会一并删除。`)) return
  error.value = null
  try {
    await invoke("delete_sheet", { id })
    emit("deleted", id)
    meta.value = null
    textBody.value = ""
  } catch (e) {
    error.value = String(e)
  }
}

async function renderPdfPage() {
  if (!pdfDoc || !canvasEl.value) return
  const page = await pdfDoc.getPage(pdfPage.value)
  const viewport = page.getViewport({ scale: 1.25 })
  const canvas = canvasEl.value
  const ctx = canvas.getContext("2d")
  if (!ctx) return
  if (renderTask) {
    try {
      renderTask.cancel()
    } catch {
      /* ignore */
    }
  }
  canvas.width = viewport.width
  canvas.height = viewport.height
  const task = page.render({ canvasContext: ctx, viewport })
  renderTask = task
  await task.promise
}

function prevPdf() {
  if (pdfPage.value > 1) {
    pdfPage.value -= 1
    void renderPdfPage()
  }
}

function nextPdf() {
  if (pdfDoc && pdfPage.value < pdfTotal.value) {
    pdfPage.value += 1
    void renderPdfPage()
  }
}

function mimeToImageExtension(mime: string): string {
  if (mime === "image/png") return "png"
  if (mime === "image/jpeg") return "jpg"
  if (mime === "image/webp") return "webp"
  if (mime === "image/gif") return "gif"
  return "png"
}

function uint8ToBase64(bytes: Uint8Array): string {
  let binary = ""
  const chunk = 0x8000
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
  }
  return btoa(binary)
}

function insertAtCursor(insert: string) {
  const ta = textAreaRef.value
  const text = textDraft.value
  if (!ta) {
    textDraft.value = text + insert
    return
  }
  const start = ta.selectionStart
  const end = ta.selectionEnd
  const before = text.slice(0, start)
  const after = text.slice(end)
  const prefix = before.length > 0 && !before.endsWith("\n") ? "\n" : ""
  const block = prefix + insert + "\n"
  textDraft.value = before + block + after
  void nextTick(() => {
    const pos = start + block.length
    ta.focus()
    ta.setSelectionRange(pos, pos)
  })
}

async function onTextPaste(e: ClipboardEvent) {
  if (!editingText.value || !props.sheetId || meta.value?.kind !== "text") return
  const cd = e.clipboardData
  if (!cd) return

  for (const item of Array.from(cd.items)) {
    if (!item.type.startsWith("image/")) continue
    const file = item.getAsFile()
    if (!file) continue
    e.preventDefault()
    error.value = null
    try {
      const buf = new Uint8Array(await file.arrayBuffer())
      const ext = mimeToImageExtension(file.type || item.type)
      const b64 = uint8ToBase64(buf)
      const snippet = await invoke<string>("save_sheet_clipboard_image", {
        sheetId: props.sheetId,
        imageBase64: b64,
        extension: ext,
      })
      insertAtCursor(snippet)
    } catch (e) {
      error.value = String(e)
    }
    return
  }
}

watch(
  () => props.sheetId,
  () => {
    void load()
  },
)

watch(
  () =>
    [textBody.value, meta.value?.absolute_path, meta.value?.kind, editingText.value] as const,
  () => {
    void refreshTextPreviewImages()
  },
)

onUnmounted(() => {
  revokePreviewBlobUrls()
  revokeImageBlobUrl()
})
</script>

<template>
  <div class="reader" :class="{ embed: variant === 'embed', page: variant === 'page' }">
    <header class="bar">
      <button
        v-if="variant === 'page'"
        type="button"
        class="back"
        @click="router.push('/')"
      >
        ← 谱库
      </button>
      <div v-if="meta" class="title-block">
        <template v-if="!titleEditing">
          <h1>{{ meta.display_title }}</h1>
          <button type="button" class="ghost" @click="startTitleEdit">改标题</button>
          <button
            v-if="variant === 'embed'"
            type="button"
            class="ghost danger"
            @click="removeSheet"
          >
            删除
          </button>
        </template>
        <template v-else>
          <input
            ref="titleInputRef"
            v-model="titleDraft"
            class="title-input"
            type="text"
            maxlength="200"
            @keydown.enter="saveTitle"
          />
          <button type="button" class="ghost" @click="saveTitle">保存</button>
          <button type="button" class="ghost" @click="cancelTitleEdit">取消</button>
        </template>
      </div>
      <div v-if="meta?.kind === 'text'" class="text-actions">
        <button type="button" @click="toggleTextEdit">
          {{ editingText ? "退出编辑" : "编辑正文" }}
        </button>
        <button
          type="button"
          class="primary"
          :disabled="!textDirty || savingText || textSaveEmpty"
          @click="saveTextBody"
        >
          {{ savingText ? "保存中…" : "保存正文" }}
        </button>
      </div>
    </header>

    <div class="reader-body">
      <p v-if="error" class="err">{{ error }}</p>

      <div v-else-if="!sheetId" class="empty">
        <p>在左侧树中选择一首曲谱，内容将显示在这里。</p>
      </div>

      <template v-else-if="meta">
        <section v-if="meta.kind === 'text'" class="text-wrap">
          <div class="controls">
            <label>字号 <input v-model.number="fontPx" type="range" min="12" max="32" /></label>
            <label>行距 <input v-model.number="lineHeight" type="range" min="1.2" max="2.4" step="0.05" /></label>
          </div>
          <p v-if="editingText" class="paste-hint">
            提示：编辑时 Ctrl+V 粘贴剪贴板图片会保存到曲谱同目录并在正文插入一行标记；退出编辑后正文里会显示图片预览。
          </p>
          <textarea
            v-if="editingText"
            ref="textAreaRef"
            v-model="textDraft"
            class="tab edit"
            :style="{ fontSize: fontPx + 'px', lineHeight: String(lineHeight) }"
            @paste="onTextPaste"
          />
          <div
            v-else
            class="tab text-preview"
            :style="{ fontSize: fontPx + 'px', lineHeight: String(lineHeight) }"
          >
            <template v-for="(seg, i) in textPreviewSegments" :key="i">
              <pre v-if="seg.type === 'text'" class="text-chunk">{{ seg.content }}</pre>
              <figure v-else class="inline-img-wrap">
                <img
                  v-if="previewUrls[seg.file]"
                  :src="previewUrls[seg.file]"
                  :alt="seg.file"
                  class="inline-img"
                />
                <p v-else class="img-missing">（无法加载图片：{{ seg.file }}）</p>
              </figure>
            </template>
          </div>
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
  </div>
</template>

<style scoped>
.reader {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1;
  overflow: hidden;
}
.reader.page {
  height: 100%;
  min-height: 0;
}
.reader.embed .bar {
  flex-shrink: 0;
}
.bar {
  flex-shrink: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
  padding: 0.65rem 0.75rem;
  border-bottom: 1px solid #e5e5e5;
  background: #fff;
}
.title-block {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
  min-width: 0;
}
.bar h1 {
  margin: 0;
  font-size: 1.05rem;
  flex: 1;
  min-width: 0;
}
.title-input {
  flex: 1;
  min-width: 8rem;
  padding: 0.35rem 0.5rem;
  font-size: 1rem;
  border: 1px solid #ccc;
  border-radius: 6px;
}
.ghost {
  border: none;
  background: none;
  cursor: pointer;
  color: #2563eb;
  font-size: 0.9rem;
  padding: 0.25rem 0.5rem;
}
.ghost.danger {
  color: #b00020;
}
.text-actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  width: 100%;
  justify-content: flex-end;
}
.text-actions button.primary {
  font-weight: 600;
  border: 1px solid #2563eb;
  background: #eff6ff;
  border-radius: 6px;
  padding: 0.35rem 0.65rem;
  cursor: pointer;
}
.text-actions button.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.back {
  border: none;
  background: none;
  cursor: pointer;
  color: #2563eb;
  font-size: 1rem;
}
.reader-body {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}
.err {
  color: #b00020;
  padding: 1rem;
}
.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: #888;
  font-size: 0.95rem;
}
.text-wrap {
  padding: 0.75rem 1rem 1rem;
  flex: 1 1 auto;
  min-height: min-content;
}
.controls {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
  margin-bottom: 0.75rem;
  font-size: 0.9rem;
}
.paste-hint {
  margin: 0 0 0.5rem;
  font-size: 0.8rem;
  color: #666;
  line-height: 1.4;
}
.tab {
  margin: 0;
  width: 100%;
  min-height: 10rem;
  box-sizing: border-box;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: #fafafa;
  border: 1px solid #eee;
  border-radius: 8px;
  padding: 1rem;
  overflow: auto;
}
.tab.edit {
  resize: vertical;
  line-height: inherit;
}
.text-preview {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.text-chunk {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  font-size: inherit;
  line-height: inherit;
}
.inline-img-wrap {
  margin: 0;
  padding: 0;
  display: flex;
  justify-content: flex-start;
}
.inline-img {
  max-width: 100%;
  height: auto;
  display: block;
  border-radius: 6px;
  border: 1px solid #e5e5e5;
}
.img-missing {
  margin: 0;
  font-size: 0.85rem;
  color: #888;
}
.img-wrap {
  flex: 1 1 auto;
  padding: 1rem;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  min-height: min-content;
}
.img-wrap img {
  max-width: 100%;
  height: auto;
}
.pdf-wrap {
  flex: 1 1 auto;
  padding: 1rem;
  min-height: min-content;
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
  border-radius: 6px;
  border: 1px solid #ccc;
  background: #fafafa;
}
</style>
