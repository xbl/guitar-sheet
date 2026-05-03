import { loadReaderChordPrefs, normalizeReaderChordPrefs } from "./readerPrefs"
import type { ReaderChordPrefs } from "./readerPrefs"
import {
  loadPracticePreferences,
  normalizePracticePreferences,
} from "../practice/practicePreferences"
import type { PracticePreferences } from "../practice/practicePreferences"

export type SheetReaderStoredState = {
  chord: ReaderChordPrefs
  practice: PracticePreferences
}

const PREFIX = "guitar-sheet.sheetReaderState.v1."

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
      }>
      return {
        chord: normalizeReaderChordPrefs(j.chord),
        practice: normalizePracticePreferences(j.practice),
      }
    } catch {
      /* fall through to migration */
    }
  }
  return {
    chord: loadReaderChordPrefs(),
    practice: loadPracticePreferences(storage),
  }
}

export function saveSheetReaderStoredState(
  storage: Storage,
  sheetId: string,
  state: SheetReaderStoredState,
): void {
  storage.setItem(
    sheetReaderStateStorageKey(sheetId),
    JSON.stringify({
      chord: { ...state.chord },
      practice: { ...state.practice },
    }),
  )
}
