import { createApp } from "vue";
import App from "./App.vue";
import LogsWindow from "./views/LogsWindow.vue";
import "./styles.css";
import { i18n } from "./i18n";

// Tauri 多窗口复用同一份前端 bundle：主窗加载无 query 的 index.html，
// 日志窗口加载 index.html?view=logs。这里根据 query 决定挂载哪个根组件，
// 让日志窗口避开 App.vue 的 onCloseRequested / 全局快捷键 / TitleBar 等主窗逻辑。
const params = new URLSearchParams(window.location.search);
const view = params.get("view");

if (view === "logs") {
  const logsApp = createApp(LogsWindow);
  logsApp.use(i18n);
  logsApp.mount("#app");
} else {
  const app = createApp(App);
  app.use(i18n);
  app.mount("#app");
}
