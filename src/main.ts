import "./styles/light-paper.css";
import "./styles/dark-app.css";
import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";

const app = createApp(App);
app.use(router);
app.mount("#app");

// #region agent log
if (import.meta.env.DEV) {
  fetch("http://127.0.0.1:7268/ingest/f8b42a76-b477-4e11-b3eb-38547a546c8e", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Debug-Session-Id": "d399f4",
    },
    body: JSON.stringify({
      sessionId: "d399f4",
      hypothesisId: "G",
      location: "main.ts:boot",
      message: "ingest smoke after mount",
      data: {},
      timestamp: Date.now(),
    }),
  }).catch(() => {})
}
// #endregion
