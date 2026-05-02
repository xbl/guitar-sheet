<script setup lang="ts">
import { RouterLink, RouterView } from "vue-router";
</script>

<template>
  <div id="shell">
    <nav class="nav">
      <RouterLink to="/" class="brand">Guitar Sheet</RouterLink>
      <RouterLink to="/settings">设置</RouterLink>
    </nav>
    <div class="shell-content">
      <RouterView />
    </div>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
}
html,
body {
  height: 100%;
  margin: 0;
  overflow: hidden;
}
body {
  font-family:
    system-ui,
    -apple-system,
    sans-serif;
  color: #111;
  background: #fff;
}
#shell {
  height: 100dvh;
  max-height: 100dvh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.shell-content {
  flex: 1;
  min-height: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid #e8e8e8;
  background: #fafafa;
  flex-shrink: 0;
}
.nav a {
  color: #2563eb;
  text-decoration: none;
  font-size: 0.95rem;
}
.nav a.router-link-active {
  font-weight: 600;
}
.brand {
  font-weight: 700;
  color: #111 !important;
}
</style>
