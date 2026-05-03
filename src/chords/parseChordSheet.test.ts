import { describe, expect, it } from "vitest"
import {
  buildChordDisplayBlocks,
  expandChordAnchorsForFirstChar,
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

describe("expandChordAnchorsForFirstChar", () => {
  it("splits each chord segment so only first char keeps the chord", () => {
    const raw = parseLyricLine("[Em]昨天今天过去不再[Em]回来")
    const cells = expandChordAnchorsForFirstChar(raw)
    expect(cells).toEqual([
      { chord: "Em", lyric: "昨" },
      { chord: null, lyric: "天今天过去不再" },
      { chord: "Em", lyric: "回" },
      { chord: null, lyric: "来" },
    ])
  })

  it("leaves single-char segments unchanged", () => {
    expect(expandChordAnchorsForFirstChar(parseLyricLine("[C]你"))).toEqual([
      { chord: "C", lyric: "你" },
    ])
  })
})

describe("parseChordSheet", () => {
  it("anchors chords to first character in sheet parse", () => {
    const lines = parseChordSheet("[Em]昨天今天过去不再[Em]回来")
    expect(lines[0]).toMatchObject({
      kind: "lyric",
      cells: [
        { chord: "Em", lyric: "昨" },
        { chord: null, lyric: "天今天过去不再" },
        { chord: "Em", lyric: "回" },
        { chord: null, lyric: "来" },
      ],
    })
  })

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

describe("buildChordDisplayBlocks", () => {
  it("maps non-parallel to lyric-line blocks", () => {
    const lines = parseChordSheet("[C]a\n[D]b")
    const blocks = buildChordDisplayBlocks(lines, false)
    expect(blocks).toHaveLength(2)
    expect(blocks[0]).toMatchObject({ kind: "lyric-line" })
    expect(blocks[1]).toMatchObject({ kind: "lyric-line" })
  })

  it("merges consecutive lyric lines in parallel mode", () => {
    const lines = parseChordSheet("[C]一行\n[G]二行")
    const blocks = buildChordDisplayBlocks(lines, true)
    expect(blocks).toHaveLength(1)
    expect(blocks[0]?.kind).toBe("lyric-flow")
    expect(
      blocks[0]?.kind === "lyric-flow" ? blocks[0].cells.length : 0,
    ).toBeGreaterThanOrEqual(2)
  })

  it("merges lyric lines across blank lines in parallel mode", () => {
    const lines = parseChordSheet("[C]a\n\n[D]b")
    const blocks = buildChordDisplayBlocks(lines, true)
    expect(blocks).toHaveLength(1)
    expect(blocks[0]).toMatchObject({ kind: "lyric-flow" })
  })

  it("merges consecutive plain lines in parallel mode", () => {
    const lines = parseChordSheet("hello\nworld")
    const blocks = buildChordDisplayBlocks(lines, true)
    expect(blocks).toHaveLength(1)
    expect(blocks[0]).toMatchObject({ kind: "plain", text: "hello world" })
  })
})
