import { createRouter, createWebHistory } from "vue-router";
import LibraryView from "../views/LibraryView.vue";
import ReaderView from "../views/ReaderView.vue";
import SettingsView from "../views/SettingsView.vue";

export const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: "/", name: "library", component: LibraryView },
    {
      path: "/sheet/:id",
      name: "reader",
      component: ReaderView,
      props: true,
    },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});
