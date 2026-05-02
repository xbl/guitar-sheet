import { type InjectionKey, reactive, watch } from "vue"

export type ChordStyle = "diagram" | "text"

export type ReaderChordPrefs = {
  transposeSemitones: number
  capoFret: number
  /** 0 小 1 中 2 大 — mapped to base font pixels */
  zoomLevel: number
  chordStyle: ChordStyle
  simplifyChords: boolean
  parallelDisplay: boolean
}

const KEY = "guitar-sheet.reader.chordPrefs"

/** Provide from `SheetReaderPanel`; inject in `ReaderChordSettingsPanel`. */
export const readerChordPrefsInjectionKey: InjectionKey<ReaderChordPrefs> =
  Symbol("guitar-sheet.readerChordPrefs")

const DEFAULTS: ReaderChordPrefs = {
  transposeSemitones: 0,
  capoFret: 0,
  zoomLevel: 1,
  chordStyle: "diagram",
  simplifyChords: false,
  parallelDisplay: false,
}

export const ZOOM_FONT_PX = [14, 18, 22] as const
export const ZOOM_LABEL = ["小", "中", "大"] as const

function clamp(n: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, n))
}

export function loadReaderChordPrefs(): ReaderChordPrefs {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) return { ...DEFAULTS }
    const j = JSON.parse(raw) as Partial<ReaderChordPrefs>
    return {
      transposeSemitones: clamp(Number(j.transposeSemitones) || 0, -11, 11),
      capoFret: clamp(Number(j.capoFret) || 0, 0, 12),
      zoomLevel: (() => {
        const z = Number(j.zoomLevel)
        return clamp(Number.isFinite(z) ? z : 1, 0, 2)
      })(),
      chordStyle: j.chordStyle === "text" ? "text" : "diagram",
      simplifyChords: Boolean(j.simplifyChords),
      parallelDisplay: Boolean(j.parallelDisplay),
    }
  } catch {
    return { ...DEFAULTS }
  }
}

export function saveReaderChordPrefs(p: ReaderChordPrefs): void {
  localStorage.setItem(KEY, JSON.stringify(p))
}

/** Reactive prefs persisted to localStorage */
export function useReaderChordPrefs() {
  const prefs = reactive<ReaderChordPrefs>(loadReaderChordPrefs())
  watch(
    prefs,
    () => saveReaderChordPrefs({ ...prefs }),
    { deep: true },
  )
  return prefs
}
