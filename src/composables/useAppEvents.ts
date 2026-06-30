import { onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { exit } from "@tauri-apps/plugin-process";

import { frpcStatus, frpcError, logs, running } from "../state/runtime";
import { prefs } from "../state/prefs";
import { loadConfig } from "../commands/config";
import { loadPrefs, refreshAutoLaunch } from "../commands/prefs";
import { setLocale, normalizeLocale } from "../i18n";
import type { FrpcStatus, LogEntry, TrafficPayload } from "../types";
import {
  initFrpcVersion,
  checkFrpcUpdate,
  downloadedPending,
} from "./useFrpcUpdate";
import { checkAppUpdate } from "./useAppUpdate";
import {
  handleTrafficPayload,
  resetTraffic,
} from "./useTraffic";

/**
 * 应用层副作用集中点：
 * - 注册五类 Tauri 事件监听（`frpc://log` / `frpc://status` /
 *   `frpc://update-downloaded` / `frpc://traffic` / 窗口 `onCloseRequested`）
 * - 启动初始化序列：`loadConfig → loadPrefs → setLocale → refreshAutoLaunch
 *   → frpc_status → initFrpcVersion → checkFrpcUpdate → checkAppUpdate`
 * - 在 `onUnmounted` 中统一 unlisten，避免泄漏
 *
 * 返回值：
 * - `showCloseConfirm`：frpc 运行时关闭窗口触发的确认弹窗开关，由
 *   `App.vue` 双向绑定给 `CloseConfirm.vue`
 */
export function useAppEvents() {
  const showCloseConfirm = ref(false);

  let unlistenLog: UnlistenFn | null = null;
  let unlistenStatus: UnlistenFn | null = null;
  let unlistenUpdate: UnlistenFn | null = null;
  let unlistenTraffic: UnlistenFn | null = null;
  let unlistenClose: UnlistenFn | null = null;

  onMounted(async () => {
    // 监听 frpc 日志
    unlistenLog = await listen<LogEntry>(
      "frpc://log",
      (event) => {
        logs.push(event.payload);
        if (logs.length > 500) logs.shift();
      }
    );
    // 监听状态变更
    unlistenStatus = await listen<{ status: string; error: string | null }>(
      "frpc://status",
      (event) => {
        frpcStatus.value = event.payload.status as FrpcStatus;
        frpcError.value = event.payload.error ?? null;
        // 状态转 stopped 时清零图表数据（保持与后端中转生命周期对齐）
        if (frpcStatus.value === "stopped") {
          resetTraffic();
        }
      }
    );
    // 监听下载完成事件
    unlistenUpdate = await listen<{ version: string }>(
      "frpc://update-downloaded",
      (event) => {
        downloadedPending.value = event.payload.version;
      }
    );
    // 监听流量更新
    unlistenTraffic = await listen<TrafficPayload>(
      "frpc://traffic",
      (event) => {
        handleTrafficPayload(event.payload);
      }
    );
    // 拦截关闭：frpc 运行时弹窗让用户选「最小化 / 退出」；
    // frpc 已停止时直接退出进程（macOS 托盘应用窗口关闭默认不退出 NSApp）
    unlistenClose = await getCurrentWindow().onCloseRequested(async (event) => {
      event.preventDefault();
      if (running.value) {
        showCloseConfirm.value = true;
      } else {
        await exit(0);
      }
    });
    // 加载已保存配置
    await loadConfig();
    // 加载应用偏好（开机启动 / 静默启动 / 定时连接 / 界面语言），并校正 OS 实际开机启动状态
    await loadPrefs();
    // 在任何子视图渲染文案之前同步 i18n 语言，避免首屏中文闪一下再切英文
    setLocale(normalizeLocale(prefs.language));
    await refreshAutoLaunch();
    // 同步当前连接状态（前端启动时若 frpc 已在跑，需要后端给出权威状态）
    try {
      const s = await invoke<{ status: string; error: string | null }>("frpc_status");
      frpcStatus.value = s.status as FrpcStatus;
      frpcError.value = s.error ?? null;
    } catch {
      /* ignore */
    }
    // frpc 版本初始化 + 后台静默检查更新
    await initFrpcVersion();
    checkFrpcUpdate();
    // 软件本体静默检查更新（与 frpc 同样不打扰）
    checkAppUpdate();
  });

  onUnmounted(() => {
    unlistenLog?.();
    unlistenStatus?.();
    unlistenUpdate?.();
    unlistenTraffic?.();
    unlistenClose?.();
  });

  return { showCloseConfirm };
}
