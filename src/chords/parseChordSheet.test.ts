import { describe, expect, it } from "vitest"
import {
  looksLikeChordSheet,
  parseChordSheet,
  parseLyricLine,
} from "./parseChordSheet"

describe("looksLikeChordSheet", () => {
  it("detects bracket chords", () => {
    expect(looksLikeChordSheet("[C]你好")).toBe(true)
    expect(looksLikeChordSheet("plain line")).toBe(false)
  })
})

describe("parseLyricLine", () => {
  it("parses ChordPro segments", () => {
    const cells = parseLyricLine("[C]在这个[G]晚上")
    expect(cells).toHaveLength(2)
    expect(cells[0]).toMatchObject({ chord: "C", lyric: "在这个" })
    expect(cells[1]).toMatchObject({ chord: "G", lyric: "晚上" })
  })

  it("handles leading plain text", () => {
    const cells = parseLyricLine("前[C]缀")
    expect(cells[0]).toMatchObject({ chord: null, lyric: "前" })
    expect(cells[1]).toMatchObject({ chord: "C", lyric: "缀" })
  })
})

describe("parseChordSheet", () => {
  it("parses sections and blank lines", () => {
    const src = "## 主歌\n\n[C]一行\n普通"
    const lines = parseChordSheet(src)
    expect(lines[0]).toMatchObject({ kind: "section", title: "主歌" })
    expect(lines[1]).toMatchObject({ kind: "plain", text: "" })
    expect(lines[2].kind).toBe("lyric")
    expect(lines[3].kind).toBe("plain")
  })

  it("parses 【section】", () => {
    const lines = parseChordSheet("【副歌】\n[G]副")
    expect(lines[0]).toMatchObject({ kind: "section", title: "副歌" })
  })
})
