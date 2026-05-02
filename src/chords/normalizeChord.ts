/** Normalize user-facing chord symbol for shape lookup (ASCII). Slash chords use part before `/`. */
export function normalizeChordSymbol(raw: string): string {
  const s = raw.trim().replace(/\s+/g, "")
  if (!s) return ""
  const main = s.includes("/") ? s.slice(0, s.indexOf("/")) : s
  const m = main.match(/^([A-Ga-g])([#b♯♭]?)(.*)$/)
  if (!m) return s
  const root = m[1]!.toUpperCase()
  let acc = m[2] || ""
  if (acc === "♯") acc = "#"
  if (acc === "♭") acc = "b"
  let suf = (m[3] || "").trim()
  if (/^m(in|i)?$/i.test(suf)) suf = "m"
  return `${root}${acc}${suf}`
}
