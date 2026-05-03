/**
 * Whether a drag operation likely carries files. WebKit / Tauri often omits the
 * `"Files"` string from `dataTransfer.types` on `dragover` while still exposing
 * `DataTransferItem` with `kind === "file"` — without `preventDefault` on dragover,
 * the host never delivers `drop`.
 */
export function dataTransferLooksLikeFileDrag(
  dt: DataTransfer | null | undefined,
): boolean {
  if (!dt) return false
  if (dt.types?.includes("Files")) return true
  for (const it of Array.from(dt.items ?? [])) {
    if (it.kind === "file") return true
  }
  return false
}

/** First bytes of a PDF file (ASCII `%PDF`). */
export function isPdfMagicHeader(bytes: Uint8Array): boolean {
  if (bytes.length < 4) return false
  return (
    bytes[0] === 0x25 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x44 &&
    bytes[3] === 0x46
  )
}

/**
 * Infer attachment extension for `save_sheet_text_attachment`.
 * macOS / Tauri WebKit often yields empty `type` for dragged PDFs — use filename first, then MIME, then magic.
 */
export function inferAttachmentExtension(
  fileName: string,
  mimeType: string,
  leadingBytes?: Uint8Array,
): string | null {
  const ext = fileName.split(".").pop()?.toLowerCase().trim()
  if (ext === "pdf") return "pdf"
  if (ext === "jpeg" || ext === "jpg") return "jpg"
  if (ext === "png" || ext === "webp" || ext === "gif") return ext

  const t = (mimeType || "").toLowerCase()
  if (t === "application/pdf" || t.includes("pdf")) return "pdf"
  if (t === "image/png") return "png"
  if (t === "image/jpeg") return "jpg"
  if (t === "image/webp") return "webp"
  if (t === "image/gif") return "gif"

  if (leadingBytes && isPdfMagicHeader(leadingBytes)) return "pdf"

  return null
}
