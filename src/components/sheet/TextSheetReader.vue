<script setup lang="ts">
import { computed, ref } from "vue"
import ChordSheetRenderer from "../chords/ChordSheetRenderer.vue"
import { looksLikeChordSheet } from "../../chords/parseChordSheet"
import type { ChordStyle } from "../../chords/readerPrefs"
import type { TextPreviewSeg } from "../../chords/textSheetEmbeds"

const textDraft = defineModel<string>("textDraft", { required: true })

const props = withDefaults(
  defineProps<{
    editingText: boolean
    textPreviewSegments: TextPreviewSeg[]
    previewUrls: Record<string, string>
    pdfPreviewUrls: Record<string, string>
    fontPx: number
    /** 正文行高倍数 */
    lineHeight?: number
    transposeSemitones: number
    simplifyChords: boolean
    chordStyle: ChordStyle
    parallelDisplay: boolean
  }>(),
  { lineHeight: 1.6 },
)

const emit = defineEmits<{
  paste: [e: ClipboardEvent]
  blur: []
  enterEdit: []
}>()

const textAreaRef = ref<HTMLTextAreaElement | null>(null)

defineExpose({
  getTextarea: () => textAreaRef.value,
})

const lhStr = computed(() => String(props.lineHeight))
</script>

<template>
  <section class="text-wrap">
    <div class="text-sheet-main">
      <p v-if="editingText" class="paste-hint">
        提示：Ctrl+V 或拖拽可插入多张图片与多个 PDF；失焦自动保存正文（空白不保存）。若使用「和弦在上、歌词在下」的文本排版，退出编辑时会自动转为
        <code>[和弦]</code> 内嵌格式。
      </p>
      <p v-else class="paste-hint paste-hint--preview">
        可将图片或 PDF 拖入下方阅读区任意位置（含空白处或预览中的图片旁），自动插入到正文末尾并保存（无需先点进编辑）。
      </p>
      <div class="text-body-drop-shell">
        <textarea
          v-if="editingText"
          ref="textAreaRef"
          v-model="textDraft"
          class="tab edit"
          :style="{ fontSize: fontPx + 'px', lineHeight: lhStr }"
          @paste="emit('paste', $event)"
          @blur="emit('blur')"
        />
        <div
          v-else
          class="tab text-preview"
          role="button"
          tabindex="0"
          title="点击编辑正文；也可拖入图片或 PDF"
          :style="{ fontSize: fontPx + 'px', lineHeight: lhStr }"
          @click="emit('enterEdit')"
          @keydown.enter.prevent="emit('enterEdit')"
        >
          <template v-for="(seg, i) in textPreviewSegments" :key="i">
            <ChordSheetRenderer
              v-if="seg.type === 'text' && looksLikeChordSheet(seg.content)"
              :source="seg.content"
              :transpose-semitones="transposeSemitones"
              :simplify-chords="simplifyChords"
              :chord-style="chordStyle"
              :parallel-display="parallelDisplay"
            />
            <pre
              v-else-if="seg.type === 'text'"
              class="text-chunk"
            >{{ seg.content }}</pre>
            <figure v-else-if="seg.type === 'img'" class="inline-img-wrap">
              <img
                v-if="previewUrls[seg.file]"
                draggable="false"
                :src="previewUrls[seg.file]"
                :alt="seg.file"
                class="inline-img"
              />
              <p v-else class="img-missing">（无法加载图片：{{ seg.file }}）</p>
            </figure>
            <figure v-else-if="seg.type === 'pdf'" class="inline-pdf-wrap">
              <embed
                v-if="pdfPreviewUrls[seg.file]"
                class="inline-pdf-embed"
                :src="pdfPreviewUrls[seg.file]"
                type="application/pdf"
                title="内嵌 PDF"
              />
              <p v-else class="img-missing">（无法加载 PDF：{{ seg.file }}）</p>
            </figure>
          </template>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.text-wrap {
  padding: 0.75rem 1rem 1rem;
  flex: 1 1 auto;
  min-height: min-content;
  display: flex;
  flex-direction: column;
}
.text-sheet-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  min-height: min-content;
}
.text-body-drop-shell {
  flex: 1 1 auto;
  min-height: min-content;
  display: flex;
  flex-direction: column;
  border-radius: var(--gs-radius-md);
  transition:
    box-shadow 0.16s ease,
    background 0.16s ease;
}
@media (prefers-reduced-motion: reduce) {
  .text-body-drop-shell {
    transition: none;
  }
}
.paste-hint {
  margin: 0 0 0.5rem;
  font-size: 0.8rem;
  color: var(--gs-text-muted);
  line-height: 1.4;
}
.tab {
  margin: 0;
  width: 100%;
  flex: 1 1 auto;
  min-height: 10rem;
  box-sizing: border-box;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  background: var(--gs-bg-muted);
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-md);
  padding: 1rem;
  overflow: auto;
}
.tab.edit {
  flex: 1 1 auto;
  min-height: 0;
  resize: none;
  line-height: inherit;
  color: var(--gs-text);
  caret-color: var(--gs-text);
}
.tab.text-preview {
  overflow: visible;
  min-height: min-content;
}
.text-preview {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  cursor: pointer;
  border-radius: var(--gs-radius-sm);
  flex: 1 1 auto;
}
.text-preview:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--gs-primary-border) 65%, transparent);
  outline-offset: 2px;
}
.text-chunk {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  font-size: inherit;
  line-height: inherit;
}
.inline-img-wrap {
  margin: 0;
  padding: 0;
  display: flex;
  justify-content: flex-start;
}
.inline-img {
  max-width: 100%;
  height: auto;
  display: block;
  border-radius: var(--gs-radius-sm);
  border: 1px solid var(--gs-border);
}
.img-missing {
  margin: 0;
  font-size: 0.85rem;
  color: var(--gs-text-muted);
}
.inline-pdf-wrap {
  margin: 0;
  width: 100%;
  min-height: clamp(16rem, 48vh, 36rem);
  display: flex;
  flex-direction: column;
}
.inline-pdf-embed {
  display: block;
  width: 100%;
  flex: 1 1 auto;
  min-height: clamp(14rem, 45vh, 32rem);
  height: clamp(16rem, 52vh, 40rem);
  max-height: min(78vh, 44rem);
  border: 1px solid var(--gs-border);
  border-radius: var(--gs-radius-sm);
  background: var(--gs-bg-muted);
}
</style>
