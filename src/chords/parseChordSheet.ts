export type ChordCell = { chord: string | null; lyric: string }

export type ParsedSheetLine =
  | { kind: "section"; title: string }
  | { kind: "lyric"; cells: ChordCell[] }
  | { kind: "plain"; text: string }

/** Heuristic: ChordPro-style [Am] markers */
export function looksLikeChordSheet(source: string): boolean {
  return /\[[^\]\r\n]{1,12}\]/.test(source)
}

/** Section: markdown ## title or 【标题】 */
function parseSectionLine(trimmed: string): string | null {
  const md = trimmed.match(/^#{2,}\s*(.+)$/)
  if (md) return md[1]!.trim()
  const cn = trimmed.match(/^【(.+)】$/)
  if (cn) return cn[1]!.trim()
  return null
}

/**
 * One lyric line with optional leading chord markers.
 * Text before first `[` is plain prefix (no chord).
 */
export function parseLyricLine(line: string): ChordCell[] {
  const cells: ChordCell[] = []
  let i = 0
  while (i < line.length) {
    if (line[i] === "[") {
      const close = line.indexOf("]", i + 1)
      if (close === -1) {
        const rest = line.slice(i)
        if (rest) mergePlain(cells, rest)
        break
      }
      const chordRaw = line.slice(i + 1, close).trim()
      i = close + 1
      let j = i
      while (j < line.length && line[j] !== "[") j++
      const lyric = line.slice(i, j)
      cells.push({
        chord: chordRaw || null,
        lyric,
      })
      i = j
    } else {
      let j = i
      while (j < line.length && line[j] !== "[") j++
      const lyric = line.slice(i, j)
      mergePlain(cells, lyric)
      i = j
    }
  }
  return cells
}

function mergePlain(cells: ChordCell[], lyric: string) {
  if (!lyric) return
  const last = cells[cells.length - 1]
  if (last && last.chord === null) last.lyric += lyric
  else cells.push({ chord: null, lyric })
}

export function parseChordSheet(source: string): ParsedSheetLine[] {
  const lines = source.replace(/\r\n/g, "\n").split("\n")
  const out: ParsedSheetLine[] = []

  for (const raw of lines) {
    const trimmed = raw.trimEnd()
    if (trimmed === "") {
      out.push({ kind: "plain", text: "" })
      continue
    }

    const sectionTitle = parseSectionLine(trimmed.trim())
    if (sectionTitle) {
      out.push({ kind: "section", title: sectionTitle })
      continue
    }

    if (!trimmed.includes("[")) {
      out.push({ kind: "plain", text: trimmed })
      continue
    }

    const cells = parseLyricLine(trimmed)
    if (cells.length === 0) out.push({ kind: "plain", text: trimmed })
    else out.push({ kind: "lyric", cells })
  }

  return out
}
