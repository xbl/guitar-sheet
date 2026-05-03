import { describe, expect, it } from "vitest"
import {
  applyScrollDelta,
  isAtScrollBottom,
  maxScrollTop,
} from "./scrollGeometry"

describe("maxScrollTop", () => {
  it("returns 0 when content fits", () => {
    expect(maxScrollTop(100, 120)).toBe(0)
  })
  it("returns overflow when content taller than viewport", () => {
    expect(maxScrollTop(500, 100)).toBe(400)
  })
})

describe("isAtScrollBottom", () => {
  it("returns true within epsilon of bottom", () => {
    expect(isAtScrollBottom(398, 500, 100, 3)).toBe(true)
  })
  it("returns false when not near bottom", () => {
    expect(isAtScrollBottom(0, 500, 100)).toBe(false)
  })
})

describe("applyScrollDelta", () => {
  it("clamps to max scroll top", () => {
    const r = applyScrollDelta({
      scrollTop: 390,
      scrollHeight: 500,
      clientHeight: 100,
      deltaY: 50,
    })
    expect(r.nextScrollTop).toBe(400)
    expect(r.reachedBottom).toBe(true)
  })

  it("does not treat non-scrollable content as bottom", () => {
    const r = applyScrollDelta({
      scrollTop: 0,
      scrollHeight: 400,
      clientHeight: 400,
      deltaY: 10,
    })
    expect(r.nextScrollTop).toBe(0)
    expect(r.reachedBottom).toBe(false)
  })
})
