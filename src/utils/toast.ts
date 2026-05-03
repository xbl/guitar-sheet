import { ref } from "vue"

export type ToastVariant = "error" | "info"

const message = ref<string | null>(null)
const variant = ref<ToastVariant>("error")
let hideTimer: ReturnType<typeof setTimeout> | null = null

const DEFAULT_DURATION_MS = 4200

/** Global floating toast (see `GsToastHost.vue`). Safe to call from any component. */
export function showToast(
  text: string,
  opts?: { durationMs?: number; variant?: ToastVariant },
): void {
  if (hideTimer) {
    clearTimeout(hideTimer)
    hideTimer = null
  }
  message.value = text
  variant.value = opts?.variant ?? "error"
  const ms = opts?.durationMs ?? DEFAULT_DURATION_MS
  hideTimer = setTimeout(() => {
    message.value = null
    hideTimer = null
  }, ms)
}

/** For `GsToastHost` — keeps toast state in one module. */
export function getToastState() {
  return { message, variant }
}
