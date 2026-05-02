<script setup lang="ts">
import { computed } from "vue"
import ChordDiagramSvg from "./ChordDiagramSvg.vue"
import { parseChordSheet, type ParsedSheetLine } from "../../chords/parseChordSheet"
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

const parsedLines = computed<ParsedSheetLine[]>(() =>
  parseChordSheet(props.source),
)

function displayChord(raw: string): string {
  let s = raw
  if (props.simplifyChords) s = simplifyChordSymbol(s)
  return transposeChordSymbol(s, props.transposeSemitones)
}
</script>

<template>
  <div
    class="chord-sheet"
    :class="{ 'chord-sheet--parallel': parallelDisplay }"
  >
    <template v-for="(line, li) in parsedLines" :key="li">
      <div v-if="line.kind === 'section'" class="section-bar">
        {{ line.title }}
      </div>
      <p v-else-if="line.kind === 'plain' && line.text === ''" class="sheet-gap" />
      <p v-else-if="line.kind === 'plain'" class="plain-line">{{ line.text }}</p>
      <div v-else class="lyric-block">
        <div class="chord-row">
          <div
            v-for="(cell, ci) in line.cells"
            :key="'c' + ci"
            class="chord-cell"
          >
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
        </div>
        <div class="lyric-row">
          <span
            v-for="(cell, ci) in line.cells"
            :key="'l' + ci"
            class="lyric-cell"
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
.chord-sheet--parallel {
  column-count: 2;
  column-gap: 1.25rem;
}
.chord-sheet--parallel .section-bar,
.chord-sheet--parallel .plain-line,
.chord-sheet--parallel .lyric-block,
.chord-sheet--parallel .sheet-gap {
  break-inside: avoid;
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
.chord-row,
.lyric-row {
  display: flex;
  flex-wrap: nowrap;
  align-items: flex-end;
  gap: 0.25em 0.35em;
}
.chord-row {
  margin-bottom: 0.08em;
  min-height: 4.35em;
}
.chord-cell {
  flex: 0 0 auto;
  min-width: 2.85em;
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
  min-width: 2.85em;
  min-height: 2.85em;
}
.lyric-cell {
  flex: 0 1 auto;
  min-width: 1.5rem;
  font-size: inherit;
  line-height: inherit;
  border-bottom: 1px dotted transparent;
}
</style>
