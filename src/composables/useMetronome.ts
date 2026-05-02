import { onScopeDispose, watch, type Ref } from "vue"

/** One short click per beat; `muted` skips audio only (interval still runs). */
export function useMetronome(options: {
  bpm: Ref<number>
  muted: Ref<boolean>
  isPlaying: Ref<boolean>
  onAudioUnavailable?: (reason: string) => void
}) {
  const { bpm, muted, isPlaying, onAudioUnavailable } = options
  let ctx: AudioContext | null = null
  let intervalId: ReturnType<typeof setInterval> | null = null

  function ensureContext(): AudioContext {
    if (!ctx) ctx = new AudioContext()
    return ctx
  }

  function playClick() {
    const c = ensureContext()
    if (muted.value) return

    const when = c.currentTime + 0.02
    const osc = c.createOscillator()
    const g = c.createGain()
    osc.type = "sine"
    osc.frequency.value = 1000
    osc.connect(g)
    g.connect(c.destination)

    g.gain.setValueAtTime(0.0001, when)
    g.gain.exponentialRampToValueAtTime(0.25, when + 0.005)
    g.gain.exponentialRampToValueAtTime(0.0001, when + 0.06)

    osc.start(when)
    osc.stop(when + 0.1)
  }

  function stopScheduling() {
    if (intervalId != null) {
      clearInterval(intervalId)
      intervalId = null
    }
  }

  function restartScheduling() {
    stopScheduling()
    if (!isPlaying.value) return

    const c = ensureContext()
    void c.resume().catch(() => {
      onAudioUnavailable?.("AudioContext could not resume")
    })

    const periodMs = (60 / Math.max(1, bpm.value)) * 1000
    playClick()
    intervalId = setInterval(() => {
      if (!isPlaying.value) return
      playClick()
    }, periodMs)
  }

  watch(
    [isPlaying, bpm, muted],
    () => {
      stopScheduling()
      if (isPlaying.value) restartScheduling()
    },
    { flush: "post" },
  )

  onScopeDispose(stopScheduling)

  return {
    async resumeIfNeeded() {
      const c = ensureContext()
      if (c.state === "suspended") await c.resume()
    },
  }
}
