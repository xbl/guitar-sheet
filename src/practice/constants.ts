export const SCROLL_LEVEL_MIN = 1
export const SCROLL_LEVEL_MAX = 20
export const SCROLL_LEVEL_DEFAULT = 10

export const BPM_MIN = 40
export const BPM_MAX = 240
export const BPM_DEFAULT = 120

export const STORAGE_KEY_BPM = "guitar-sheet.practice.bpm"
export const STORAGE_KEY_SCROLL_LEVEL = "guitar-sheet.practice.scrollLevel"
export const STORAGE_KEY_METRONOME_MUTED = "guitar-sheet.practice.metronomeMuted"

/** Linear map: level 1 → 10 px/s, level 20 → 200 px/s */
export const SCROLL_PX_PER_SEC_AT_MIN = 10
export const SCROLL_PX_PER_SEC_AT_MAX = 200
