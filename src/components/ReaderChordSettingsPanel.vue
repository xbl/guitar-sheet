<script setup lang="ts">
import { inject } from "vue"
import type { ReaderChordPrefs } from "../chords/readerPrefs"
import {
  readerChordPrefsInjectionKey,
  readerPanelNotesInjectionKey,
  ZOOM_LABEL,
} from "../chords/readerPrefs"
import { PANEL_NOTES_MAX_LEN } from "../chords/sheetReaderState"
import { transposeOffsetLabel } from "../chords/transposeChord"

const injected = inject(readerChordPrefsInjectionKey)
if (injected === undefined) {
  throw new Error("ReaderChordSettingsPanel: missing readerChordPrefs provider")
}
const prefs: ReaderChordPrefs = injected

const panelNotes = inject(readerPanelNotesInjectionKey)
if (panelNotes === undefined) {
  throw new Error("ReaderChordSettingsPanel: missing readerPanelNotes provider")
}

function clampSemi(delta: number) {
  prefs.transposeSemitones = Math.min(
    11,
    Math.max(-11, prefs.transposeSemitones + delta),
  )
}

function zoomStep(delta: number) {
  prefs.zoomLevel = Math.min(2, Math.max(0, prefs.zoomLevel + delta))
}

function toggle(key: "simplifyChords" | "parallelDisplay") {
  prefs[key] = !prefs[key]
}
</script>

<template>
  <aside class="reader-settings" aria-label="谱面设置">
    <h3 class="panel-title">谱面</h3>

    <div class="field row">
      <span class="label">移调</span>
      <span class="value key">{{ transposeOffsetLabel(prefs.transposeSemitones) }}</span>
      <div class="stepper">
        <button type="button" class="step-btn" @click="clampSemi(-1)">
          −
        </button>
        <button type="button" class="step-btn" @click="clampSemi(1)">
          +
        </button>
      </div>
    </div>

    <label class="field capo">
      <span class="label">变调夹</span>
      <input
        v-model.number="prefs.capoFret"
        class="capo-input"
        type="number"
        min="0"
        max="12"
        step="1"
      />
      <span class="suffix">品</span>
    </label>

    <div class="field row">
      <span class="label">缩放</span>
      <span class="value">{{ ZOOM_LABEL[prefs.zoomLevel] }}</span>
      <div class="stepper">
        <button
          type="button"
          class="step-btn"
          :disabled="prefs.zoomLevel <= 0"
          @click="zoomStep(-1)"
        >
          −
        </button>
        <button
          type="button"
          class="step-btn"
          :disabled="prefs.zoomLevel >= 2"
          @click="zoomStep(1)"
        >
          +
        </button>
      </div>
    </div>

    <label class="field select-row">
      <span class="label">和弦样式</span>
      <select v-model="prefs.chordStyle" class="select">
        <option value="diagram">和弦图</option>
        <option value="text">文字和弦</option>
      </select>
    </label>

    <label class="field toggle-row">
      <span class="label">简化和弦</span>
      <button
        type="button"
        class="switch"
        :class="{ on: prefs.simplifyChords }"
        role="switch"
        :aria-checked="prefs.simplifyChords"
        @click="toggle('simplifyChords')"
      />
    </label>

    <label class="field toggle-row">
      <span class="label">并行显示</span>
      <button
        type="button"
        class="switch"
        :class="{ on: prefs.parallelDisplay }"
        role="switch"
        :aria-checked="prefs.parallelDisplay"
        @click="toggle('parallelDisplay')"
      />
    </label>

    <label class="field notes-field">
      <span class="label">说明</span>
      <textarea
        v-model="panelNotes"
        class="notes-textarea"
        rows="5"
        :maxlength="PANEL_NOTES_MAX_LEN"
        placeholder="备忘文字，仅保存在本谱阅读设置中，不会写入正文。"
        spellcheck="false"
      />
    </label>
  </aside>
</template>

<style scoped>
.reader-settings {
  flex-shrink: 0;
  width: min(13.5rem, 100%);
  padding: 0.65rem 0.75rem 1rem;
  border-left: 1px solid var(--gs-border);
  background: var(--gs-bg-muted);
  border-radius: var(--gs-radius-md) 0 0 var(--gs-radius-md);
  font-size: 0.875rem;
}
.panel-title {
  margin: 0 0 0.65rem;
  font-size: 0.95rem;
  font-weight: 700;
  color: var(--gs-text-muted);
}
.field {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.65rem;
  flex-wrap: wrap;
}
.field.row {
  justify-content: space-between;
}
.label {
  color: var(--gs-text-muted);
  min-width: 4rem;
}
.value {
  flex: 1;
  text-align: right;
  font-variant-numeric: tabular-nums;
  color: var(--gs-text);
}
.value.key {
  font-weight: 600;
}
.stepper {
  display: flex;
  gap: 0.35rem;
}
.step-btn {
  width: 2rem;
  height: 2rem;
  padding: 0;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: var(--gs-text);
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
}
.step-btn:hover:not(:disabled) {
  border-color: var(--gs-primary-border);
  color: var(--gs-link);
}
.step-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
.capo {
  align-items: center;
}
.capo-input {
  width: 3.25rem;
  padding: 0.35rem 0.45rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: inherit;
  font-size: 0.9rem;
}
.suffix {
  color: var(--gs-text-muted);
  font-size: 0.85rem;
}
.select-row {
  flex-direction: column;
  align-items: stretch;
  gap: 0.35rem;
}
.select-row .label {
  min-width: 0;
}
.select {
  width: 100%;
  padding: 0.4rem 0.5rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: inherit;
  font-size: 0.9rem;
}
.toggle-row {
  justify-content: space-between;
  cursor: pointer;
  user-select: none;
}
.switch {
  width: 2.75rem;
  height: 1.45rem;
  border-radius: 999px;
  border: 1px solid var(--gs-border);
  background: #e5e5e7;
  position: relative;
  flex-shrink: 0;
  cursor: pointer;
  transition: background 0.15s;
}
.switch::after {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 1.1rem;
  height: 1.1rem;
  border-radius: 50%;
  background: var(--gs-bg-surface);
  box-shadow: var(--gs-shadow-sm);
  transition: transform 0.15s;
}
.switch.on {
  background: var(--gs-chord, #c62828);
  border-color: transparent;
}
.switch.on::after {
  transform: translateX(1.25rem);
}
.notes-field {
  flex-direction: column;
  align-items: stretch;
  gap: 0.35rem;
  margin-bottom: 0;
}
.notes-field .label {
  min-width: 0;
}
.notes-textarea {
  width: 100%;
  min-height: 4.5rem;
  padding: 0.45rem 0.5rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: inherit;
  font-size: 0.8rem;
  line-height: 1.45;
  resize: vertical;
  font-family: inherit;
}
.notes-textarea::placeholder {
  color: var(--gs-text-muted);
  opacity: 0.85;
}
</style>
