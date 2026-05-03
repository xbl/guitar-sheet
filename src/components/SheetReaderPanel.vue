<script setup lang="ts">
import {
  computed,
  nextTick,
  onUnmounted,
  provide,
  reactive,
  ref,
  watch,
} from "vue"
import { useRouter } from "vue-router"
import { invoke } from "@tauri-apps/api/core"
import { convertFileSrc } from "@tauri-apps/api/core"
import { readFile, readTextFile } from "@tauri-apps/plugin-fs"
import * as pdfjsLib from "pdfjs-dist"
import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url"
import type { SheetMeta } from "../types/sheet"
import ChordSheetRenderer from "./chords/ChordSheetRenderer.vue"
import PracticeToolbar from "./practice/PracticeToolbar.vue"
import ReaderChordSettingsPanel from "./ReaderChordSettingsPanel.vue"
import {
  convertAsciiChordSheetToChordPro,
  shouldConvertAsciiToChordPro,
} from "../chords/convertAsciiChordSheet"
import { looksLikeChordSheet } from "../chords/parseChordSheet"
import {
  normalizeReaderChordPrefs,
  readerChordPrefsInjectionKey,
  ZOOM_FONT_PX,
} from "../chords/readerPrefs"
import {
  loadSheetReaderStoredState,
  parseSheetReaderStoredStateJson,
  saveSheetReaderStoredState,
  serializeSheetReaderStoredState,
} from "../chords/sheetReaderState"
import type { SheetReaderStoredState } from "../chords/sheetReaderState"
import {
  buildTextPreviewSegments,
  listEmbedFilenames,
} from "../chords/textSheetEmbeds"
import { useAutoScroll } from "../composables/useAutoScroll"
import { useMetronome } from "../composables/useMetronome"
import { normalizePracticePreferences } from "../practice/practicePreferences"
import { confirmTwice } from "../utils/confirmTwice"
import { dataTransferLooksLikeFileDrag, inferAttachmentExtension } from "../utils/attachmentDrop"
import { showToast } from "../utils/toast"

pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl

const props = withDefaults(
  defineProps<{
    sheetId: string | null
    variant?: "embed" | "page"
    /**
     * Parent increments after谱库树拖放移动（同 sheetId 时磁盘路径会变）。
     */
    reloadNonce?: number
  }>(),
  { variant: "embed", reloadNonce: 0 },
)

const emit = defineEmits<{
  deleted: [id: string]
  /** 标题已在后端更新；父级应同步谱库列表中的 display_title */
  titleRenamed: [{ id: string; title: string }]
  /**
   * 当前文本谱是否为「未命名 + 空正文」可丢弃草稿（切换树选中时父级可自动删除）。
   */
  emptyDraftChange: [id: string | null]
}>()

const router = useRouter()
const error = ref<string | null>(null)
const meta = ref<SheetMeta | null>(null)
const textBody = ref("")
const textBaseline = ref("")
const textDraft = ref("")
const editingText = ref(false)

const titleEditing = ref(false)
const titleDraft = ref("")

const chordPrefs = reactive(normalizeReaderChordPrefs(undefined))
provide(readerChordPrefsInjectionKey, chordPrefs)
const fontPx = ref(ZOOM_FONT_PX[chordPrefs.zoomLevel as 0 | 1 | 2])
const TEXT_LINE_HEIGHT = 1.6

watch(
  () => chordPrefs.zoomLevel,
  (z) => {
    fontPx.value = ZOOM_FONT_PX[z as 0 | 1 | 2]
  },
)
const imgSrc = ref("")
const textAreaRef = ref<HTMLTextAreaElement | null>(null)
const titleInputRef = ref<HTMLInputElement | null>(null)
const readerBodyRef = ref<HTMLElement | null>(null)
const practicePlaying = ref(false)
const practiceBpm = ref(120)
const practiceScrollLevel = ref(20)
const practiceMetronomeMuted = ref(false)
const practiceAudioWarning = ref("")
/** Revoked when switching sheets or unmount; separate from imgSrc string when using blob URLs. */
let imageBlobUrl: string | null = null
const readerSettingsOpen = ref(false)
const readerSettingsSlotRef = ref<HTMLElement | null>(null)
let readerSettingsDismiss: ((e: MouseEvent) => void) | null = null

function onReaderSettingsEscape(e: KeyboardEvent) {
  if (e.key === "Escape") readerSettingsOpen.value = false
}

function collectSheetReaderState(): SheetReaderStoredState {
  return {
    chord: { ...chordPrefs },
    practice: {
      bpm: practiceBpm.value,
      scrollLevel: practiceScrollLevel.value,
      metronomeMuted: practiceMetronomeMuted.value,
    },
  }
}

let readerPersistTimer: ReturnType<typeof setTimeout> | null = null

async function flushReaderStateToBackendAndLocal(sheetId: string) {
  const state = collectSheetReaderState()
  saveSheetReaderStoredState(localStorage, sheetId, state)
  try {
    await invoke("set_sheet_reader_state", {
      sheetId,
      json: serializeSheetReaderStoredState(state),
    })
  } catch {
    /* web / offline */
  }
}

function schedulePersistReader() {
  const id = props.sheetId
  if (!id) return
  if (readerPersistTimer) clearTimeout(readerPersistTimer)
  readerPersistTimer = setTimeout(() => {
    readerPersistTimer = null
    void flushReaderStateToBackendAndLocal(id)
  }, 400)
}

async function flushReaderStateNow(sheetId: string) {
  if (readerPersistTimer) {
    clearTimeout(readerPersistTimer)
    readerPersistTimer = null
  }
  await flushReaderStateToBackendAndLocal(sheetId)
}

function applySheetReaderState(s: SheetReaderStoredState) {
  Object.assign(chordPrefs, s.chord)
  practiceBpm.value = s.practice.bpm
  practiceScrollLevel.value = s.practice.scrollLevel
  practiceMetronomeMuted.value = s.practice.metronomeMuted
}

function resetReaderStateToDefaults() {
  Object.assign(chordPrefs, normalizeReaderChordPrefs(undefined))
  const d = normalizePracticePreferences(undefined)
  practiceBpm.value = d.bpm
  practiceScrollLevel.value = d.scrollLevel
  practiceMetronomeMuted.value = d.metronomeMuted
}

async function hydrateReaderForSheet(sheetId: string) {
  try {
    const raw = await invoke<string | null>("get_sheet_reader_state", { sheetId })
    if (raw != null && String(raw).trim() !== "") {
      const parsed = parseSheetReaderStoredStateJson(String(raw))
      if (parsed) {
        applySheetReaderState(parsed)
        saveSheetReaderStoredState(localStorage, sheetId, parsed)
        return
      }
    }
  } catch {
    /* not running in Tauri */
  }
  applySheetReaderState(loadSheetReaderStoredState(localStorage, sheetId))
}

watch(
  chordPrefs,
  () => {
    schedulePersistReader()
  },
  { deep: true },
)

watch([practiceBpm, practiceScrollLevel, practiceMetronomeMuted], () => {
  schedulePersistReader()
})

watch(readerSettingsOpen, (open) => {
  if (open) {
    void nextTick(() => {
      const fn = (e: MouseEvent) => {
        const el = readerSettingsSlotRef.value
        if (el && !el.contains(e.target as Node)) readerSettingsOpen.value = false
      }
      readerSettingsDismiss = fn
      document.addEventListener("mousedown", fn, true)
      document.addEventListener("keydown", onReaderSettingsEscape)
    })
  } else {
    document.removeEventListener("keydown", onReaderSettingsEscape)
    if (readerSettingsDismiss) {
      document.removeEventListener("mousedown", readerSettingsDismiss, true)
      readerSettingsDismiss = null
    }
  }
})

watch(editingText, (ed) => {
  if (ed) {
    practicePlaying.value = false
    readerSettingsOpen.value = false
  }
})

/** 未命名且正文为空：切换左侧树其他项时由父级删除该谱 */
const abandonableEmptySheetId = computed(() => {
  if (!props.sheetId || !meta.value || meta.value.kind !== "text") return null
  const title = titleEditing.value
    ? titleDraft.value.trim()
    : meta.value.display_title.trim()
  const body = (editingText.value ? textDraft.value : textBody.value).trim()
  if (title === "未命名" && body === "") return props.sheetId
  return null
})

watch(
  abandonableEmptySheetId,
  (v) => emit("emptyDraftChange", v),
  { immediate: true },
)

useAutoScroll({
  scrollParentRef: readerBodyRef,
  isPlaying: practicePlaying,
  scrollLevel: practiceScrollLevel,
})

const { resumeIfNeeded: practiceResumeMetronome } = useMetronome({
  bpm: practiceBpm,
  muted: practiceMetronomeMuted,
  isPlaying: practicePlaying,
  onAudioUnavailable: (r) => {
    practiceAudioWarning.value = r
  },
})

async function onPracticeTogglePlay() {
  if (editingText.value) return
  if (!practicePlaying.value) {
    await practiceResumeMetronome()
  }
  practicePlaying.value = !practicePlaying.value
}

function setPracticeBpm(v: number) {
  practiceBpm.value = v
}

function setPracticeScrollLevel(v: number) {
  practiceScrollLevel.value = v
}

function setPracticeMetronomeMuted(v: boolean) {
  practiceMetronomeMuted.value = v
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
const pdfPreviewUrls = ref<Record<string, string>>({})
let previewGen = 0

function revokePreviewBlobUrls() {
  for (const url of Object.values(previewUrls.value) as string[]) {
    if (url.startsWith("blob:")) URL.revokeObjectURL(url)
  }
  previewUrls.value = {}
  for (const url of Object.values(pdfPreviewUrls.value) as string[]) {
    if (url.startsWith("blob:")) URL.revokeObjectURL(url)
  }
  pdfPreviewUrls.value = {}
}

async function refreshTextPreviewImages() {
  const gen = ++previewGen
  revokePreviewBlobUrls()
  if (editingText.value || meta.value?.kind !== "text" || !meta.value.absolute_path) return

  const tabPath = meta.value.absolute_path
  const { images, pdfs } = listEmbedFilenames(textBody.value)
  const nextImg: Record<string, string> = {}
  const nextPdf: Record<string, string> = {}
  for (const file of [...new Set(images)]) {
    if (gen !== previewGen) return
    try {
      const abs = siblingFilePath(tabPath, file)
      const bytes = await readFile(abs)
      const blob = new Blob([bytes], { type: guessImageMime(abs) })
      nextImg[file] = URL.createObjectURL(blob)
    } catch {
      /* missing or unreadable */
    }
  }
  for (const file of [...new Set(pdfs)]) {
    if (gen !== previewGen) return
    try {
      const abs = siblingFilePath(tabPath, file)
      const bytes = await readFile(abs)
      const blob = new Blob([bytes], { type: "application/pdf" })
      nextPdf[file] = URL.createObjectURL(blob)
    } catch {
      /* missing or unreadable */
    }
  }
  if (gen !== previewGen) {
    for (const u of Object.values(nextImg)) URL.revokeObjectURL(u)
    for (const u of Object.values(nextPdf)) URL.revokeObjectURL(u)
    return
  }
  previewUrls.value = nextImg
  pdfPreviewUrls.value = nextPdf
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
  }
}

function enterBodyEdit() {
  if (!meta.value || meta.value.kind !== "text" || editingText.value) return
  textDraft.value = textBaseline.value
  editingText.value = true
  void nextTick(() => textAreaRef.value?.focus())
}

function startTitleEdit() {
  if (!meta.value) return
  titleDraft.value = meta.value.display_title
  titleEditing.value = true
  void nextTick(() => {
    titleInputRef.value?.focus()
    titleInputRef.value?.select()
  })
}

function cancelTitleEdit() {
  if (!meta.value) return
  titleDraft.value = meta.value.display_title
  titleEditing.value = false
  error.value = null
}

async function commitTitleEdit() {
  if (!titleEditing.value || !meta.value || !props.sheetId) return
  const t = titleDraft.value.trim()
  if (!t) {
    titleDraft.value = meta.value.display_title
    titleEditing.value = false
    error.value = null
    return
  }
  if (t === meta.value.display_title) {
    titleEditing.value = false
    return
  }
  error.value = null
  try {
    await invoke("rename_sheet_title", { id: props.sheetId, title: t })
    meta.value = { ...meta.value, display_title: t }
    emit("titleRenamed", { id: props.sheetId, title: t })
    titleEditing.value = false
  } catch (e) {
    error.value = String(e)
  }
}

function onTitleBlur() {
  void commitTitleEdit()
}

function onTitleEnter() {
  void commitTitleEdit()
}

async function commitTextEdit() {
  if (!editingText.value || meta.value?.kind !== "text") return
  if (textDraft.value.trim() === "") {
    textDraft.value = textBaseline.value
    editingText.value = false
    return
  }
  if (shouldConvertAsciiToChordPro(textDraft.value)) {
    textDraft.value = convertAsciiChordSheetToChordPro(textDraft.value)
  }
  if (textDraft.value !== textBaseline.value) {
    await saveTextBody()
  }
  editingText.value = false
}

function onTextBlur() {
  void commitTextEdit()
}

async function removeSheet() {
  const id = props.sheetId
  if (!id || !meta.value) return
  if (
    !(await confirmTwice(
      `删除「${meta.value.display_title}」？本地文件会一并删除。`,
      "再次确认：删除后不可恢复，确定删除吗？",
    ))
  )
    return
  try {
    await invoke("delete_sheet", { id })
    emit("deleted", id)
    meta.value = null
    textBody.value = ""
  } catch (e) {
    showToast(String(e))
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
      const snippet = await invoke<string>("save_sheet_text_attachment", {
        sheetId: props.sheetId,
        fileBase64: b64,
        extension: ext,
      })
      insertAtCursor(snippet)
    } catch (e) {
      error.value = String(e)
    }
    return
  }
}

function gatherFilesFromDataTransfer(dt: DataTransfer): File[] {
  const out: File[] = []
  if (dt.files?.length) {
    for (let i = 0; i < dt.files.length; i++) out.push(dt.files[i]!)
  }
  if (!out.length && dt.items?.length) {
    for (const it of Array.from(dt.items)) {
      if (it.kind !== "file") continue
      const f = it.getAsFile()
      if (f) out.push(f)
    }
  }
  // #region agent log
  fetch("http://127.0.0.1:7268/ingest/f8b42a76-b477-4e11-b3eb-38547a546c8e", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Debug-Session-Id": "d399f4",
    },
    body: JSON.stringify({
      sessionId: "d399f4",
      hypothesisId: "B",
      location: "SheetReaderPanel.vue:gatherFilesFromDataTransfer",
      message: "gathered files",
      data: {
        filesLen: dt.files?.length ?? 0,
        itemsLen: dt.items?.length ?? 0,
        outCount: out.length,
        names: out.map((f) => f.name),
        types: out.map((f) => f.type),
      },
      timestamp: Date.now(),
    }),
  }).catch(() => {})
  // #endregion
  return out
}

async function snippetFromDroppedFile(file: File): Promise<string | null> {
  const buf = new Uint8Array(await file.arrayBuffer())
  if (!buf.length) return null
  const head = buf.subarray(0, Math.min(8, buf.length))
  const ext = inferAttachmentExtension(file.name, file.type, head)
  // #region agent log
  fetch("http://127.0.0.1:7268/ingest/f8b42a76-b477-4e11-b3eb-38547a546c8e", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Debug-Session-Id": "d399f4",
    },
    body: JSON.stringify({
      sessionId: "d399f4",
      hypothesisId: "C",
      location: "SheetReaderPanel.vue:snippetFromDroppedFile",
      message: "after infer ext",
      data: {
        name: file.name,
        mime: file.type,
        ext,
        head0: head[0],
        head1: head[1],
        head2: head[2],
        head3: head[3],
      },
      timestamp: Date.now(),
    }),
  }).catch(() => {})
  // #endregion
  if (!ext) return null
  if (!props.sheetId) return null
  const b64 = uint8ToBase64(buf)
  return await invoke<string>("save_sheet_text_attachment", {
    sheetId: props.sheetId,
    fileBase64: b64,
    extension: ext,
  })
}

async function appendDroppedAttachments(files: File[]) {
  if (!props.sheetId || meta.value?.kind !== "text") return
  const snippets: string[] = []
  for (const file of files) {
    try {
      const s = await snippetFromDroppedFile(file)
      if (s) snippets.push(s.trimEnd())
    } catch (err) {
      // #region agent log
      fetch("http://127.0.0.1:7268/ingest/f8b42a76-b477-4e11-b3eb-38547a546c8e", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Debug-Session-Id": "d399f4",
        },
        body: JSON.stringify({
          sessionId: "d399f4",
          hypothesisId: "D",
          location: "SheetReaderPanel.vue:appendDroppedAttachments",
          message: "snippet/invoke error",
          data: { err: String(err), fileName: file.name },
          timestamp: Date.now(),
        }),
      }).catch(() => {})
      // #endregion
      error.value = String(err)
      return
    }
  }
  // #region agent log
  fetch("http://127.0.0.1:7268/ingest/f8b42a76-b477-4e11-b3eb-38547a546c8e", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Debug-Session-Id": "d399f4",
    },
    body: JSON.stringify({
      sessionId: "d399f4",
      hypothesisId: "E",
      location: "SheetReaderPanel.vue:appendDroppedAttachments",
      message: "before branch",
      data: {
        snippetsCount: snippets.length,
        editing: editingText.value,
      },
      timestamp: Date.now(),
    }),
  }).catch(() => {})
  // #endregion
  if (!snippets.length) return
  if (editingText.value) {
    for (const line of snippets) {
      insertAtCursor(line)
    }
    return
  }
  const current = textBody.value
  const sep = current.length > 0 && !current.endsWith("\n") ? "\n" : ""
  const addition = sep + snippets.join("\n") + "\n"
  textDraft.value = current + addition
  await saveTextBody()
}

function onBodyDragOver(e: DragEvent) {
  if (!props.sheetId || meta.value?.kind !== "text") return
  const types = e.dataTransfer ? [...e.dataTransfer.types] : []
  const hasFiles = e.dataTransfer?.types?.includes("Files") ?? false
  const looksLikeFile = dataTransferLooksLikeFileDrag(e.dataTransfer)
  // #region agent log
  fetch("http://127.0.0.1:7268/ingest/f8b42a76-b477-4e11-b3eb-38547a546c8e", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Debug-Session-Id": "d399f4",
    },
    body: JSON.stringify({
      sessionId: "d399f4",
      hypothesisId: "A",
      location: "SheetReaderPanel.vue:onBodyDragOver",
      message: "dragover",
      data: {
        sheetId: props.sheetId,
        kind: meta.value?.kind,
        types,
        hasFiles,
        looksLikeFile,
        willPrevent: looksLikeFile,
      },
      timestamp: Date.now(),
    }),
  }).catch(() => {})
  // #endregion
  if (looksLikeFile && e.dataTransfer) {
    e.preventDefault()
    e.dataTransfer.dropEffect = "copy"
  }
}

async function onBodyDrop(e: DragEvent) {
  if (!props.sheetId || meta.value?.kind !== "text") return
  const dt = e.dataTransfer
  if (!dt) return
  const types = [...dt.types]
  const files = gatherFilesFromDataTransfer(dt)
  // #region agent log
  fetch("http://127.0.0.1:7268/ingest/f8b42a76-b477-4e11-b3eb-38547a546c8e", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Debug-Session-Id": "d399f4",
    },
    body: JSON.stringify({
      sessionId: "d399f4",
      hypothesisId: "A",
      location: "SheetReaderPanel.vue:onBodyDrop",
      message: "drop start",
      data: {
        types,
        fileCount: files.length,
        sheetId: props.sheetId,
        kind: meta.value?.kind,
      },
      timestamp: Date.now(),
    }),
  }).catch(() => {})
  // #endregion
  if (!files.length) return
  e.preventDefault()
  await appendDroppedAttachments(files)
}

watch(
  () => props.sheetId,
  async (newId, oldId) => {
    if (oldId) await flushReaderStateNow(oldId)
    readerSettingsOpen.value = false
    if (newId) {
      await hydrateReaderForSheet(newId)
    } else {
      resetReaderStateToDefaults()
    }
    practicePlaying.value = false
    await load()
  },
  { immediate: true },
)

watch(
  () => props.reloadNonce,
  () => {
    if (props.sheetId) void load()
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
  if (props.sheetId) void flushReaderStateNow(props.sheetId)
  document.removeEventListener("keydown", onReaderSettingsEscape)
  if (readerSettingsDismiss) {
    document.removeEventListener("mousedown", readerSettingsDismiss, true)
  }
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
          <button
            type="button"
            class="title-display"
            title="点击编辑标题"
            @click="startTitleEdit"
          >
            {{ meta.display_title }}
          </button>
          <button
            v-if="variant === 'page'"
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
            @keydown.enter.prevent="onTitleEnter"
            @blur="onTitleBlur"
          />
          <button
            type="button"
            class="title-edit-cancel"
            @mousedown.prevent
            @click="cancelTitleEdit"
          >
            取消
          </button>
        </template>
      </div>
      <details v-if="meta?.kind === 'pdf'" class="reader-overflow-narrow">
        <summary class="overflow-sum" title="翻页">⋯</summary>
        <div class="overflow-panel overflow-panel-row">
          <button type="button" :disabled="pdfPage <= 1" @click="prevPdf">上一页</button>
          <span>{{ pdfPage }} / {{ pdfTotal || "…" }}</span>
          <button type="button" :disabled="!pdfDoc || pdfPage >= pdfTotal" @click="nextPdf">下一页</button>
        </div>
      </details>
    </header>

    <div class="reader-body">
      <div
        v-if="meta"
        class="practice-strip"
        :class="{ 'practice-strip--with-settings': meta.kind === 'text' }"
      >
        <PracticeToolbar
          :is-playing="practicePlaying"
          :bpm="practiceBpm"
          :scroll-level="practiceScrollLevel"
          :metronome-muted="practiceMetronomeMuted"
          :audio-warning="practiceAudioWarning"
          :disabled="editingText"
          @toggle-play="onPracticeTogglePlay"
          @update:bpm="setPracticeBpm"
          @update:scroll-level="setPracticeScrollLevel"
          @update:metronome-muted="setPracticeMetronomeMuted"
        />
        <div
          v-if="meta.kind === 'text' && !editingText"
          ref="readerSettingsSlotRef"
          class="reader-settings-slot"
        >
          <button
            type="button"
            class="reader-settings-toggle"
            :aria-expanded="readerSettingsOpen"
            aria-controls="reader-chord-settings-panel"
            @click="readerSettingsOpen = !readerSettingsOpen"
          >
            谱面设置
          </button>
          <div
            v-show="readerSettingsOpen"
            id="reader-chord-settings-panel"
            class="reader-settings-popover"
            role="region"
            aria-label="谱面设置"
          >
            <ReaderChordSettingsPanel />
          </div>
        </div>
      </div>

      <div ref="readerBodyRef" class="reader-scroll">
        <p v-if="error" class="err">{{ error }}</p>

        <div v-else-if="!sheetId" class="empty">
          <p>在左侧树中选择一首曲谱，内容将显示在这里。</p>
        </div>

        <template v-else-if="meta">
        <section v-if="meta.kind === 'text'" class="text-wrap">
          <div class="text-sheet-main">
              <p v-if="editingText" class="paste-hint">
                提示：Ctrl+V 或拖拽可插入多张图片与多个 PDF；失焦自动保存正文（空白不保存）。若使用「和弦在上、歌词在下」的文本排版，退出编辑时会自动转为
                <code>[和弦]</code> 内嵌格式。
              </p>
              <p v-else class="paste-hint paste-hint--preview">
                可将图片或 PDF 拖入此处，自动插入到正文末尾并保存（无需先点进编辑）。
              </p>
              <textarea
                v-if="editingText"
                ref="textAreaRef"
                v-model="textDraft"
                class="tab edit"
                :style="{ fontSize: fontPx + 'px', lineHeight: String(TEXT_LINE_HEIGHT) }"
                @paste="onTextPaste"
                @dragover="onBodyDragOver"
                @drop="onBodyDrop"
                @blur="onTextBlur"
              />
              <div
                v-else
                class="tab text-preview"
                role="button"
                tabindex="0"
                title="点击编辑正文；也可拖入图片或 PDF"
                :style="{ fontSize: fontPx + 'px', lineHeight: String(TEXT_LINE_HEIGHT) }"
                @click="enterBodyEdit"
                @keydown.enter.prevent="enterBodyEdit"
                @dragover="onBodyDragOver"
                @drop="onBodyDrop"
              >
                <template v-for="(seg, i) in textPreviewSegments" :key="i">
                  <ChordSheetRenderer
                    v-if="seg.type === 'text' && looksLikeChordSheet(seg.content)"
                    :source="seg.content"
                    :transpose-semitones="chordPrefs.transposeSemitones"
                    :simplify-chords="chordPrefs.simplifyChords"
                    :chord-style="chordPrefs.chordStyle"
                    :parallel-display="chordPrefs.parallelDisplay"
                  />
                  <pre
                    v-else-if="seg.type === 'text'"
                    class="text-chunk"
                  >{{ seg.content }}</pre>
                  <figure v-else-if="seg.type === 'img'" class="inline-img-wrap">
                    <img
                      v-if="previewUrls[seg.file]"
                      :src="previewUrls[seg.file]"
                      :alt="seg.file"
                      class="inline-img"
                    />
                    <p v-else class="img-missing">（无法加载图片：{{ seg.file }}）</p>
                  </figure>
                  <figure v-else-if="seg.type === 'pdf'" class="inline-pdf-wrap">
                    <embed
                      v-if="pdfPreviewUrls[seg.file]"
                      class="inline-pdf-embed"
                      :src="pdfPreviewUrls[seg.file]"
                      type="application/pdf"
                      title="内嵌 PDF"
                    />
                    <p v-else class="img-missing">（无法加载 PDF：{{ seg.file }}）</p>
                  </figure>
                </template>
              </div>
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
  </div>
</template>

<style scoped>
.reader {
  container-type: inline-size;
  container-name: reader;
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
  border-bottom: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
}
.reader-overflow-narrow {
  display: none;
  position: relative;
  margin-left: auto;
}
.overflow-sum {
  cursor: pointer;
  list-style: none;
  width: 2rem;
  text-align: center;
  font-size: 1.1rem;
  line-height: 1;
  padding: 0.25rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  color: var(--gs-text-muted);
}
.reader-overflow-narrow summary::-webkit-details-marker {
  display: none;
}
.overflow-panel {
  position: absolute;
  z-index: 5;
  margin-top: 0.25rem;
  right: 0;
  padding: 0.5rem 0.65rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
  box-shadow: var(--gs-shadow-sm);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  min-width: 12rem;
}
.overflow-panel-row {
  flex-direction: row;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  min-width: auto;
}
@container reader (max-width: 36rem) {
  .reader-overflow-narrow {
    display: block;
  }
  .pdf-wrap .pdf-controls {
    display: none;
  }
}
.title-block {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
  min-width: 0;
}
.title-display {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
  flex: 1;
  min-width: 0;
  text-align: left;
  border: none;
  background: none;
  padding: 0;
  color: inherit;
  font-family: inherit;
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.title-display:hover {
  text-decoration: underline;
  text-underline-offset: 2px;
}
.title-input {
  flex: 1;
  min-width: 8rem;
  box-sizing: border-box;
  padding: 0.4rem 0.5rem;
  font-size: 1rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
  color: inherit;
}
.title-edit-cancel {
  flex-shrink: 0;
  cursor: pointer;
  padding: 0.4rem 0.75rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  color: var(--gs-text);
  font-size: 0.9rem;
}
.ghost {
  border: none;
  background: none;
  cursor: pointer;
  color: var(--gs-link);
  font-size: 0.9rem;
  padding: 0.25rem 0.5rem;
}
.ghost.danger {
  color: var(--gs-danger);
}
.back {
  border: none;
  background: none;
  cursor: pointer;
  color: var(--gs-link);
  font-size: 1rem;
}
.practice-strip {
  flex-shrink: 0;
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: flex-start;
  gap: 0.5rem 0.75rem;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  box-shadow: 0 1px 0 color-mix(in srgb, var(--gs-border) 40%, transparent);
}
.practice-strip--with-settings {
  justify-content: space-between;
}
.practice-strip :deep(.practice-toolbar) {
  flex: 1;
  min-width: min(100%, 12rem);
}
.reader-settings-slot {
  position: relative;
  flex-shrink: 0;
  align-self: center;
}
.reader-settings-toggle {
  cursor: pointer;
  padding: 0.35rem 0.65rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: var(--gs-text);
  font-size: 0.85rem;
  white-space: nowrap;
}
.reader-settings-toggle:hover {
  border-color: var(--gs-primary-border);
  color: var(--gs-link);
}
.reader-settings-popover {
  position: absolute;
  top: calc(100% + 0.35rem);
  right: 0;
  z-index: 6;
  min-width: min(13.5rem, calc(100vw - 2rem));
  max-width: min(22rem, calc(100vw - 2rem));
  border-radius: var(--gs-radius-md);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  box-shadow: var(--gs-shadow-sm);
}
.reader-settings-popover :deep(.reader-settings) {
  width: auto;
  min-width: 0;
  border-left: none;
  border-radius: var(--gs-radius-md);
}
.reader-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.reader-scroll {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}
.err {
  color: var(--gs-danger);
  padding: 1rem;
}
.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: var(--gs-text-muted);
  font-size: 0.95rem;
}
.text-wrap {
  padding: 0.75rem 1rem 1rem;
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.text-sheet-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.paste-hint {
  margin: 0 0 0.5rem;
  font-size: 0.8rem;
  color: var(--gs-text-muted);
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
  background: var(--gs-bg-muted);
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-md);
  padding: 1rem;
  overflow: auto;
}
.tab.edit {
  resize: vertical;
  line-height: inherit;
  color: var(--gs-text);
  caret-color: var(--gs-text);
}
.text-preview {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  cursor: pointer;
  border-radius: var(--gs-radius-sm);
}
.text-preview:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--gs-primary-border) 65%, transparent);
  outline-offset: 2px;
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
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
}
.img-missing {
  margin: 0;
  font-size: 0.85rem;
  color: var(--gs-text-muted);
}
.inline-pdf-wrap {
  margin: 0;
  width: 100%;
}
.inline-pdf-embed {
  display: block;
  width: 100%;
  min-height: 14rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-muted);
}
.img-wrap {
  flex: 1 1 auto;
  padding: 1rem;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  min-height: 0;
}
.img-wrap img {
  max-width: 100%;
  height: auto;
}
.pdf-wrap {
  flex: 1 1 auto;
  padding: 1rem;
  min-height: 0;
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
  border: 1px solid var(--gs-border);
}
button {
  cursor: pointer;
  padding: 0.35rem 0.65rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
}
</style>
