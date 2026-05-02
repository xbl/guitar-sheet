import { describe, expect, it } from "vitest"
import { clampScrollLevel, scrollLevelToPixelsPerSecond } from "./scrollSpeed"

describe("clampScrollLevel", () => {
  it("clamps to 1–20 and rounds", () => {
    expect(clampScrollLevel(0)).toBe(1)
    expect(clampScrollLevel(21)).toBe(20)
    expect(clampScrollLevel(7.6)).toBe(8)
  })
})

describe("scrollLevelToPixelsPerSecond", () => {
  it("is monotonic and maps 1→10 and 20→200 px/s", () => {
    expect(scrollLevelToPixelsPerSecond(1)).toBeCloseTo(10, 5)
    expect(scrollLevelToPixelsPerSecond(20)).toBeCloseTo(200, 5)
    expect(scrollLevelToPixelsPerSecond(10)).toBeGreaterThan(
      scrollLevelToPixelsPerSecond(9),
    )
  })
})
