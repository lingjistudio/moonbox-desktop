//! frpc 与本地真实服务之间的 TCP 中转层。
//!
//! 设计动机：frpc v0.69.1 客户端的 `/api/status` 不暴露任何流量/速率字段
//! （详见 docs/plans/2026-06-30-frpc-traffic-monitor.md）。要让用户看到吞吐
//! 曲线，MoonProxy 必须自己「夹」在 frpc 与用户服务之间：frpc 不再直连用户
//! 的 `localIP:localPort`，而是连到 MoonProxy 自开的动态端口；MoonProxy 双向
//! copy 字节并按方向计数。
//!
//! 该模块对所有 TCP 类代理（含 HTTP/HTTPS——它们本质也是 TCP 流）启用中转；
//! UDP 首版不中转。

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;
use tauri::{Emitter, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// 单条代理的运行态：accept 任务句柄 + 双向字节累计 + 当前活跃连接计数。
///
/// `in_bytes`：frpc ← 用户服务方向（用户视角 upload，上行）
/// `out_bytes`：用户服务 ← frpc 方向（用户视角 download，下行）
pub struct RelayEntry {
    /// 用户真实本地地址 `host:port`
    pub upstream: String,
    pub in_bytes: AtomicU64,
    pub out_bytes: AtomicU64,
    pub connections: AtomicI64,
    /// accept 循环任务句柄；stop_relay abort 它即关闭 listener（listener own 在任务内）
    pub accept_task: Mutex<Option<JoinHandle<()>>>,
}

impl RelayEntry {
    fn new(upstream: String) -> Self {
        Self {
            upstream,
            in_bytes: AtomicU64::new(0),
            out_bytes: AtomicU64::new(0),
            connections: AtomicI64::new(0),
            accept_task: Mutex::new(None),
        }
    }
}

/// 全部代理中转层的集合，挂在 `FrpcState` 上。
pub struct RelayState {
    /// 按 proxy 名字索引；同名 proxy 重启时覆盖（实际 build_toml 已校验唯一）
    pub entries: Mutex<Vec<(String, Arc<RelayEntry>)>>,
}

impl Default for RelayState {
    fn default() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }
}

/// 通过 `frpc://traffic` 事件广播给前端的载荷。
#[derive(Serialize, Clone, Default)]
pub struct TrafficPayload {
    pub total_in_bytes: u64,
    pub total_out_bytes: u64,
    pub in_rate: u64,
    pub out_rate: u64,
    pub connections: i64,
}

/// 连接计数 RAII guard：构造 +1，drop -1，确保任何退出路径（含早期 return / 错误）都配对。
struct ConnGuard<'a>(&'a AtomicI64);
impl<'a> ConnGuard<'a> {
    fn new(c: &'a AtomicI64) -> Self {
        c.fetch_add(1, Ordering::Relaxed);
        Self(c)
    }
}
impl Drop for ConnGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// 单条 frpc → 中转端口的 TCP 连接处理：再连到用户真实服务，双向 copy 并计数。
///
/// 任何方向 IO 出错都终止本连接；连接计数由 ConnGuard 在作用域进出时配对加减。
async fn handle_relay_conn(mut frpc_conn: TcpStream, entry: Arc<RelayEntry>) {
    let mut user_conn = match TcpStream::connect(&entry.upstream).await {
        Ok(s) => s,
        Err(_) => return,
    };
    let _guard = ConnGuard::new(&entry.connections);

    // 拆分两条 half-conn，按方向计数。
    // copy_bidirectional 返回 (user_to_frpc, frpc_to_user) 字节数；
    // 我们要把方向对齐到「用户视角」：
    //   - user_conn → frpc_conn：用户服务流向 frpc，= in（upload，上行）
    //   - frpc_conn → user_conn：frpc 流向用户服务，= out（download，下行）
    let (user_to_frpc, frpc_to_user) = {
        let (mut frpc_rd, mut frpc_wr) = frpc_conn.split();
        let (mut user_rd, mut user_wr) = user_conn.split();
        copy_bidirectional_counted(
            &mut user_rd, &mut frpc_wr, &entry.in_bytes,
            &mut frpc_rd, &mut user_wr, &entry.out_bytes,
        ).await
    };

    let _ = user_to_frpc;
    let _ = frpc_to_user;
}

/// 与 `tokio::io::copy_bidirectional` 等价，但分别在两个方向上累加字节计数。
///
/// 实现策略：不用 `copy_bidirectional`（其内部不暴露计数），改为并发跑两个
/// `copy_counted`，先完成者通过 `tokio::select!` 取消另一条。
async fn copy_bidirectional_counted<R1, W1, R2, W2>(
    user_rd: &mut R1,
    frpc_wr: &mut W1,
    in_counter: &AtomicU64,
    frpc_rd: &mut R2,
    user_wr: &mut W2,
    out_counter: &AtomicU64,
) -> (u64, u64)
where
    R1: tokio::io::AsyncRead + Unpin + ?Sized,
    W1: tokio::io::AsyncWrite + Unpin + ?Sized,
    R2: tokio::io::AsyncRead + Unpin + ?Sized,
    W2: tokio::io::AsyncWrite + Unpin + ?Sized,
{
    let in_task = copy_one_way(user_rd, frpc_wr, in_counter);
    let out_task = copy_one_way(frpc_rd, user_wr, out_counter);
    tokio::pin!(in_task);
    tokio::pin!(out_task);

    let mut in_bytes = 0u64;
    let mut out_bytes = 0u64;
    loop {
        tokio::select! {
            res = &mut in_task => {
                if let Ok(n) = res { in_bytes += n; }
                break;
            }
            res = &mut out_task => {
                if let Ok(n) = res { out_bytes += n; }
                break;
            }
        }
    }
    (in_bytes, out_bytes)
}

/// 单向 copy：读完立即写、立即累加计数器；与 `tokio::io::copy` 等价但加计数。
async fn copy_one_way<R, W>(
    rd: &mut R,
    wr: &mut W,
    counter: &AtomicU64,
) -> std::io::Result<u64>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
    W: tokio::io::AsyncWrite + Unpin + ?Sized,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut buf = vec![0u8; 8 * 1024];
    let mut total = 0u64;
    loop {
        let n = rd.read(&mut buf).await?;
        if n == 0 {
            return Ok(total);
        }
        wr.write_all(&buf[..n]).await?;
        total += n as u64;
        counter.fetch_add(n as u64, Ordering::Relaxed);
    }
}

use crate::types::{ProxyConfig, StartArgs};

/// 为 `args.proxies` 中所有可中转代理（TCP/HTTP/HTTPS）开 listener 并 spawn 接受循环；
/// 返回「改写后的 args」——把每条可中转代理的 `localIP/localPort` 替换为
/// `127.0.0.1:<relay_port>`，UDP 保持原样。
///
/// 调用时机：`start_frpc` 在 `build_toml` 之前。失败（listener bind 失败）逐条
/// 记录并跳过该条（不阻塞其他 proxy）。
pub async fn start_relay(
    app: &AppHandle,
    args: &StartArgs,
) -> (StartArgs, RelayState) {
    let state = RelayState::default();
    let mut rewritten = args.clone();

    let mut entries = state.entries.lock().await;
    for p in &mut rewritten.proxies {
        let (name, local_ip, local_port) = match p {
            ProxyConfig::Tcp { name, local_ip, local_port, .. }
            | ProxyConfig::Http { name, local_ip, local_port, .. }
            | ProxyConfig::Https { name, local_ip, local_port, .. } => {
                (name.clone(), local_ip.clone(), *local_port)
            }
            // UDP 首版不中转
            ProxyConfig::Udp { .. } => continue,
        };

        // 绑定动态端口
        let listener = match TcpListener::bind(("127.0.0.1", 0u16)).await {
            Ok(l) => l,
            Err(e) => {
                crate::frpc_state::emit_log(app, "system", format!("中转端口绑定失败，跳过 {name}：{e}"));
                continue;
            }
        };
        let relay_port = match listener.local_addr() {
            Ok(a) => a.port(),
            Err(_) => continue,
        };

        // 改写 args 中本条 proxy 的 localIP/localPort
        let new_ip = "127.0.0.1".to_string();
        match p {
            ProxyConfig::Tcp { local_ip, local_port, .. }
            | ProxyConfig::Http { local_ip, local_port, .. }
            | ProxyConfig::Https { local_ip, local_port, .. } => {
                *local_ip = new_ip;
                *local_port = relay_port;
            }
            ProxyConfig::Udp { .. } => {
                // 上方循环顶部已 `continue` 跳过 UDP；此处逻辑不可达
            }
        }

        let entry = Arc::new(RelayEntry::new(format!("{local_ip}:{local_port}")));
        let entry_for_loop = entry.clone();

        // spawn 接受循环：listener 被 move 进任务，abort 即关闭端口
        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((conn, _)) => {
                        let e = entry_for_loop.clone();
                        tokio::spawn(handle_relay_conn(conn, e));
                    }
                    Err(_) => break,
                }
            }
        });
        {
            let mut guard = entry.accept_task.lock().await;
            *guard = Some(handle);
        }

        entries.push((name, entry));
    }
    drop(entries);
    (rewritten, state)
}

/// 关闭所有 listener 与活跃中转任务。
///
/// 调用时机：`stop_frpc` 或 `Terminated`。abort accept 任务即关闭 listener
/// （listener own 在任务内）；活跃中转任务在 IO 出错时自然结束。
pub async fn stop_relay(state: &RelayState) {
    let mut entries = state.entries.lock().await;
    for (_, entry) in entries.drain(..) {
        if let Some(h) = entry.accept_task.lock().await.take() {
            h.abort();
        }
    }
}

/// 每 1 秒采样一次累计字节，差分出瞬时速率，聚合 connections，
/// 通过 `frpc://traffic` 广播。
///
/// 退出条件与 `poll_conn_state` 对齐：
/// - `relay` 被清空（stop / Terminated 已接管）
/// - `poll_gen` 与起始记录的不一致（新一轮 start 已接管）
///
/// **必须由调用方 `tauri::async_runtime::spawn` 包装**——函数本身是无界循环，
/// 直接 await 会挂住 start_frpc 命令的返回；spawn 后由 poll_gen 自然退出。
pub async fn poll_traffic(app: AppHandle, start_gen: u64) {
    const TICK: Duration = Duration::from_secs(1);
    loop {
        tokio::time::sleep(TICK).await;
        let state = match app.try_state::<crate::frpc_state::FrpcState>() {
            Some(s) => s,
            None => break,
        };
        if state.poll_gen.load(Ordering::Acquire) != start_gen {
            break;
        }
        // relay 锁保护：stop_frpc 会把 relay 置 None
        let payload = {
            let guard = state.relay.lock().await;
            match guard.as_ref() {
                Some(r) => sample(r, &state).await,
                None => break,
            }
        };
        let _ = app.emit("frpc://traffic", payload);
    }
}

async fn sample(
    relay: &RelayState,
    state: &crate::frpc_state::FrpcState,
) -> TrafficPayload {
    let entries = relay.entries.lock().await;
    let mut total_in = 0u64;
    let mut total_out = 0u64;
    let mut conns = 0i64;
    for (_, e) in entries.iter() {
        total_in += e.in_bytes.load(Ordering::Relaxed);
        total_out += e.out_bytes.load(Ordering::Relaxed);
        conns += e.connections.load(Ordering::Relaxed);
    }
    drop(entries);

    let last_in = state.last_in_bytes.swap(total_in, Ordering::Relaxed);
    let last_out = state.last_out_bytes.swap(total_out, Ordering::Relaxed);
    let in_rate = total_in.saturating_sub(last_in);
    let out_rate = total_out.saturating_sub(last_out);

    TrafficPayload {
        total_in_bytes: total_in,
        total_out_bytes: total_out,
        in_rate,
        out_rate,
        connections: conns,
    }
}
