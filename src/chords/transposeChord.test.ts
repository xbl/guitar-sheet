import { describe, expect, it } from "vitest"
import { transposeChordSymbol, transposeOffsetLabel } from "./transposeChord"

describe("transposeChordSymbol", () => {
  it("transposes roots", () => {
    expect(transposeChordSymbol("C", 2)).toBe("D")
    expect(transposeChordSymbol("Am", 2)).toBe("Bm")
    expect(transposeChordSymbol("G", -2)).toBe("F")
  })

  it("handles slash chords", () => {
    expect(transposeChordSymbol("Am7/G", 2)).toBe("Bm7/A")
  })

  it("no-op on zero", () => {
    expect(transposeChordSymbol("F#m", 0)).toBe("F#m")
  })
})

describe("transposeOffsetLabel", () => {
  it("maps semitone offset from C", () => {
    expect(transposeOffsetLabel(0)).toBe("C")
    expect(transposeOffsetLabel(2)).toBe("D")
  })
})
