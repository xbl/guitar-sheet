import { describe, expect, it } from "vitest"
import {
  buildTextPreviewSegments,
  parseImageEmbedLine,
  parsePdfEmbedLine,
} from "./textSheetEmbeds"

describe("textSheetEmbeds", () => {
  it("parses image and markdown image lines", () => {
    expect(parseImageEmbedLine("  {{IMG:a.png}}  ")).toBe("a.png")
    expect(parseImageEmbedLine("![](rel/x.jpg)")).toBe("rel/x.jpg")
    expect(parseImageEmbedLine("not only img")).toBeNull()
  })

  it("parses PDF embed line", () => {
    expect(parsePdfEmbedLine("{{PDF:doc.pdf}}")).toBe("doc.pdf")
    expect(parsePdfEmbedLine("{PDF:doc.pdf}")).toBe("doc.pdf")
    expect(parsePdfEmbedLine("x")).toBeNull()
  })

  it("parses legacy single-brace IMG line", () => {
    expect(parseImageEmbedLine("{IMG:a.png}")).toBe("a.png")
  })

  it("builds segments with img and pdf in order", () => {
    const body = "intro\n{{IMG:i.png}}\n{{PDF:d.pdf}}\nout"
    const segs = buildTextPreviewSegments(body)
    expect(segs).toEqual([
      { type: "text", content: "intro" },
      { type: "img", file: "i.png" },
      { type: "pdf", file: "d.pdf" },
      { type: "text", content: "out" },
    ])
  })

  it("builds pdf segment for legacy single-brace token", () => {
    const segs = buildTextPreviewSegments("a\n{PDF:x.pdf}\nb")
    expect(segs).toEqual([
      { type: "text", content: "a" },
      { type: "pdf", file: "x.pdf" },
      { type: "text", content: "b" },
    ])
  })
})
