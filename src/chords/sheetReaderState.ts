import { loadReaderChordPrefs, normalizeReaderChordPrefs } from "./readerPrefs"
import type { ReaderChordPrefs } from "./readerPrefs"
import {
  loadPracticePreferences,
  normalizePracticePreferences,
} from "../practice/practicePreferences"
import type { PracticePreferences } from "../practice/practicePreferences"

/** Max length for 谱面设置「说明」文本（与 SQLite JSON 上限配合） */
export const PANEL_NOTES_MAX_LEN = 4000

export type SheetReaderStoredState = {
  chord: ReaderChordPrefs
  practice: PracticePreferences
  /** 谱面设置侧栏说明；不入正文文件 */
  panelNotes: string
}

const PREFIX = "guitar-sheet.sheetReaderState.v1."

export function normalizePanelNotes(raw: unknown): string {
  if (typeof raw !== "string") return ""
  return raw.length > PANEL_NOTES_MAX_LEN ? raw.slice(0, PANEL_NOTES_MAX_LEN) : raw
}

export function sheetReaderStateStorageKey(sheetId: string): string {
  return `${PREFIX}${sheetId}`
}

/**
 * Per-sheet reader UI: chord preview prefs + practice toolbar (BPM, scroll, mute).
 * When no blob exists yet, seeds once from legacy global `localStorage` keys.
 */
export function loadSheetReaderStoredState(
  storage: Storage,
  sheetId: string,
): SheetReaderStoredState {
  const raw = storage.getItem(sheetReaderStateStorageKey(sheetId))
  if (raw) {
    try {
      const j = JSON.parse(raw) as Partial<{
        chord: Partial<ReaderChordPrefs>
        practice: Partial<PracticePreferences>
        panelNotes: unknown
      }>
      return {
        chord: normalizeReaderChordPrefs(j.chord),
        practice: normalizePracticePreferences(j.practice),
        panelNotes: normalizePanelNotes(j.panelNotes),
      }
    } catch {
      /* fall through to migration */
    }
  }
  return {
    chord: loadReaderChordPrefs(),
    practice: loadPracticePreferences(storage),
    panelNotes: "",
  }
}

export function saveSheetReaderStoredState(
  storage: Storage,
  sheetId: string,
  state: SheetReaderStoredState,
): void {
  storage.setItem(
    sheetReaderStateStorageKey(sheetId),
    serializeSheetReaderStoredState(state),
  )
}

/** Stable JSON for SQLite `reader_state_json` and Tauri `invoke`. */
export function serializeSheetReaderStoredState(state: SheetReaderStoredState): string {
  return JSON.stringify({
    chord: { ...state.chord },
    practice: { ...state.practice },
    panelNotes: state.panelNotes,
  })
}

/** Parse backend JSON; returns `null` if invalid. */
export function parseSheetReaderStoredStateJson(
  raw: string,
): SheetReaderStoredState | null {
  const t = raw.trim()
  if (!t) return null
  try {
    const j = JSON.parse(t) as Partial<{
      chord: Partial<ReaderChordPrefs>
      practice: Partial<PracticePreferences>
      panelNotes: unknown
    }>
    return {
      chord: normalizeReaderChordPrefs(j.chord),
      practice: normalizePracticePreferences(j.practice),
      panelNotes: normalizePanelNotes(j.panelNotes),
    }
  } catch {
    return null
  }
}
