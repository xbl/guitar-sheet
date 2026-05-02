<script setup lang="ts">
import { computed } from "vue"
import { buildChordLayout } from "../../chords/chordDiagramLayout"

const props = withDefaults(
  defineProps<{
    name: string
    /** Playback / focus highlight (optional) */
    highlighted?: boolean
  }>(),
  { highlighted: false },
)

const layout = computed(() => buildChordLayout(props.name))

const vb = "0 0 52 48"

/** Fret-cell centers for four rows below nut (base-relative positions). */
function dotCy(fret: number, base: number): number {
  const rel = fret - base + 1
  const centers = [20, 26, 32, 38]
  return centers[Math.min(Math.max(rel, 1), 4) - 1]!
}

const fretWireYs = [22, 28, 34, 40]

const stringXs = [6, 14, 22, 30, 38, 46]
</script>

<template>
  <div class="chord-diagram" :class="{ highlighted }">
    <div class="chord-title">{{ name }}</div>
    <svg
      v-if="layout.kind === 'grid'"
      class="chord-svg"
      :viewBox="vb"
      aria-hidden="true"
    >
      <text
        v-if="layout.baseFret > 1"
        x="1"
        y="27"
        class="fret-num"
        font-size="8"
      >
        {{ layout.baseFret }}
      </text>
      <rect
        v-if="layout.baseFret === 1"
        x="4"
        y="14"
        width="44"
        height="3.5"
        class="nut"
        rx="0.5"
      />
      <line
        v-else
        x1="4"
        x2="48"
        y1="17.5"
        y2="17.5"
        class="fret-line"
      />
      <line
        v-for="(fy, idx) in fretWireYs"
        :key="'f' + idx"
        x1="4"
        x2="48"
        :y1="fy"
        :y2="fy"
        class="fret-line"
      />
      <line
        v-for="(sx, si) in stringXs"
        :key="'s' + si"
        :x1="sx"
        :x2="sx"
        y1="17.5"
        y2="41"
        class="string-line"
      />
      <text
        v-for="si in layout.mutes"
        :key="'m' + si"
        :x="stringXs[si]!"
        y="11"
        text-anchor="middle"
        class="x-mute"
        font-size="9"
      >
        ×
      </text>
      <circle
        v-for="si in layout.opens"
        :key="'o' + si"
        :cx="stringXs[si]"
        cy="11"
        r="2.2"
        class="open-o"
      />
      <circle
        v-for="(d, i) in layout.dots"
        :key="'d' + i"
        :cx="stringXs[d.stringIndex]"
        :cy="dotCy(d.fret, layout.baseFret)"
        r="3.2"
        class="finger-dot"
      />
    </svg>
    <div v-else class="chord-fallback" aria-hidden="true">
      <span class="chord-title sm">{{ name }}</span>
    </div>
  </div>
</template>

<style scoped>
.chord-diagram {
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  min-width: 3rem;
}
.chord-title {
  font-size: 0.72rem;
  font-weight: 700;
  color: var(--gs-chord, var(--gs-danger));
  line-height: 1.1;
  margin-bottom: 2px;
  text-align: center;
}
.chord-title.sm {
  font-size: 0.65rem;
}
.highlighted .chord-title {
  color: var(--gs-link);
}
.chord-svg {
  width: 2.75rem;
  height: auto;
  display: block;
}
.nut {
  fill: var(--gs-text);
}
.fret-line,
.string-line {
  stroke: var(--gs-border);
}
.string-line {
  stroke-width: 0.35;
}
.fret-line {
  stroke-width: 0.55;
}
.fret-num {
  fill: var(--gs-text-muted);
}
.x-mute {
  fill: var(--gs-text-muted);
}
.open-o {
  fill: none;
  stroke: var(--gs-text);
  stroke-width: 0.9;
}
.finger-dot {
  fill: var(--gs-text);
}
.chord-fallback {
  min-height: 2.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.15rem;
  border: 1px dashed var(--gs-border);
  border-radius: var(--gs-radius-sm);
}
</style>
