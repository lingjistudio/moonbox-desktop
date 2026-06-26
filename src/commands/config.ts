import { invoke } from "@tauri-apps/api/core";

import { config, toArgs } from "../state";
import type { FrpcConfig } from "../types";

/**
 * 加载已保存的客户端配置。
 *
 * 返回 `Promise<boolean>`（而非 `string | null`）以与其它命令区分：
 * - `true`：命中已存配置并已写入响应式 `config`
 * - `false`：首次启动无配置，或解析失败（视同首次启动）
 *
 * 调用方根据布尔值决定是否显示引导卡片。
 */
export async function loadConfig(): Promise<boolean> {
  try {
    const saved = await invoke<FrpcConfig | null>("load_config");
    if (saved) {
      Object.assign(config, saved);
      return true;
    }
  } catch {
    /* 首次启动无配置 */
  }
  return false;
}

/** 返回 null 表示保存成功，否则为错误消息 */
export async function saveConfig(): Promise<string | null> {
  try {
    await invoke("save_config", { args: toArgs() });
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? "保存失败";
  }
}
