import { shapeForChord } from "./chordShapes"
import { normalizeChordSymbol } from "./normalizeChord"

export type ChordLayout =
  | {
      kind: "grid"
      frets: (number | null)[]
      baseFret: number
      dots: { stringIndex: number; fret: number }[]
      opens: number[]
      mutes: number[]
    }
  | { kind: "unknown" }

function pressedFrets(frets: (number | null)[]): { stringIndex: number; fret: number }[] {
  const out: { stringIndex: number; fret: number }[] = []
  for (let s = 0; s < 6; s++) {
    const f = frets[s]
    if (f !== null && f !== undefined && f > 0) {
      out.push({ stringIndex: s, fret: f })
    }
  }
  return out
}

/** Layout for SVG: base fret window + dots (absolute fret numbers on strings). */
export function buildChordLayout(chordName: string): ChordLayout {
  const key = normalizeChordSymbol(chordName)
  const frets = shapeForChord(key) ?? shapeForChord(chordName)
  if (!frets) return { kind: "unknown" }

  const pressed = pressedFrets(frets)
  if (pressed.length === 0) return { kind: "unknown" }

  const minF = Math.min(...pressed.map((p) => p.fret))
  const maxF = Math.max(...pressed.map((p) => p.fret))

  let baseFret = 1
  if (minF > 1 || maxF > 4) {
    baseFret = minF
  }
  if (maxF - baseFret > 3) {
    baseFret = maxF - 3
  }

  const dots = [...pressed]
  const opens: number[] = []
  const mutes: number[] = []
  for (let s = 0; s < 6; s++) {
    const f = frets[s] ?? null
    if (f === null) mutes.push(s)
    else if (f === 0) opens.push(s)
  }

  return {
    kind: "grid",
    frets,
    baseFret,
    dots,
    opens,
    mutes,
  }
}
