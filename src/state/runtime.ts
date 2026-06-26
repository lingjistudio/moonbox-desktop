import { computed, ref, shallowReactive } from "vue";

import type { FrpcStatus, LogEntry } from "../types";

/** frpc 连接状态；`connected` 仅由后端通过 frpc `/api/status` 推导派发。 */
export const frpcStatus = ref<FrpcStatus>("stopped");
/** 进入 `error` 时的提示文案；状态变更即清空。 */
export const frpcError = ref<string | null>(null);

/**
 * frpc 进程是否已启动（connecting / connected / error 均算）。
 * CloseConfirm 弹窗据此决定关闭时是直接退出还是弹确认框。
 * UI 需要精确状态时直接读 `frpcStatus`。
 */
export const running = computed(() => frpcStatus.value !== "stopped");

/** 实时日志缓冲（前端自带 500 条上限）。 */
export const logs = shallowReactive<LogEntry[]>([]);
