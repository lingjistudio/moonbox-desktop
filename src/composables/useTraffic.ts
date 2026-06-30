import { ref } from "vue";

import type { TrafficPayload } from "../types";

/** 滚动窗口长度（秒）；与后端 1s 采样频率对齐 */
const WINDOW_SIZE = 60;

/** 后端 payload 增量值；in_rate/out_rate 已是瞬时值，无需差分 */
export interface TrafficSnapshot {
  timestamp: number;
  in_rate: number;
  out_rate: number;
  connections: number;
}

/** 累计值 */
export const totalInBytes = ref(0);
export const totalOutBytes = ref(0);
/** 滚动窗口数据 */
export const trafficHistory = ref<TrafficSnapshot[]>([]);
/** 最新一次 payload，供组件单值展示 */
export const latestTraffic = ref<TrafficSnapshot>({
  timestamp: 0,
  in_rate: 0,
  out_rate: 0,
  connections: 0,
});

/**
 * 处理一次后端 payload：更新累计值与滚动窗口。
 *
 * 由 `useAppEvents` 在 traffic 事件触发时调用；本函数不负责 listen 注册。
 */
export function handleTrafficPayload(p: TrafficPayload) {
  totalInBytes.value = p.total_in_bytes;
  totalOutBytes.value = p.total_out_bytes;

  const now = Date.now();
  const snapshot: TrafficSnapshot = {
    timestamp: now,
    in_rate: p.in_rate,
    out_rate: p.out_rate,
    connections: p.connections,
  };
  latestTraffic.value = snapshot;

  const next = trafficHistory.value.concat(snapshot);
  if (next.length > WINDOW_SIZE) {
    next.splice(0, next.length - WINDOW_SIZE);
  }
  trafficHistory.value = next;
}

/** frpc 停止时调用：清零累计与窗口（图表重置） */
export function resetTraffic() {
  totalInBytes.value = 0;
  totalOutBytes.value = 0;
  trafficHistory.value = [];
  latestTraffic.value = { timestamp: 0, in_rate: 0, out_rate: 0, connections: 0 };
}

/** 把字节数格式化为人类可读速率字符串，如 "12.3 KB/s" */
export function formatRate(bytesPerSec: number): string {
  if (bytesPerSec < 1024) return `${bytesPerSec} B/s`;
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
  if (bytesPerSec < 1024 * 1024 * 1024)
    return `${(bytesPerSec / 1024 / 1024).toFixed(2)} MB/s`;
  return `${(bytesPerSec / 1024 / 1024 / 1024).toFixed(2)} GB/s`;
}

/** 把累计字节数格式化为人类可读容量字符串，如 "1.23 GB" */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
