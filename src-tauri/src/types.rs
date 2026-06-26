//! 客户端启动参数与代理规则——跨模块共享的数据载体。
//!
//! 同时承担两个角色：
//! 1. `tauri::command` 的入参 / 返回值类型，与前端 `src/types.ts` 对齐
//! 2. `config.rs` 生成 TOML、`process.rs` 启动 frpc 时的数据载体

use serde::{Deserialize, Serialize};

/// 客户端启动参数：与服务商建立连接 + 一组穿透规则。
///
/// `Option<String>` 字段在前端 `toArgs()` 中由空字符串转换而来，后端据此
/// 决定是否写入对应 TOML 字段（详见 `config::build_toml`）。
#[derive(Deserialize, Serialize, Clone)]
pub struct StartArgs {
    /// 内置服务商 id（"builtin:..."）或 "custom"
    #[serde(default)]
    pub provider_id: Option<String>,
    /// 自定义服务商显示名（仅当 `provider_id == "custom"` 时有意义）
    #[serde(default)]
    pub custom_name: Option<String>,
    /// FRP 服务端地址，例如 frp.example.com
    pub server_addr: String,
    /// FRP 服务端端口，例如 7000
    pub server_port: u16,
    /// 客户端与 FRP 服务端建立连接时使用的身份验证密钥
    pub token: Option<String>,
    /// 客户端唯一标识，需在服务端唯一
    pub user: Option<String>,
    /// 要穿透的代理规则列表（TCP/UDP/HTTP/HTTPS）
    pub proxies: Vec<ProxyConfig>,
}

/// 单条代理规则。`proxy_type` 经 `#[serde(rename = "type")]` 与前端字段名对齐。
#[derive(Deserialize, Serialize, Clone)]
pub struct ProxyConfig {
    /// 代理名称，需唯一
    pub name: String,
    /// 代理类型：`tcp` / `udp` / `http` / `https`（由 [`SUPPORTED_PROXY_TYPES`] 约束）
    #[serde(rename = "type")]
    pub proxy_type: String,
    /// 本地服务地址
    pub local_ip: String,
    /// 本地服务端口
    pub local_port: u16,
    /// 公网访问端口
    pub remote_port: u16,
}

/// 受支持的代理类型白名单。`proxy_health.rs` 的探测策略与 `config.rs` 的
/// 校验逻辑都应基于此判定，避免与前端 TS 端的字面量字符串失同步。
pub const SUPPORTED_PROXY_TYPES: [&str; 4] = ["tcp", "udp", "http", "https"];

/// 判断字符串是否为受支持的代理类型。
///
/// `config.rs::build_toml` 借此在生成 frpc.toml 前拒绝未知类型，避免无效配置
/// 写盘；`proxy_health.rs` 借此在探测时跳过未知类型（标记"未检测"）。两处共用
/// 这条白名单，确保与前端 TS 端的字面量字符串不脱节。
pub fn is_supported_proxy_type(t: &str) -> bool {
    SUPPORTED_PROXY_TYPES.iter().any(|s| *s == t)
}
