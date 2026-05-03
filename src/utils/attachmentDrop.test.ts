import { describe, expect, it } from "vitest"
import {
  dataTransferLooksLikeFileDrag,
  inferAttachmentExtension,
  isPdfMagicHeader,
} from "./attachmentDrop"

describe("isPdfMagicHeader", () => {
  it("detects %PDF prefix", () => {
    expect(isPdfMagicHeader(new Uint8Array([37, 80, 68, 70, 45]))).toBe(true)
    expect(isPdfMagicHeader(new Uint8Array([37, 80, 68, 70]))).toBe(true)
    expect(isPdfMagicHeader(new Uint8Array([1, 2, 3]))).toBe(false)
  })
})

describe("dataTransferLooksLikeFileDrag", () => {
  it("is true when types include Files", () => {
    const dt = { types: ["text/plain", "Files"], items: [] } as unknown as DataTransfer
    expect(dataTransferLooksLikeFileDrag(dt)).toBe(true)
  })

  it("is true when a file item exists but Files is not in types (WebKit-style)", () => {
    const dt = {
      types: ["text/uri-list"],
      items: [{ kind: "file" as const, getAsFile: () => null }],
    } as unknown as DataTransfer
    expect(dataTransferLooksLikeFileDrag(dt)).toBe(true)
  })

  it("is false for non-file drags", () => {
    const dt = {
      types: ["text/plain"],
      items: [{ kind: "string" as const, getAsFile: () => null }],
    } as unknown as DataTransfer
    expect(dataTransferLooksLikeFileDrag(dt)).toBe(false)
  })
})

describe("inferAttachmentExtension", () => {
  it("prefers filename over empty mime for pdf", () => {
    expect(inferAttachmentExtension("a.PDF", "", undefined)).toBe("pdf")
    expect(inferAttachmentExtension("scan.pdf", "", undefined)).toBe("pdf")
  })

  it("uses mime when name has no extension", () => {
    expect(inferAttachmentExtension("blob", "application/pdf", undefined)).toBe(
      "pdf",
    )
    expect(inferAttachmentExtension("", "image/png", undefined)).toBe("png")
  })

  it("uses magic when name and mime are empty", () => {
    const hdr = new Uint8Array([37, 80, 68, 70, 45, 49, 46, 52])
    expect(inferAttachmentExtension("", "", hdr)).toBe("pdf")
  })
})
