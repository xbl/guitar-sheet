<script setup lang="ts">
import { computed } from "vue"
import type { ChordCell } from "../../chords/parseChordSheet"
import ChordDiagramSvg from "./ChordDiagramSvg.vue"
import {
  buildChordDisplayBlocks,
  parseChordSheet,
} from "../../chords/parseChordSheet"
import { simplifyChordSymbol } from "../../chords/simplifyChord"
import { transposeChordSymbol } from "../../chords/transposeChord"

const props = withDefaults(
  defineProps<{
    source: string
    transposeSemitones?: number
    simplifyChords?: boolean
    chordStyle?: "diagram" | "text"
    parallelDisplay?: boolean
  }>(),
  {
    transposeSemitones: 0,
    simplifyChords: false,
    chordStyle: "diagram",
    parallelDisplay: false,
  },
)

const parsedLines = computed(() => parseChordSheet(props.source))

const displayBlocks = computed(() =>
  buildChordDisplayBlocks(parsedLines.value, props.parallelDisplay),
)

function displayChord(raw: string): string {
  let s = raw
  if (props.simplifyChords) s = simplifyChordSymbol(s)
  return transposeChordSymbol(s, props.transposeSemitones)
}

/** Chord stacks on the first lyric character only (see `expandChordAnchorsForFirstChar`). */
function isChordAnchor(cell: ChordCell): boolean {
  return cell.chord !== null && cell.lyric.length === 1
}
</script>

<template>
  <div
    class="chord-sheet"
    :class="{ 'chord-sheet--parallel': parallelDisplay }"
  >
    <template v-for="(block, bi) in displayBlocks" :key="bi">
      <div v-if="block.kind === 'section'" class="section-bar">
        {{ block.title }}
      </div>
      <p v-else-if="block.kind === 'gap'" class="sheet-gap" />
      <p v-else-if="block.kind === 'plain'" class="plain-line">{{ block.text }}</p>

      <div v-else-if="block.kind === 'lyric-line'" class="lyric-block">
        <div class="lyric-line-cols">
          <div
            v-for="(cell, ci) in block.cells"
            :key="'col' + ci"
            class="lyric-col"
            :class="{ 'lyric-col--anchor': isChordAnchor(cell) }"
          >
            <div class="chord-cell">
              <ChordDiagramSvg
                v-if="cell.chord && chordStyle === 'diagram'"
                :name="displayChord(cell.chord)"
              />
              <span
                v-else-if="cell.chord && chordStyle === 'text'"
                class="chord-text"
              >{{ displayChord(cell.chord) }}</span>
              <div v-else class="chord-empty" />
            </div>
            <span
              class="lyric-cell"
              :class="{ 'lyric-cell--anchored': isChordAnchor(cell) }"
            >{{ cell.lyric }}</span>
          </div>
        </div>
      </div>

      <div v-else class="lyric-block lyric-block--flow">
        <div
          v-for="(cell, ci) in block.cells"
          :key="'f' + ci"
          class="lyric-unit"
          :class="{ 'lyric-unit--anchor': isChordAnchor(cell) }"
        >
          <div class="chord-cell">
            <ChordDiagramSvg
              v-if="cell.chord && chordStyle === 'diagram'"
              :name="displayChord(cell.chord)"
            />
            <span
              v-else-if="cell.chord && chordStyle === 'text'"
              class="chord-text"
            >{{ displayChord(cell.chord) }}</span>
            <div v-else class="chord-empty" />
          </div>
          <span
            class="lyric-cell"
            :class="{ 'lyric-cell--anchored': isChordAnchor(cell) }"
          >{{ cell.lyric }}</span>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.chord-sheet {
  font-family: ui-sans-serif, system-ui, sans-serif;
  font-size: inherit;
  color: var(--gs-text);
  line-height: 1.55;
}
.chord-sheet--parallel .plain-line {
  white-space: normal;
}
.section-bar {
  margin: 1rem 0 0.6rem;
  padding: 0.35rem 0.75rem;
  background: var(--gs-bg-muted);
  border-radius: var(--gs-radius-sm);
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--gs-text-muted);
  letter-spacing: 0.05em;
}
.section-bar:first-child {
  margin-top: 0;
}
.sheet-gap {
  margin: 0;
  min-height: 0.35rem;
}
.plain-line {
  margin: 0.15rem 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: inherit;
}
.lyric-block {
  margin: 0.35rem 0 0.75rem;
}
.lyric-line-cols {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  column-gap: 0;
  row-gap: 0.15em;
}
.lyric-col {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 0 1 auto;
  min-width: 0;
  padding-top: 4.45em;
  overflow: visible;
}
.lyric-col .chord-cell {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  min-height: 4.25em;
  min-width: 0;
  margin-bottom: 0;
  display: flex;
  justify-content: center;
  align-items: flex-end;
}
.lyric-col--anchor :deep(.chord-diagram) {
  min-width: 0;
}
.lyric-col--anchor :deep(.chord-svg) {
  width: 2.35em;
  min-width: 0;
}
.lyric-block--flow {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  column-gap: 0;
  row-gap: 0.15em;
  width: 100%;
}
.lyric-unit {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  flex: 0 1 auto;
  min-width: 0;
  max-width: 100%;
  padding-top: 4.45em;
  overflow: visible;
}
.lyric-block--flow .chord-cell {
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  min-height: 4.25em;
  min-width: 0;
  margin-bottom: 0;
  display: flex;
  justify-content: center;
  align-items: flex-end;
}
.lyric-unit--anchor :deep(.chord-diagram) {
  min-width: 0;
}
.lyric-unit--anchor :deep(.chord-svg) {
  width: 2.35em;
  min-width: 0;
}
.chord-cell {
  flex: 0 0 auto;
  display: flex;
  justify-content: center;
  align-items: flex-end;
}
.chord-text {
  font-size: 0.82em;
  font-weight: 700;
  color: var(--gs-chord, var(--gs-danger));
  line-height: 1.1;
  padding-bottom: 0.15em;
}
.chord-empty {
  min-width: 0;
  width: 0;
  min-height: 0;
  height: 0;
  overflow: hidden;
}
.lyric-block--flow .lyric-cell {
  flex: none;
  max-width: 100%;
  word-break: break-word;
}
.lyric-cell {
  flex: 0 1 auto;
  min-width: 0;
  font-size: inherit;
  line-height: inherit;
  align-self: stretch;
  text-align: left;
}
.lyric-cell--anchored {
  text-decoration: underline;
  text-decoration-thickness: 1.5px;
  text-underline-offset: 0.18em;
  font-weight: 600;
  text-align: center;
}
</style>
