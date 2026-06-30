import { invoke } from "@tauri-apps/api/core";

/** 后端 `latency::LatencyResult`——单次 TCP 握手探测结果（字段严格 snake_case）。 */
export interface LatencyResult {
  ok: boolean;
  latency_ms: number;
  error_kind: string | null;
}

/**
 * 探测本机到 `server_addr:server_port` 的单次 TCP 握手延迟（毫秒）。
 *
 * 后端用 `spawn_blocking` 跑阻塞 `connect_timeout`，详见
 * `src-tauri/AGENTS.md` 延迟探测小节。探测本身永远返回结构化
 * `LatencyResult`（`ok=false` + `error_kind` 分类）；仅当 invoke 通道异常
 * （Rust panic 等，理论不可达）时返回 `null`，调用方按 `unreachable` 兜底。
 */
export async function probeServerLatency(
  serverAddr: string,
  serverPort: number,
): Promise<LatencyResult | null> {
  try {
    return await invoke<LatencyResult>("probe_server_latency", {
      serverAddr,
      serverPort,
    });
  } catch {
    return null;
  }
}
