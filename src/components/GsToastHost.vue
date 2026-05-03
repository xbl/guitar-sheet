<script setup lang="ts">
import { getToastState } from "../utils/toast"

const { message, variant } = getToastState()
</script>

<template>
  <Teleport to="body">
    <Transition name="gs-toast">
      <div
        v-if="message"
        class="gs-toast"
        :class="'gs-toast--' + variant"
        :role="variant === 'error' ? 'alert' : 'status'"
      >
        {{ message }}
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.gs-toast {
  position: fixed;
  top: max(0.75rem, env(safe-area-inset-top));
  left: 50%;
  transform: translateX(-50%);
  z-index: 10050;
  max-width: min(92vw, 26rem);
  padding: 0.65rem 1rem;
  border-radius: var(--gs-radius-md);
  font-size: 0.9rem;
  line-height: 1.4;
  box-shadow: var(--gs-shadow-sm), 0 8px 28px rgb(0 0 0 / 0.22);
  border: 1px solid var(--gs-border);
  background: var(--gs-bg-surface);
  color: var(--gs-text);
  pointer-events: none;
}
.gs-toast--error {
  border-color: color-mix(in srgb, var(--gs-danger) 45%, var(--gs-border));
  color: var(--gs-danger);
  background: color-mix(in srgb, var(--gs-bg-surface) 92%, var(--gs-danger) 8%);
}
.gs-toast--info {
  border-color: color-mix(in srgb, var(--gs-link) 35%, var(--gs-border));
  color: var(--gs-text);
}

.gs-toast-enter-active,
.gs-toast-leave-active {
  transition:
    opacity 0.22s ease,
    transform 0.22s ease;
}
.gs-toast-enter-from,
.gs-toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-0.5rem);
}
@media (prefers-reduced-motion: reduce) {
  .gs-toast-enter-active,
  .gs-toast-leave-active {
    transition: opacity 0.15s ease;
  }
  .gs-toast-enter-from,
  .gs-toast-leave-to {
    transform: translateX(-50%);
  }
}
</style>
