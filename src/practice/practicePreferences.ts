import {
  BPM_DEFAULT,
  BPM_MAX,
  BPM_MIN,
  SCROLL_LEVEL_DEFAULT,
  STORAGE_KEY_BPM,
  STORAGE_KEY_METRONOME_MUTED,
  STORAGE_KEY_SCROLL_LEVEL,
} from "./constants"
import { clampScrollLevel } from "./scrollSpeed"

export type PracticePreferences = {
  bpm: number
  scrollLevel: number
  metronomeMuted: boolean
}

function clampBpm(raw: number): number {
  const r = Math.round(raw)
  return Math.min(BPM_MAX, Math.max(BPM_MIN, r))
}

export function loadPracticePreferences(
  storage: Storage = localStorage,
): PracticePreferences {
  let bpm = BPM_DEFAULT
  let scrollLevel = SCROLL_LEVEL_DEFAULT
  let metronomeMuted = false

  const rawBpm = storage.getItem(STORAGE_KEY_BPM)
  if (rawBpm != null) {
    const n = Number(rawBpm)
    if (!Number.isNaN(n)) bpm = clampBpm(n)
  }

  const rawLv = storage.getItem(STORAGE_KEY_SCROLL_LEVEL)
  if (rawLv != null) {
    const n = Number(rawLv)
    if (!Number.isNaN(n)) scrollLevel = clampScrollLevel(n)
  }

  const rawMute = storage.getItem(STORAGE_KEY_METRONOME_MUTED)
  if (rawMute === "1" || rawMute === "true") metronomeMuted = true
  if (rawMute === "0" || rawMute === "false") metronomeMuted = false

  return { bpm, scrollLevel, metronomeMuted }
}

export function savePracticePreferences(
  storage: Storage,
  prefs: PracticePreferences,
): void {
  storage.setItem(STORAGE_KEY_BPM, String(clampBpm(prefs.bpm)))
  storage.setItem(STORAGE_KEY_SCROLL_LEVEL, String(clampScrollLevel(prefs.scrollLevel)))
  storage.setItem(STORAGE_KEY_METRONOME_MUTED, prefs.metronomeMuted ? "1" : "0")
}
