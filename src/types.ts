/** 前后端共享类型。字段命名严格 snake_case，与后端 Rust 类型一一对应。 */

export interface ProxyConfig {
  name: string;
  type: string;
  local_ip: string;
  local_port: number;
  remote_port: number;
}

export interface FrpcConfig {
  provider_id: string;
  custom_name: string;
  server_addr: string;
  server_port: number;
  token: string;
  user: string;
  proxies: ProxyConfig[];
}

/**
 * 服务商：内置不可改地址/端口；自定义可全部编辑。
 * 内置服务商 id 必须以 "builtin:" 前缀开头，便于与自定义 id 区分。
 */
export interface Provider {
  id: string;
  name: string;
  builtin: boolean;
  server_addr: string;
  server_port: number;
  /** 可选：内置服务商 JSON 不必填（运行时兜底为 ""）；自定义时为用户输入的 user 字段。 */
  user?: string;
  /** 用户名是否必填；false 时 UI 隐藏用户名输入框。 */
  username_required: boolean;
}

/**
 * 应用级偏好。与服务端的 frpc 配置（FrpcConfig）解耦，
 * 通过 tauri-plugin-store 持久化到 prefs.json。
 */
export interface Prefs {
  /** 开机启动（OS 注册表 / LaunchAgent 实际状态） */
  auto_launch: boolean;
  /** 静默启动：开机自启时隐藏到系统托盘 */
  silent_start: boolean;
  /** 开机自动连接：OS 自启后自动拉起 frpc（仅 --auto-launched 触发） */
  auto_connect: boolean;
  /** 定时连接配置 */
  schedule: Schedule;
  /** 界面语言 code（"zh-CN" | "en"）；后端默认 "zh-CN" */
  language: string;
}

/**
 * 定时连接配置：按星期多选 + 起止时间。
 * 后端 `chrono::Weekday::num_days_from_monday` 与 `weekdays` 下标一一对应。
 */
export interface Schedule {
  /** 主开关；false 时调度器与启动补跑均跳过 */
  enabled: boolean;
  /** 一周 7 天，下标 0=周一 … 6=周日 */
  weekdays: [boolean, boolean, boolean, boolean, boolean, boolean, boolean];
  /** 启动时间，24 小时制 "HH:MM" */
  start_time: string;
  /** 断开时间，24 小时制 "HH:MM" */
  stop_time: string;
}

/**
 * frpc 与 frps 的连接状态。
 * - `stopped`：进程未运行
 * - `connecting`：进程已起，但还没在 frps 注册任何代理
 * - `connected`：frpc 报告至少一条代理 status="running"
 * - `error`：30s 内未观测到代理 running（地址/端口/Token 错或网络不通）
 *
 * 成功状态必须由后端通过 frpc 自身探测派发，前端不自行猜测。
 */
export type FrpcStatus = "stopped" | "connecting" | "connected" | "error";

/** frpc 日志一条：流类型 + 文本行。 */
export interface LogEntry {
  stream: "stdout" | "stderr" | "system";
  line: string;
}
