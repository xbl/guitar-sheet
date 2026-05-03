<script setup lang="ts">
defineProps<{
  embedUrl: string
}>()
</script>

<template>
  <section class="pdf-standalone">
    <figure class="inline-pdf-wrap">
      <embed
        v-if="embedUrl"
        class="inline-pdf-embed"
        :src="embedUrl"
        type="application/pdf"
        title="PDF"
      />
      <p v-else class="img-missing">（无法加载 PDF）</p>
    </figure>
  </section>
</template>

<style scoped>
.pdf-standalone {
  flex: 1 1 auto;
  padding: 1rem;
  min-height: 0;
  box-sizing: border-box;
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
.img-missing {
  margin: 0;
  font-size: 0.85rem;
  color: var(--gs-text-muted);
}
</style>
