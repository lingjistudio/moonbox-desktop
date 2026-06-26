//! frpc 与 frps 的连接状态机。
//!
//! 所有派生都基于 frpc 自带 webServer `/api/status`，不依赖 Tauri 自维护。
//!
//! 推导规则（`poll_conn_state`，3s 间隔）：
//! - 子进程已退或新一轮 polling 已接管 → 退出
//! - 任意代理 `status="running"` → `Connected`（清空错误）
//! - Connected 掉线（running 全消失） → 回到 `Connecting` 并刷新 `started_at`，给 frpc 30s 自愈窗口
//! - 否则按 `started_at` 经过时长决定：≤30s 保持 `Connecting`，>30s 升级为 `Error`

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandChild;

/// frpc 启动后逐行写入；同时 500 条环形缓冲，供独立日志窗口打开时拉取历史。
pub const LOG_BUFFER_LIMIT: usize = 500;

/// 与 `frpc://log` 事件载荷同形的公开类型，`get_logs` 命令与事件共用。
#[derive(Serialize, Clone)]
pub struct LogEntry {
    pub line: String,
    pub stream: String,
}

/// frpc 与 frps 的连接状态。`Connected` 是成功状态的唯一权威来源：
/// 必须有代理 `status="running"`。
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FrpcConnState {
    Stopped,
    Connecting,
    Connected,
    Error,
}

impl FrpcConnState {
    pub fn as_str(self) -> &'static str {
        match self {
            FrpcConnState::Stopped => "stopped",
            FrpcConnState::Connecting => "connecting",
            FrpcConnState::Connected => "connected",
            FrpcConnState::Error => "error",
        }
    }
}

impl Default for FrpcConnState {
    fn default() -> Self {
        FrpcConnState::Stopped
    }
}

/// 进程级 + 连接级共享状态。命令通过 `tauri::State<'_, FrpcState>` 访问。
pub struct FrpcState {
    /// sidecar 子进程句柄；`None` 表示未启动 / 已终止
    pub child: Mutex<Option<CommandChild>>,
    /// 当前连接态
    pub conn: Mutex<FrpcConnState>,
    /// 进入 `Error` 时的用户提示文案
    pub error_msg: Mutex<Option<String>>,
    /// spawn 时刻；polling task 据此做 30s 兜底判定
    pub started_at: Mutex<Option<Instant>>,
    /// 每次 `mark_connecting` 自增；旧 polling task 据此自动退出
    pub poll_gen: AtomicU64,
    /// 最近 N 条 frpc 日志（`frpc://log` 事件同源）；日志窗口打开时一次性拉取
    pub logs: Mutex<VecDeque<LogEntry>>,
}

impl Default for FrpcState {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            conn: Mutex::new(FrpcConnState::default()),
            error_msg: Mutex::new(None),
            started_at: Mutex::new(None),
            poll_gen: AtomicU64::new(0),
            logs: Mutex::new(VecDeque::new()),
        }
    }
}

/// 把一条日志写入环形缓冲（同时由调用方 emit `frpc://log`）。
pub fn push_log(state: &FrpcState, line: String, stream: &str) {
    if let Ok(mut buf) = state.logs.lock() {
        buf.push_back(LogEntry {
            line,
            stream: stream.to_string(),
        });
        while buf.len() > LOG_BUFFER_LIMIT {
            buf.pop_front();
        }
    }
}

/// 写日志到环形缓冲并广播 `frpc://log` 事件给前端。
///
/// start_frpc 与其 IO 任务、stop_frpc 等多处调用，统一入口。
pub fn emit_log(app: &AppHandle, stream: &str, line: String) {
    let state = app.state::<FrpcState>();
    push_log(state.inner(), line.clone(), stream);
    let _ = app.emit(
        "frpc://log",
        LogEntry {
            line,
            stream: stream.to_string(),
        },
    );
}

#[derive(Serialize, Clone)]
pub struct StatusPayload {
    pub status: String,
    pub error: Option<String>,
}

/// 广播 `frpc://status` 事件，载荷由当前 `FrpcState` 推导。
pub fn emit_status(app: &AppHandle, state: &FrpcState) {
    let status = state
        .conn
        .lock()
        .map(|g| g.as_str().to_string())
        .unwrap_or_else(|_| "stopped".to_string());
    let error = state.error_msg.lock().ok().and_then(|g| g.clone());
    let _ = app.emit("frpc://status", StatusPayload { status, error });
}

/// 把连接状态字段整体重置为 `Stopped`。`stop_frpc` 与 `Terminated` 都调它。
pub fn reset_to_stopped(state: &FrpcState) {
    if let Ok(mut conn) = state.conn.lock() {
        *conn = FrpcConnState::Stopped;
    }
    if let Ok(mut err) = state.error_msg.lock() {
        *err = None;
    }
    if let Ok(mut started) = state.started_at.lock() {
        *started = None;
    }
}

/// 把连接状态字段切到 `Connecting`、记录起始时刻并 bump generation。
/// `start_frpc` 在 spawn 后调它；旧 polling task 在下次循环检测到 gen 变化即退出。
pub fn mark_connecting(state: &FrpcState) {
    if let Ok(mut conn) = state.conn.lock() {
        *conn = FrpcConnState::Connecting;
    }
    if let Ok(mut err) = state.error_msg.lock() {
        *err = None;
    }
    if let Ok(mut started) = state.started_at.lock() {
        *started = Some(Instant::now());
    }
    state.poll_gen.fetch_add(1, Ordering::AcqRel);
}

/// 探测 frpc webServer `/api/status` 响应中是否存在 `status="running"` 的代理。
/// 响应顶层按代理类型分组（tcp/http/udp/...），每组值是数组。
fn has_running_proxy(body: &serde_json::Value) -> bool {
    if let Some(obj) = body.as_object() {
        for (_, arr) in obj {
            if let Some(list) = arr.as_array() {
                for item in list {
                    if item["status"].as_str() == Some("running") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 3 秒间隔探测 frpc 与 frps 的连接状态。
///
/// 任何状态变化都通过 `emit_status` 广播。退出条件（任一）：
/// - 子进程终止（`child.is_none()`）
/// - 新一轮 polling 已接管（`poll_gen` 与起始时记录的不一致）
///
/// Connected 掉线（running 全消失）会被重置回 Connecting 并刷新 `started_at`，
/// 给 frpc 一个 30s 自愈窗口；超过 30s 仍未恢复则升级为 Error。
pub async fn poll_conn_state(app: AppHandle) {
    let client = match reqwest::Client::builder()
        .user_agent("Moonbox/0.1 (frpc-state)")
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    const URL: &str = "http://127.0.0.1:7400/api/status";
    const POLL_INTERVAL: Duration = Duration::from_secs(3);
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

    let my_gen = match app.try_state::<FrpcState>() {
        Some(s) => s.poll_gen.load(Ordering::Acquire),
        None => return,
    };

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        let state = match app.try_state::<FrpcState>() {
            Some(s) => s,
            None => break,
        };
        if state.poll_gen.load(Ordering::Acquire) != my_gen {
            break;
        }
        if !state.child.lock().map(|g| g.is_some()).unwrap_or(false) {
            break;
        }
        let running = match client
            .get(URL)
            .basic_auth("admin", Some("admin"))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => {
                let body: serde_json::Value =
                    r.json().await.unwrap_or(serde_json::Value::Null);
                has_running_proxy(&body)
            }
            _ => false,
        };
        let cur = state
            .conn
            .lock()
            .map(|g| *g)
            .unwrap_or(FrpcConnState::Stopped);
        let elapsed = state
            .started_at
            .lock()
            .ok()
            .and_then(|g| g.map(|t| t.elapsed()))
            .unwrap_or_default();
        let new = if running {
            FrpcConnState::Connected
        } else if cur == FrpcConnState::Connected {
            // Connected 掉线：重置计时给 frpc 30s 自愈窗口
            if let Ok(mut started) = state.started_at.lock() {
                *started = Some(Instant::now());
            }
            FrpcConnState::Connecting
        } else if elapsed > CONNECT_TIMEOUT {
            FrpcConnState::Error
        } else {
            FrpcConnState::Connecting
        };
        if new != cur {
            if let Ok(mut conn) = state.conn.lock() {
                *conn = new;
            }
            if let Ok(mut err) = state.error_msg.lock() {
                match new {
                    FrpcConnState::Error => {
                        *err = Some(
                            "frpc 启动 30 秒内未连接到服务端，请检查地址 / 端口 / Token".into(),
                        );
                    }
                    FrpcConnState::Connected => *err = None,
                    _ => {}
                }
            }
            emit_status(&app, state.inner());
        }
    }
}
