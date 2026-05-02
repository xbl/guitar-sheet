import { onScopeDispose, watch, type Ref } from "vue"
import { applyScrollDelta } from "../practice/scrollGeometry"
import { scrollLevelToPixelsPerSecond } from "../practice/scrollSpeed"

export function useAutoScroll(options: {
  scrollParentRef: Ref<HTMLElement | null>
  isPlaying: Ref<boolean>
  scrollLevel: Ref<number>
  onStoppedAtBottom?: () => void
}) {
  const { scrollParentRef, isPlaying, scrollLevel, onStoppedAtBottom } = options
  let rafId: number | null = null
  let lastTs: number | null = null

  function tick(now: number) {
    const el = scrollParentRef.value
    if (!el || !isPlaying.value) {
      lastTs = null
      return
    }
    if (lastTs == null) lastTs = now
    const dtMs = now - lastTs
    lastTs = now

    const pxPerSec = scrollLevelToPixelsPerSecond(scrollLevel.value)
    const deltaY = (pxPerSec * dtMs) / 1000

    const { nextScrollTop, reachedBottom } = applyScrollDelta({
      scrollTop: el.scrollTop,
      scrollHeight: el.scrollHeight,
      clientHeight: el.clientHeight,
      deltaY,
    })

    el.scrollTop = nextScrollTop

    if (reachedBottom) {
      isPlaying.value = false
      lastTs = null
      onStoppedAtBottom?.()
      return
    }

    rafId = requestAnimationFrame(tick)
  }

  function stopLoop() {
    if (rafId != null) cancelAnimationFrame(rafId)
    rafId = null
    lastTs = null
  }

  watch(
    isPlaying,
    (playing) => {
      stopLoop()
      if (playing) {
        lastTs = null
        rafId = requestAnimationFrame(tick)
      }
    },
    { flush: "sync" },
  )

  watch(scrollLevel, () => {
    if (isPlaying.value) {
      lastTs = null
    }
  })

  onScopeDispose(stopLoop)

  return { stopLoop }
}
