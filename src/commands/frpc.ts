import { invoke } from "@tauri-apps/api/core";

import { toArgs } from "../state";

export async function startFrpc(): Promise<string | null> {
  try {
    await invoke("start_frpc", { args: toArgs() });
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? "启动失败";
  }
}

export async function stopFrpc(): Promise<string | null> {
  try {
    await invoke("stop_frpc");
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? "停止失败";
  }
}
