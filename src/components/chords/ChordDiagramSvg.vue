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
      shape-rendering="geometricPrecision"
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
        height="4"
        class="nut"
        rx="0.5"
      />
      <line
        v-else
        x1="4"
        x2="48"
        y1="18"
        y2="18"
        class="fret-line fret-line--top"
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
        y1="18"
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
        r="2.4"
        class="open-o"
      />
      <circle
        v-for="(d, i) in layout.dots"
        :key="'d' + i"
        :cx="stringXs[d.stringIndex]"
        :cy="dotCy(d.fret, layout.baseFret)"
        r="3.4"
        class="finger-dot"
      />
    </svg>
    <div v-else class="chord-fallback" aria-hidden="true">
      <span class="chord-title sm">{{ name }}</span>
    </div>
  </div>
</template>

<style scoped>
/* Size in em → scales with SheetReaderPanel 字号 (parent .text-preview font-size) */
.chord-diagram {
  display: inline-flex;
  flex-direction: column;
  align-items: center;
  font-size: inherit;
  min-width: 2.75em;
}
.chord-title {
  font-size: 0.82em;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--gs-chord, var(--gs-danger));
  line-height: 1.15;
  margin-bottom: 0.12em;
  text-align: center;
}
.chord-title.sm {
  font-size: 0.78em;
}
.highlighted .chord-title {
  color: var(--gs-link);
}
.chord-svg {
  width: 2.85em;
  min-width: 2.4em;
  height: auto;
  display: block;
  vertical-align: bottom;
}
.nut {
  fill: var(--gs-text);
}
.fret-line {
  stroke: var(--gs-chord-fret, #d1d5db);
  stroke-width: 0.5;
  stroke-linecap: square;
}
.fret-line--top {
  stroke: var(--gs-chord-fret, #d1d5db);
  stroke-width: 0.55;
}
.string-line {
  stroke: var(--gs-chord-string, #94a3b8);
  stroke-width: 0.28;
  stroke-linecap: square;
}
.fret-num {
  fill: var(--gs-text-muted);
  font-weight: 600;
}
.x-mute {
  fill: var(--gs-text-muted);
  font-weight: 700;
}
.open-o {
  fill: none;
  stroke: var(--gs-text);
  stroke-width: 1;
}
.finger-dot {
  fill: var(--gs-chord-dot, #374151);
}
.chord-fallback {
  min-height: 2.75em;
  min-width: 2.75em;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.12em;
  border: 1px dashed var(--gs-border);
  border-radius: var(--gs-radius-sm);
}
</style>
