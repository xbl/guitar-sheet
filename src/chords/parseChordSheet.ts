export type ChordCell = { chord: string | null; lyric: string }

export type ParsedSheetLine =
  | { kind: "section"; title: string }
  | { kind: "lyric"; cells: ChordCell[] }
  | { kind: "plain"; text: string }

/** Flat layout for ChordSheetRenderer (normal vs 并行流式换行). */
export type ChordDisplayBlock =
  | { kind: "section"; title: string }
  | { kind: "gap" }
  | { kind: "plain"; text: string }
  | { kind: "lyric-line"; cells: ChordCell[] }
  | { kind: "lyric-flow"; cells: ChordCell[] }

/**
 * When `parallel` is true: merge consecutive lyric lines into one wrap-friendly flow.
 * Blank lines between chord lines are ignored (common in sources); sections and plain
 * (non-chord) lines still break the flow. Consecutive plain lines merge with spaces.
 */
export function buildChordDisplayBlocks(
  lines: ParsedSheetLine[],
  parallel: boolean,
): ChordDisplayBlock[] {
  if (!parallel) {
    const out: ChordDisplayBlock[] = []
    for (const line of lines) {
      if (line.kind === "section") {
        out.push({ kind: "section", title: line.title })
      } else if (line.kind === "plain") {
        out.push(line.text === "" ? { kind: "gap" } : { kind: "plain", text: line.text })
      } else {
        out.push({ kind: "lyric-line", cells: line.cells })
      }
    }
    return out
  }

  const out: ChordDisplayBlock[] = []
  let flow: ChordCell[] = []
  const plainParts: string[] = []

  function flushFlow() {
    if (flow.length > 0) {
      out.push({ kind: "lyric-flow", cells: flow.slice() })
      flow = []
    }
  }

  function flushPlain() {
    if (plainParts.length > 0) {
      out.push({ kind: "plain", text: plainParts.join(" ") })
      plainParts.length = 0
    }
  }

  for (const line of lines) {
    if (line.kind === "section") {
      flushFlow()
      flushPlain()
      out.push({ kind: "section", title: line.title })
      continue
    }
    if (line.kind === "plain") {
      if (line.text === "") {
        continue
      }
      flushFlow()
      plainParts.push(line.text)
      continue
    }
    flushPlain()
    flow.push(...line.cells)
  }
  flushFlow()
  flushPlain()
  return out
}

/** Heuristic: ChordPro-style [Am] markers (allow longer spellings e.g. Cmaj7(#11)) */
export function looksLikeChordSheet(source: string): boolean {
  return /\[[^\]\r\n]{1,24}\]/.test(source)
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
