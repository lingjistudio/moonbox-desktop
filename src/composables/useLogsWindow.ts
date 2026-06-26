import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import { i18n } from "../i18n";

const LOGS_WINDOW_LABEL = "logs";

/**
 * 打开独立日志窗口。若已存在则聚焦唤起，避免多开。
 * 重复点击按钮只会重定向到同一个窗口。
 *
 * 注意：窗口标题在创建时按当前语言取值；用户切语言后已开的窗口不会跟随，
 * 需重新打开。代价可接受（日志窗口非高频）。
 */
export async function openLogs(): Promise<void> {
  const existing = await WebviewWindow.getByLabel(LOGS_WINDOW_LABEL);
  if (existing) {
    await existing.unminimize();
    await existing.setFocus();
    return;
  }
  const title = i18n.global.t("logs_section_title");
  // WebviewWindow 构造是异步执行；监听 created / error 事件方便定位失败原因
  const w = new WebviewWindow(LOGS_WINDOW_LABEL, {
    // 用相对根 + query，避免某些 Tauri 版本对带 index.html 的相对 URL 解析失败
    url: "/?view=logs",
    title,
    width: 720,
    height: 520,
    resizable: true,
    maximizable: true,
    center: true,
    decorations: true,
  });
  w.once("tauri://created", () => {
    console.log("[logs-window] created");
  });
  w.once("tauri://error", (e) => {
    console.error("[logs-window] create error", e);
  });
}
