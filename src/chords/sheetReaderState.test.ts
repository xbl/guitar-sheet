import { afterEach, describe, expect, it } from "vitest"
import { saveReaderChordPrefs } from "./readerPrefs"
import {
  loadSheetReaderStoredState,
  PANEL_NOTES_MAX_LEN,
  parseSheetReaderStoredStateJson,
  saveSheetReaderStoredState,
  serializeSheetReaderStoredState,
  sheetReaderStateStorageKey,
} from "./sheetReaderState"
import {
  STORAGE_KEY_BPM,
  STORAGE_KEY_METRONOME_MUTED,
  STORAGE_KEY_SCROLL_LEVEL,
} from "../practice/constants"

function mockStorage() {
  const m = new Map<string, string>()
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => {
      m.set(k, v)
    },
  } as Storage
}

describe("sheetReaderState", () => {
  afterEach(() => {
    localStorage.clear()
  })

  it("uses per-sheet key", () => {
    expect(sheetReaderStateStorageKey("abc")).toBe(
      "guitar-sheet.sheetReaderState.v1.abc",
    )
  })

  it("round-trips chord and practice fields", () => {
    const s = mockStorage()
    saveSheetReaderStoredState(s, "s1", {
      chord: {
        transposeSemitones: 2,
        capoFret: 3,
        zoomLevel: 2,
        chordStyle: "text",
        simplifyChords: true,
        parallelDisplay: true,
      },
      practice: { bpm: 88, scrollLevel: 15, metronomeMuted: true },
      panelNotes: "第一段慢练",
    })
    const got = loadSheetReaderStoredState(s, "s1")
    expect(got.chord.transposeSemitones).toBe(2)
    expect(got.chord.capoFret).toBe(3)
    expect(got.chord.zoomLevel).toBe(2)
    expect(got.chord.chordStyle).toBe("text")
    expect(got.chord.simplifyChords).toBe(true)
    expect(got.chord.parallelDisplay).toBe(true)
    expect(got.practice.bpm).toBe(88)
    expect(got.practice.scrollLevel).toBe(15)
    expect(got.practice.metronomeMuted).toBe(true)
    expect(got.panelNotes).toBe("第一段慢练")
  })

  it("isolates prefs between two sheet ids", () => {
    const s = mockStorage()
    saveSheetReaderStoredState(s, "a", {
      chord: {
        transposeSemitones: -1,
        capoFret: 0,
        zoomLevel: 0,
        chordStyle: "diagram",
        simplifyChords: false,
        parallelDisplay: false,
      },
      practice: { bpm: 100, scrollLevel: 5, metronomeMuted: false },
      panelNotes: "A 谱",
    })
    saveSheetReaderStoredState(s, "b", {
      chord: {
        transposeSemitones: 5,
        capoFret: 2,
        zoomLevel: 1,
        chordStyle: "diagram",
        simplifyChords: true,
        parallelDisplay: false,
      },
      practice: { bpm: 140, scrollLevel: 18, metronomeMuted: true },
      panelNotes: "B 谱",
    })
    const ga = loadSheetReaderStoredState(s, "a")
    const gb = loadSheetReaderStoredState(s, "b")
    expect(ga.practice.scrollLevel).toBe(5)
    expect(gb.practice.scrollLevel).toBe(18)
    expect(ga.chord.transposeSemitones).toBe(-1)
    expect(gb.chord.transposeSemitones).toBe(5)
    expect(ga.panelNotes).toBe("A 谱")
    expect(gb.panelNotes).toBe("B 谱")
  })

  it("migrates from legacy global keys when per-sheet blob missing", () => {
    const s = mockStorage()
    s.setItem(STORAGE_KEY_BPM, "92")
    s.setItem(STORAGE_KEY_SCROLL_LEVEL, "7")
    s.setItem(STORAGE_KEY_METRONOME_MUTED, "1")
    saveReaderChordPrefs({
      transposeSemitones: 1,
      capoFret: 4,
      zoomLevel: 2,
      chordStyle: "text",
      simplifyChords: true,
      parallelDisplay: false,
    })
    const got = loadSheetReaderStoredState(s, "new-sheet")
    expect(got.practice.bpm).toBe(92)
    expect(got.practice.scrollLevel).toBe(7)
    expect(got.practice.metronomeMuted).toBe(true)
    expect(got.chord.transposeSemitones).toBe(1)
    expect(got.chord.capoFret).toBe(4)
    expect(got.panelNotes).toBe("")
  })

  it("parseSheetReaderStoredStateJson returns null for invalid", () => {
    expect(parseSheetReaderStoredStateJson("")).toBeNull()
    expect(parseSheetReaderStoredStateJson("{")).toBeNull()
  })

  it("serializeSheetReaderStoredState and parse round-trip", () => {
    const s = mockStorage()
    const state = loadSheetReaderStoredState(s, "z")
    const raw = serializeSheetReaderStoredState(state)
    const back = parseSheetReaderStoredStateJson(raw)
    expect(back).not.toBeNull()
    expect(back!.chord.chordStyle).toBe(state.chord.chordStyle)
    expect(back!.practice.scrollLevel).toBe(state.practice.scrollLevel)
    expect(back!.panelNotes).toBe(state.panelNotes)
  })

  it("parse treats missing panelNotes as empty string", () => {
    const raw = JSON.stringify({
      chord: {
        transposeSemitones: 0,
        capoFret: 0,
        zoomLevel: 1,
        chordStyle: "diagram",
        simplifyChords: false,
        parallelDisplay: false,
      },
      practice: { bpm: 120, scrollLevel: 20, metronomeMuted: false },
    })
    const p = parseSheetReaderStoredStateJson(raw)
    expect(p).not.toBeNull()
    expect(p!.panelNotes).toBe("")
  })

  it("parse truncates panelNotes to max length", () => {
    const long = "x".repeat(PANEL_NOTES_MAX_LEN + 80)
    const raw = JSON.stringify({
      chord: {
        transposeSemitones: 0,
        capoFret: 0,
        zoomLevel: 1,
        chordStyle: "diagram",
        simplifyChords: false,
        parallelDisplay: false,
      },
      practice: { bpm: 120, scrollLevel: 20, metronomeMuted: false },
      panelNotes: long,
    })
    const p = parseSheetReaderStoredStateJson(raw)
    expect(p!.panelNotes.length).toBe(PANEL_NOTES_MAX_LEN)
  })
})
