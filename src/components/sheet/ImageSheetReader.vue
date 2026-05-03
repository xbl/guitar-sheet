<script setup lang="ts">
defineProps<{
  imgSrc: string
}>()
</script>

<template>
  <section class="img-wrap">
    <img v-if="imgSrc" :src="imgSrc" alt="sheet" />
  </section>
</template>

<style scoped>
.img-wrap {
  flex: 1 1 auto;
  padding: 1rem;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  min-height: 0;
}
.img-wrap img {
  max-width: 100%;
  height: auto;
}
</style>
