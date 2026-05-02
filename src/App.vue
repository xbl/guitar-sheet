<script setup lang="ts">
import { RouterLink, RouterView } from "vue-router";
</script>

<template>
  <div id="shell">
    <nav class="nav">
      <RouterLink to="/" class="brand">Guitar Sheet</RouterLink>
      <RouterLink to="/settings">设置</RouterLink>
    </nav>
    <RouterView />
  </div>
</template>

<style>
* {
  box-sizing: border-box;
}
body {
  margin: 0;
  font-family:
    system-ui,
    -apple-system,
    sans-serif;
  color: #111;
  background: #fff;
}
#shell {
  min-height: 100vh;
}
.nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  border-bottom: 1px solid #e8e8e8;
  background: #fafafa;
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
