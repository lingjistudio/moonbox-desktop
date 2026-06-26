import { reactive } from "vue";

import type { FrpcConfig } from "../types";

/** 用户配置：服务商 + 代理规则。由 `commands/config.ts` 从 `config.store.json` 加载 / 写回。 */
export const config = reactive<FrpcConfig>({
  provider_id: "",
  custom_name: "",
  server_addr: "",
  server_port: 7000,
  token: "",
  user: "",
  proxies: [],
});

/** 是否已完成初始配置（有服务端地址且至少一条代理）。 */
export function isConfigured(): boolean {
  return (
    config.server_addr.trim().length > 0 &&
    config.server_port > 0 &&
    config.proxies.length > 0
  );
}

/** 序列化为 Rust 端 StartArgs 格式：trim / Number 化，空字符串 → null。 */
export function toArgs() {
  return {
    provider_id: (config.provider_id ?? "").trim() || null,
    custom_name: (config.custom_name ?? "").trim() || null,
    server_addr: (config.server_addr ?? "").trim(),
    server_port: Number(config.server_port),
    token: (config.token ?? "").trim() || null,
    user: (config.user ?? "").trim() || null,
    proxies: config.proxies.map((p) => ({
      name: p.name.trim(),
      type: p.type.trim(),
      local_ip: p.local_ip.trim(),
      local_port: Number(p.local_port),
      remote_port: Number(p.remote_port),
    })),
  };
}
