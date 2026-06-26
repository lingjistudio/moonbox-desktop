import { invoke } from "@tauri-apps/api/core";

import { i18n } from "../i18n";
import { prefs } from "../state";
import type { Prefs, Schedule } from "../types";

/** 拉取应用偏好并写入响应式 prefs。返回错误消息。 */
export async function loadPrefs(): Promise<string | null> {
  try {
    const p = await invoke<Prefs>("get_prefs");
    if (p) {
      prefs.auto_launch = !!p.auto_launch;
      prefs.silent_start = !!p.silent_start;
      prefs.auto_connect = !!p.auto_connect;
      if (p.schedule) {
        prefs.schedule.enabled = !!p.schedule.enabled;
        prefs.schedule.weekdays = (p.schedule.weekdays ?? [
          false, false, false, false, false, false, false,
        ]) as Schedule["weekdays"];
        prefs.schedule.start_time = p.schedule.start_time || "08:00";
        prefs.schedule.stop_time = p.schedule.stop_time || "18:00";
      }
      if (typeof p.language === "string" && p.language) {
        prefs.language = p.language;
      }
    }
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? i18n.global.t("err_load_prefs");
  }
}

/** 持久化应用偏好（不触发 OS 动作，仅写 store）。 */
export async function savePrefs(): Promise<string | null> {
  try {
    await invoke("save_prefs", {
      prefs: {
        auto_launch: prefs.auto_launch,
        silent_start: prefs.silent_start,
        auto_connect: prefs.auto_connect,
        schedule: prefs.schedule,
        language: prefs.language,
      },
    });
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? i18n.global.t("err_save_prefs");
  }
}

/**
 * 切换开机启动：调用后端 autostart manager 写 OS 启动项，
 * 并以 OS 实际状态回填 `prefs.auto_launch`。
 * 返回错误消息；成功返回 null。
 */
export async function setAutoLaunch(enabled: boolean): Promise<string | null> {
  try {
    const actual = await invoke<boolean>("set_auto_launch", { enabled });
    prefs.auto_launch = !!actual;
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? i18n.global.t("err_operation");
  }
}

/** 同步 OS 启动项实际状态到 prefs（用于启动时校正）。 */
export async function refreshAutoLaunch(): Promise<string | null> {
  try {
    const actual = await invoke<boolean>("get_auto_launch");
    prefs.auto_launch = !!actual;
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? i18n.global.t("err_query_auto_launch");
  }
}
