//! 服务端 TCP 连通性 + 延迟探测。
//!
//! 在「服务商」Tab 给用户一个「测试」按钮：单次 TCP 握手到
//! `server_addr:server_port`，返回毫秒级延迟。用途是保存前预先验证
//! 服务端是否可达、网络顺不顺，避免启动 frpc 后才发现穿透失败。

use std::time::{Duration, Instant};

use serde::Serialize;

/// 单次握手的连接超时。
///
/// 远距离 / 跨境 / 慢网下 1.5s 容易误报失败；用户主动点击的场景可以多等一点。
const PROBE_TIMEOUT: Duration = Duration::from_millis(3000);

/// `error_kind` 取值：前端按此用 i18n key 翻译，避免硬编码中文文案。
///
/// 枚举值用 snake_case 字符串；新增分类时同步前端 `formatLatency` 分支。
#[derive(Serialize, Clone)]
pub struct LatencyResult {
    /// 握手是否成功
    pub ok: bool,
    /// 成功时的耗时毫秒；失败时填 0
    pub latency_ms: u64,
    /// 失败原因分类；成功时为 `None`
    pub error_kind: Option<String>,
}

fn probe_once(addr: &str, port: u16) -> LatencyResult {
    let endpoint = format!("{addr}:{port}");
    let addrs = match std::net::ToSocketAddrs::to_socket_addrs(&endpoint) {
        Ok(iter) => iter.collect::<Vec<_>>(),
        Err(_) => {
            return LatencyResult {
                ok: false,
                latency_ms: 0,
                error_kind: Some("resolve".into()),
            };
        }
    };
    // 单次握手语义：只测第一个解析到的 IP。
    //
    // 多 IP 主机上 `to_socket_addrs` 可能返回 v4 + v6 等多个地址；若逐个
    // 串行 connect，会把「3s 超时」累积放大为 N×3s，违反「单次」承诺。
    // 这里只取第一个，与 DNS 解析客户端实际使用的目标对齐。
    let Some(a) = addrs.into_iter().next() else {
        return LatencyResult {
            ok: false,
            latency_ms: 0,
            error_kind: Some("resolve".into()),
        };
    };
    let started = Instant::now();
    match std::net::TcpStream::connect_timeout(&a, PROBE_TIMEOUT) {
        Ok(_) => {
            let ms = started.elapsed().as_millis() as u64;
            LatencyResult {
                ok: true,
                latency_ms: ms,
                error_kind: None,
            }
        }
        Err(e) => {
            let kind = match e.kind() {
                std::io::ErrorKind::TimedOut => "timeout",
                std::io::ErrorKind::ConnectionRefused => "refused",
                _ => "unreachable",
            };
            LatencyResult {
                ok: false,
                latency_ms: 0,
                error_kind: Some(kind.into()),
            }
        }
    }
}

/// 探测本机到 `server_addr:server_port` 的单次 TCP 握手耗时。
///
/// `TcpStream::connect_timeout` 是阻塞 IO，必须用 `spawn_blocking` 丢到
/// blocking 线程池；用 `spawn` 会污染 tokio 运行时（参见 `proxy_health.rs`
/// 注释与 src-tauri/AGENTS.md §5.4 并发模型）。
#[tauri::command]
pub async fn probe_server_latency(
    server_addr: String,
    server_port: u16,
) -> Result<LatencyResult, String> {
    tauri::async_runtime::spawn_blocking(move || probe_once(&server_addr, server_port))
        .await
        .map_err(|e| format!("测试任务异常退出：{e}"))
}
