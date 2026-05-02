<script setup lang="ts">
import {
  BPM_MAX,
  BPM_MIN,
  SCROLL_LEVEL_MAX,
  SCROLL_LEVEL_MIN,
} from "../../practice/constants"

defineProps<{
  isPlaying: boolean
  bpm: number
  scrollLevel: number
  metronomeMuted: boolean
  audioWarning?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  togglePlay: []
  "update:bpm": [number]
  "update:scrollLevel": [number]
  "update:metronomeMuted": [boolean]
}>()
</script>

<template>
  <div class="practice-toolbar" role="toolbar" aria-label="练习">
    <button
      type="button"
      class="play-btn"
      :disabled="disabled"
      :title="disabled ? '编辑正文中无法跟谱播放' : undefined"
      @click="emit('togglePlay')"
    >
      {{ isPlaying ? "暂停" : "播放" }}
    </button>
    <label class="field">
      <span class="label">BPM</span>
      <input
        class="input-num"
        type="number"
        :min="BPM_MIN"
        :max="BPM_MAX"
        :value="bpm"
        :disabled="disabled"
        @input="
          emit('update:bpm', Number(($event.target as HTMLInputElement).value))
        "
      />
    </label>
    <label class="field check">
      <input
        type="checkbox"
        :checked="metronomeMuted"
        :disabled="disabled"
        @change="
          emit(
            'update:metronomeMuted',
            ($event.target as HTMLInputElement).checked,
          )
        "
      />
      节拍器静音
    </label>
    <label class="field range-field">
      <span class="label">滚动</span>
      <input
        type="range"
        :min="SCROLL_LEVEL_MIN"
        :max="SCROLL_LEVEL_MAX"
        step="1"
        :value="scrollLevel"
        :disabled="disabled"
        @input="
          emit(
            'update:scrollLevel',
            Number(($event.target as HTMLInputElement).value),
          )
        "
      />
      <span class="level">{{ scrollLevel }}</span>
    </label>
    <p v-if="audioWarning" class="warn">{{ audioWarning }}</p>
  </div>
</template>

<style scoped>
.practice-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 0.65rem 1rem;
  align-items: center;
  font-size: 0.9rem;
}
.play-btn {
  padding: 0.35rem 0.75rem;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: var(--gs-text);
  cursor: pointer;
}
.play-btn:hover:not(:disabled) {
  border-color: var(--gs-link);
  color: var(--gs-link);
}
.play-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.field {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  margin: 0;
}
.field.check {
  gap: 0.25rem;
}
.label {
  color: var(--gs-text-muted);
  font-size: 0.85rem;
}
.input-num {
  width: 4rem;
  padding: 0.25rem 0.35rem;
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-surface);
  color: inherit;
}
.range-field {
  flex: 1 1 12rem;
  min-width: 10rem;
}
.range-field input[type="range"] {
  flex: 1;
  min-width: 4rem;
}
.level {
  min-width: 1.5rem;
  font-variant-numeric: tabular-nums;
  color: var(--gs-text-muted);
}
.warn {
  margin: 0;
  flex-basis: 100%;
  font-size: 0.8rem;
  color: var(--gs-danger);
}
</style>
