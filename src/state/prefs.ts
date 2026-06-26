import { reactive } from "vue";

import type { Prefs } from "../types";

/**
 * 应用偏好（开机启动 / 静默启动 / 开机自动连接 / 定时连接 / 界面语言）。
 *
 * - `auto_launch` 字段以 OS 注册表 / LaunchAgent 的实际状态为准
 *   （避免插件在某些平台 silently 失败导致 UI 与系统状态不一致）
 * - 通过 `tauri-plugin-store` 持久化到 `prefs.json`
 */
export const prefs = reactive<Prefs>({
  auto_launch: false,
  silent_start: false,
  auto_connect: false,
  schedule: {
    enabled: false,
    weekdays: [false, false, false, false, false, false, false],
    start_time: "08:00",
    stop_time: "18:00",
  },
  language: "zh-CN",
});
