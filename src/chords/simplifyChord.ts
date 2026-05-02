/**
 * Display-only simplification (extensions → easier shapes). Conservative rules.
 */
export function simplifyChordSymbol(symbol: string): string {
  let s = symbol.trim()
  if (!s) return s
  // Drop common extensions / alterations for beginner view
  s = s.replace(/maj7|maj9|Maj7|M7\b/gi, "")
  s = s.replace(/m7(?![a-z])/gi, "m")
  s = s.replace(/add9|add11/gi, "")
  s = s.replace(/sus2|sus4/gi, "")
  s = s.replace(/dim7?|°/gi, "dim")
  s = s.replace(/aug|\+(?![0-9])/gi, "")
  // collapse double spaces from removals
  return s.replace(/\s+/g, "").replace(/^(\[[^\]]+\])$/, "$1")
}
