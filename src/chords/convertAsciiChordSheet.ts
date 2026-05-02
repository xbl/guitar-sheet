/**
 * Match common guitar chord symbols in ASCII sheets (Chinese TAB sites).
 * Examples: C, Am, Em, G7, Fmaj7, D/F#, Asus4, Bb
 */
export const ASCII_CHORD_TOKEN =
  /([A-G][#b♯♭]?(?:maj7|maj9|m7|sus[24]|dim|aug|add\d+|m|7|9|11|13|6|\d+)?(?:\/[A-G][#b♯♭]?)?)/gi

export type ChordMatch = { name: string; index: number }

export function extractChordMatches(line: string): ChordMatch[] {
  const re = new RegExp(ASCII_CHORD_TOKEN.source, "gi")
  const out: ChordMatch[] = []
  let m: RegExpExecArray | null
  while ((m = re.exec(line)) !== null) {
    out.push({ name: m[1]!, index: m.index })
  }
  return out
}

/** Line contains only chord tokens, whitespace, and harmless punctuation. */
export function isChordOnlyLine(line: string): boolean {
  const trimmed = line.trim()
  if (!trimmed) return false
  const without = line.replace(new RegExp(ASCII_CHORD_TOKEN.source, "gi"), "")
  const rest = without.replace(/\s+/g, "").replace(/[_|·．·]/g, "")
  if (rest.length > 0) return false
  return extractChordMatches(line).length > 0
}

/** Heuristic: lyric line (Chinese/Latin), not a chord-only row. */
export function isLyricLine(line: string): boolean {
  const t = line.trim()
  if (!t) return false
  if (isChordOnlyLine(line)) return false
  return /[\u4e00-\u9fff]|[a-zA-Z]{2,}/.test(t)
}

export function pairChordLineToChordPro(chordLine: string, lyricLine: string): string {
  const chords = extractChordMatches(chordLine)
  const lyric = lyricLine.replace(/\r/g, "")
  if (chords.length === 0) return lyric
  if (chords.length === 1) {
    return `[${chords[0]!.name}]${lyric}`
  }

  const L = lyric.length
  const N = Math.max(chordLine.length, 1)
  let result = ""
  for (let i = 0; i < chords.length; i++) {
    const start = chords[i]!.index
    const nextStart = i + 1 < chords.length ? chords[i + 1]!.index : N
    const s = Math.floor((start / N) * L)
    const e = Math.floor((nextStart / N) * L)
    result += `[${chords[i]!.name}]${lyric.slice(s, e)}`
  }
  return result
}

export function hasAsciiChordLyricPairs(lines: string[]): boolean {
  for (let i = 0; i < lines.length - 1; i++) {
    if (isChordOnlyLine(lines[i]!) && isLyricLine(lines[i + 1]!)) {
      return true
    }
  }
  return false
}

/** Avoid clobbering hand-written ChordPro (`[C]`). */
export function shouldConvertAsciiToChordPro(raw: string): boolean {
  const lines = raw.replace(/\r\n/g, "\n").split("\n")
  if (!hasAsciiChordLyricPairs(lines)) return false
  const chordProLines = lines.filter((l) => /\[[A-G]/.test(l)).length
  if (lines.length > 0 && chordProLines / lines.length > 0.35) {
    return false
  }
  return true
}

/**
 * Convert "chord row above lyric row" ASCII layout to inline `[Chord]` format.
 * Optional leading title: first line is non-chord text, then blank optional, then pairs.
 */
export function convertAsciiChordSheetToChordPro(raw: string): string {
  const lines = raw.replace(/\r\n/g, "\n").split("\n")
  const out: string[] = []
  let i = 0

  while (i < lines.length) {
    // Leading title, optional blank, chord line, lyric line
    if (
      i === 0 &&
      lines[0] !== undefined &&
      lines[0].trim() !== "" &&
      !isChordOnlyLine(lines[0]) &&
      lines[1] === "" &&
      lines[2] !== undefined &&
      isChordOnlyLine(lines[2]) &&
      lines[3] !== undefined &&
      isLyricLine(lines[3])
    ) {
      out.push(`## ${lines[0]!.trim()}`)
      out.push(pairChordLineToChordPro(lines[2]!, lines[3]!))
      i = 4
      continue
    }

    // Leading title directly followed by chord + lyric (no blank)
    if (
      i === 0 &&
      lines[0] !== undefined &&
      lines[0].trim() !== "" &&
      !isChordOnlyLine(lines[0]) &&
      lines[1] !== undefined &&
      isChordOnlyLine(lines[1]) &&
      lines[2] !== undefined &&
      isLyricLine(lines[2])
    ) {
      out.push(`## ${lines[0]!.trim()}`)
      out.push(pairChordLineToChordPro(lines[1]!, lines[2]!))
      i = 3
      continue
    }

    const line = lines[i]!
    const next = lines[i + 1]
    if (next !== undefined && isChordOnlyLine(line) && isLyricLine(next)) {
      out.push(pairChordLineToChordPro(line, next))
      i += 2
      continue
    }

    out.push(line)
    i += 1
  }

  return out.join("\n")
}
