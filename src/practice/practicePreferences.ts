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

function parseMetronomeMuted(v: unknown): boolean | undefined {
  if (v === "1" || v === "true" || v === true || v === 1) return true
  if (v === "0" || v === "false" || v === false || v === 0) return false
  return undefined
}

/** Merge partial values (e.g. parsed JSON) into clamped prefs. */
export function normalizePracticePreferences(
  j: Partial<PracticePreferences> | null | undefined,
): PracticePreferences {
  const bpm =
    typeof j?.bpm === "number" && Number.isFinite(j.bpm)
      ? clampBpm(j.bpm)
      : BPM_DEFAULT
  const scrollLevel =
    typeof j?.scrollLevel === "number" && Number.isFinite(j.scrollLevel)
      ? clampScrollLevel(Math.round(j.scrollLevel))
      : SCROLL_LEVEL_DEFAULT
  const m = parseMetronomeMuted(j?.metronomeMuted)
  const metronomeMuted = m === undefined ? false : m
  return { bpm, scrollLevel, metronomeMuted }
}

export function loadPracticePreferences(
  storage: Storage = localStorage,
): PracticePreferences {
  const partial: Partial<PracticePreferences> = {}

  const rawBpm = storage.getItem(STORAGE_KEY_BPM)
  if (rawBpm != null) {
    const n = Number(rawBpm)
    if (!Number.isNaN(n)) partial.bpm = n
  }

  const rawLv = storage.getItem(STORAGE_KEY_SCROLL_LEVEL)
  if (rawLv != null) {
    const n = Number(rawLv)
    if (!Number.isNaN(n)) partial.scrollLevel = n
  }

  const rawMute = storage.getItem(STORAGE_KEY_METRONOME_MUTED)
  const m = parseMetronomeMuted(rawMute)
  if (m !== undefined) partial.metronomeMuted = m

  return normalizePracticePreferences(partial)
}

export function savePracticePreferences(
  storage: Storage,
  prefs: PracticePreferences,
): void {
  storage.setItem(STORAGE_KEY_BPM, String(clampBpm(prefs.bpm)))
  storage.setItem(STORAGE_KEY_SCROLL_LEVEL, String(clampScrollLevel(prefs.scrollLevel)))
  storage.setItem(STORAGE_KEY_METRONOME_MUTED, prefs.metronomeMuted ? "1" : "0")
}
