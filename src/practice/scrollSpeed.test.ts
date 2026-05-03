import { describe, expect, it } from "vitest"
import { SCROLL_LEVEL_MAX, SCROLL_LEVEL_MIN } from "./constants"
import { clampScrollLevel, scrollLevelToPixelsPerSecond } from "./scrollSpeed"

describe("clampScrollLevel", () => {
  it("clamps to 1–SCROLL_LEVEL_MAX and rounds", () => {
    expect(clampScrollLevel(0)).toBe(1)
    expect(clampScrollLevel(SCROLL_LEVEL_MAX + 1)).toBe(SCROLL_LEVEL_MAX)
    expect(clampScrollLevel(7.6)).toBe(8)
  })
})

describe("scrollLevelToPixelsPerSecond", () => {
  it("is monotonic and maps 1→10 and max→200 px/s", () => {
    expect(scrollLevelToPixelsPerSecond(1)).toBeCloseTo(10, 5)
    expect(scrollLevelToPixelsPerSecond(SCROLL_LEVEL_MAX)).toBeCloseTo(200, 5)
    expect(scrollLevelToPixelsPerSecond(10)).toBeGreaterThan(
      scrollLevelToPixelsPerSecond(9),
    )
  })

  it("uses finer linear steps (40 levels) so low-mid adjacent deltas are smaller", () => {
    const d = scrollLevelToPixelsPerSecond(10) - scrollLevelToPixelsPerSecond(9)
    expect(d).toBeLessThan(6)
    expect(d).toBeCloseTo(190 / (SCROLL_LEVEL_MAX - SCROLL_LEVEL_MIN), 5)
  })
})
