import { describe, expect, it } from "vitest"
import {
  convertAsciiChordSheetToChordPro,
  isChordOnlyLine,
  pairChordLineToChordPro,
  shouldConvertAsciiToChordPro,
} from "./convertAsciiChordSheet"

describe("isChordOnlyLine", () => {
  it("accepts spaced chords", () => {
    expect(isChordOnlyLine("Em               Em")).toBe(true)
    expect(isChordOnlyLine("D Em")).toBe(true)
  })
  it("rejects lyric mix", () => {
    expect(isChordOnlyLine("昨天今天")).toBe(false)
  })
})

describe("pairChordLineToChordPro", () => {
  it("wraps single chord", () => {
    expect(pairChordLineToChordPro("Em", "昨天今天")).toBe("[Em]昨天今天")
  })
})

describe("convertAsciiChordSheetToChordPro", () => {
  it("converts sample blocks", () => {
    const src = `一生所爱

Em               Em
昨天今天过去不再回来
Em               G
红颜落下色彩变苍白`

    const out = convertAsciiChordSheetToChordPro(src)
    expect(out).toContain("## 一生所爱")
    expect(out).toMatch(/\[Em\]/)
    expect(out).toMatch(/\[G\]/)
    expect(shouldConvertAsciiToChordPro(src)).toBe(true)
  })

  it("does not claim ChordPro-heavy text", () => {
    const src = "[C]你好\n[D]世界\n[E]测试\n[F]四行"
    expect(shouldConvertAsciiToChordPro(src)).toBe(false)
  })
})
