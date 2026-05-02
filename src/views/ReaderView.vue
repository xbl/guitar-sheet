<script setup lang="ts">
import SheetReaderPanel from "../components/SheetReaderPanel.vue"

defineProps<{ id: string }>()
</script>

<template>
  <SheetReaderPanel :sheet-id="id" variant="page" />
</template>
