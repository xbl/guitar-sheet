/** One line = one standalone image token (same as SheetReaderPanel / backend paste). */
export function parseImageEmbedLine(line: string): string | null {
  const dbl = line.match(/^\s*\{\{IMG:([^}]+)\}\}\s*$/)
  if (dbl) return dbl[1]!.trim()
  /** Legacy: Rust used to emit single-brace tokens via mistaken `format!` escaping. */
  const single = line.match(/^\s*\{IMG:([^}]+)\}\s*$/)
  if (single) return single[1]!.trim()
  const md = line.match(/^\s*!\[[^\]]*\]\(([^)]+)\)\s*$/)
  if (md) return md[1]!.trim()
  return null
}

export function parsePdfEmbedLine(line: string): string | null {
  const dbl = line.match(/^\s*\{\{PDF:([^}]+)\}\}\s*$/)
  if (dbl) return dbl[1]!.trim()
  /** Legacy: see `parseImageEmbedLine`. */
  const single = line.match(/^\s*\{PDF:([^}]+)\}\s*$/)
  if (single) return single[1]!.trim()
  return null
}

export type TextPreviewSeg =
  | { type: "text"; content: string }
  | { type: "img"; file: string }
  | { type: "pdf"; file: string }

export function buildTextPreviewSegments(body: string): TextPreviewSeg[] {
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
    const img = parseImageEmbedLine(line)
    if (img) {
      flush()
      out.push({ type: "img", file: img })
      continue
    }
    const pdf = parsePdfEmbedLine(line)
    if (pdf) {
      flush()
      out.push({ type: "pdf", file: pdf })
      continue
    }
    textBuf.push(line)
  }
  flush()
  return out
}

export function listEmbedFilenames(body: string): { images: string[]; pdfs: string[] } {
  const segs = buildTextPreviewSegments(body)
  const images: string[] = []
  const pdfs: string[] = []
  for (const s of segs) {
    if (s.type === "img") images.push(s.file)
    if (s.type === "pdf") pdfs.push(s.file)
  }
  return { images, pdfs }
}
