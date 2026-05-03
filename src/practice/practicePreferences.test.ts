import { afterEach, describe, expect, it } from "vitest"
import {
  loadPracticePreferences,
  normalizePracticePreferences,
  savePracticePreferences,
} from "./practicePreferences"

function mockStorage() {
  const m = new Map<string, string>()
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => {
      m.set(k, v)
    },
  } as Storage
}

describe("practicePreferences", () => {
  afterEach(() => {
    localStorage.clear()
  })

  it("round-trips defaults when empty", () => {
    const s = mockStorage()
    const a = loadPracticePreferences(s)
    expect(a.bpm).toBe(120)
    savePracticePreferences(s, { ...a, bpm: 90 })
    const b = loadPracticePreferences(s)
    expect(b.bpm).toBe(90)
  })

  it("normalizePracticePreferences clamps and fills defaults", () => {
    expect(normalizePracticePreferences({ bpm: 300, scrollLevel: 99 })).toEqual({
      bpm: 240,
      scrollLevel: 40,
      metronomeMuted: false,
    })
    expect(normalizePracticePreferences({ metronomeMuted: true })).toEqual({
      bpm: 120,
      scrollLevel: 20,
      metronomeMuted: true,
    })
  })
})
