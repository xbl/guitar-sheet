/**
 * Standard tuning E–A–D–G–B–e (low → high). Six chars: digit = fret, x/X = mute.
 * Open strings use 0.
 */
export const CHORD_SHAPE_TAB: Record<string, string> = {
  C: "x32010",
  "Cmaj7": "x32000",
  "Cm": "x35543",
  Am: "x02210",
  "Am7": "x02010",
  A: "x02220",
  "A7": "x02020",
  Asus2: "x02200",
  Asus4: "x02230",
  G: "320003",
  "G7": "320001",
  Gm: "355333",
  Em: "022000",
  "Em7": "020000",
  D: "xx0232",
  Dm: "xx0231",
  "D7": "xx0212",
  Dsus4: "xx0233",
  F: "xx3211",
  "Fmaj7": "xx3210",
  Fm: "xx3111",
  E: "022100",
  "E7": "020100",
  Bm: "x24432",
  "B7": "x21202",
  B: "x24442",
  "Bb": "x13331",
  "Bbm": "x13321",
  // enharmonic helpers
  "A#": "x13331",
  Am6: "x04222",
}

export function parseShapeTab(tab: string): (number | null)[] {
  const out: (number | null)[] = []
  const t = tab.replace(/\s/g, "").toLowerCase()
  for (let i = 0; i < t.length && out.length < 6; i++) {
    const c = t[i]!
    if (c === "x") out.push(null)
    else if (c >= "0" && c <= "9") out.push(Number(c))
  }
  while (out.length < 6) out.push(0)
  return out.slice(0, 6)
}

export function shapeForChord(normalizedSymbol: string): (number | null)[] | null {
  const tab = CHORD_SHAPE_TAB[normalizedSymbol]
  if (!tab) return null
  return parseShapeTab(tab)
}
