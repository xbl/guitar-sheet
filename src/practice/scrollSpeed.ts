import {
  SCROLL_LEVEL_MAX,
  SCROLL_LEVEL_MIN,
  SCROLL_PX_PER_SEC_AT_MAX,
  SCROLL_PX_PER_SEC_AT_MIN,
} from "./constants"

export function clampScrollLevel(raw: number): number {
  const rounded = Math.round(raw)
  return Math.min(SCROLL_LEVEL_MAX, Math.max(SCROLL_LEVEL_MIN, rounded))
}

export function scrollLevelToPixelsPerSecond(level: number): number {
  const lv = clampScrollLevel(level)
  const t = (lv - SCROLL_LEVEL_MIN) / (SCROLL_LEVEL_MAX - SCROLL_LEVEL_MIN)
  return (
    SCROLL_PX_PER_SEC_AT_MIN + t * (SCROLL_PX_PER_SEC_AT_MAX - SCROLL_PX_PER_SEC_AT_MIN)
  )
}
