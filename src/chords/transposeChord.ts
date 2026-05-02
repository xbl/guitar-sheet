/** Semitone indices from C, sharp spelling preferred for display. */
const SEMI_SHARP = [
  "C",
  "C#",
  "D",
  "D#",
  "E",
  "F",
  "F#",
  "G",
  "G#",
  "A",
  "A#",
  "B",
] as const

const NOTE_ALIASES: Record<string, number> = {
  C: 0,
  "C#": 1,
  Db: 1,
  D: 2,
  "D#": 3,
  Eb: 3,
  E: 4,
  F: 5,
  "F#": 6,
  Gb: 6,
  G: 7,
  "G#": 8,
  Ab: 8,
  A: 9,
  "A#": 10,
  Bb: 10,
  B: 11,
}

export function noteToSemitone(note: string): number | null {
  const n = note.trim()
  const m = n.match(/^([A-G])([#b♯♭]?)$/i)
  if (!m) return null
  const base = m[1]!.toUpperCase()
  let acc = m[2] || ""
  if (acc === "♯") acc = "#"
  if (acc === "♭") acc = "b"
  const key = `${base}${acc}` as keyof typeof NOTE_ALIASES
  return NOTE_ALIASES[key] ?? null
}

export function semitoneToSharpNote(semi: number): string {
  const s = ((semi % 12) + 12) % 12
  return SEMI_SHARP[s]!
}

/** Label like reference「移调: D」— offset from written pitch shown as key (C + n semitones). */
export function transposeOffsetLabel(semitones: number): string {
  return semitoneToSharpNote(semitones)
}

function splitSlash(symbol: string): { main: string; bass: string | null } {
  const i = symbol.indexOf("/")
  if (i === -1) return { main: symbol, bass: null }
  return { main: symbol.slice(0, i), bass: symbol.slice(i + 1).trim() || null }
}

function transposeSingleNote(note: string, semi: number): string {
  const s = noteToSemitone(note)
  if (s === null) return note
  return semitoneToSharpNote(s + semi)
}

/**
 * Transpose a chord symbol by semitones (display). Handles slash bass, e.g. `Am7/G`.
 */
export function transposeChordSymbol(symbol: string, semitones: number): string {
  if (semitones === 0) return symbol
  const { main, bass } = splitSlash(symbol.trim())
  const mainOut = transposeMainSymbol(main, semitones)
  if (!bass) return mainOut
  const bassOut = transposeSingleNote(bass, semitones)
  return `${mainOut}/${bassOut}`
}

function transposeMainSymbol(main: string, semi: number): string {
  const m = main.match(/^([A-G])([#b♯♭]?)(.*)$/i)
  if (!m) return main
  const base = m[1]!.toUpperCase()
  let acc = m[2] || ""
  if (acc === "♯") acc = "#"
  if (acc === "♭") acc = "b"
  const key = `${base}${acc}` as keyof typeof NOTE_ALIASES
  const s0 = NOTE_ALIASES[key]
  if (s0 === undefined) return main
  const root = semitoneToSharpNote(s0 + semi)
  const suffix = m[3] ?? ""
  return `${root}${suffix}`
}
