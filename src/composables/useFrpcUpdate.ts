import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

import { i18n } from "../i18n";

export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  release_notes: string;
  asset_name: string;
  asset_size: number;
}

/** 当前生效的 frpc 版本（已经过 setup 阶段 apply_pending 处理） */
export const frpcVersion = ref("");
/** 检查到的可下载新版本；null 表示无更新或尚未检查 */
export const updateInfo = ref<UpdateInfo | null>(null);
/** 已下载、待下次启动应用的版本 */
export const downloadedPending = ref<string | null>(null);
/** 本次启动相对上次启动新应用了哪个版本（用于"已升级"提示） */
export const recentlyApplied = ref<string | null>(null);
/** 是否正在执行下载 */
export const downloading = ref(false);

/** 读取当前 frpc 版本；与 localStorage 比对，若发生变化则置 recentlyApplied */
export async function initFrpcVersion(): Promise<void> {
  try {
    const v = await invoke<string>("get_frpc_version");
    const prev = localStorage.getItem("moonbox.frpcVersion");
    if (prev && prev !== v) {
      recentlyApplied.value = v;
    }
    localStorage.setItem("moonbox.frpcVersion", v);
    frpcVersion.value = v;
  } catch {
    /* ignore */
  }
}

/** 后台静默检查 GitHub 是否有新版本 */
export async function checkFrpcUpdate(): Promise<void> {
  try {
    updateInfo.value = await invoke<UpdateInfo | null>("check_frpc_update");
  } catch (e) {
    console.warn("[frpc-update] 检查失败", e);
  }
}

/** 下载指定版本到 pending 目录，成功后置 downloadedPending */
export async function downloadFrpcUpdate(version: string): Promise<string | null> {
  downloading.value = true;
  try {
    await invoke("download_frpc_update", { version });
    downloadedPending.value = version;
    updateInfo.value = null;
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? i18n.global.t("err_download");
  } finally {
    downloading.value = false;
  }
}
