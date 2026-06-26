//! 按星期定时启停 frpc 的调度器。
//!
//! 设计目标：
//! - **幂等**：触发时检查当前 frpc 状态，已在跑则启动触发跳过；未在跑则停止
//!   触发跳过。避免在主开关被用户手动覆盖时调度器"插队"重连。
//! - **分钟对齐**：tokio 任务睡到下一个整分，唤醒后比对当前 HH:MM 与
//!   `start_time` / `stop_time`。比"每秒扫一次"省 CPU，又不会错过触发点。
//! - **区间判定**：tick 用 `[start, stop)` 区间判定启动、`>= stop` 判定停止。
//!   产品意图是「区间内大概触发」，不要求整分精确——即便 OS 冻结（macOS
//!   App Nap / Windows Modern Standby）或长 sleep 抖动跨过整分，只要下一
//!   分钟仍在窗口内就会补跑。重复触发由 `try_fire_start` / `try_fire_stop`
//!   内部的 `child.is_some()` 幂等检查拦截，无需额外的日期 flag。
//! - **热加载**：每次 tick 都从 store 重新读 `Prefs`；用户在 UI 改完保存，
//!   下一分钟就生效，无需重启应用。
//! - **启动补跑**：setup hook 中调用 `maybe_catch_up_start`，若当前正处在
//!   调度"应运行"窗口内且 frpc 未在跑，则启动 frpc。覆盖应用刚启动 / 系统
//!   自启 / 应用崩溃后被用户重新打开等场景。
//!
//! **已知限制**：调度器与用户实时操作之间有毫秒级 TOCTOU 间隙——用户在
//! 窗口内手动停止后，下一分钟 tick 会重新拉起 frpc。调度精度为分钟级且
//! 用户撞秒概率极低，v1 接受此限制。

use std::time::Duration;

use chrono::{Datelike, Local, Timelike};
use tauri::{AppHandle, Manager};

use crate::prefs::{self, Schedule};

/// 距离下一分钟整点还有多少秒（取值范围 1..=60）。
fn secs_to_next_minute() -> u64 {
    let now = Local::now();
    let s = now.second() as u64;
    if s == 0 {
        60
    } else {
        60 - s
    }
}

/// 把 `chrono::Weekday` 映射到 `Schedule::weekdays` 的下标 0..7（Mon=0）。
fn weekday_idx(wd: chrono::Weekday) -> usize {
    wd.num_days_from_monday() as usize
}

/// 把当前本地时间格式化为 "HH:MM"。
fn fmt_hhmm(now: chrono::DateTime<Local>) -> String {
    format!("{:02}:{:02}", now.hour(), now.minute())
}

/// 当前时间是否落在调度"应运行"窗口内（含星期判定）。
/// `enabled=false` 时直接返回 false。
pub fn in_window(schedule: &Schedule, now: chrono::DateTime<Local>) -> bool {
    if !schedule.enabled {
        return false;
    }
    if !schedule
        .weekdays
        .get(weekday_idx(now.weekday()))
        .copied()
        .unwrap_or(false)
    {
        return false;
    }
    let cur = fmt_hhmm(now);
    cur >= schedule.start_time && cur < schedule.stop_time
}

/// 应用启动时调用：若处在调度窗口内且 frpc 未在跑，则异步触发启动。
///
/// 不阻塞 setup hook——内部 spawn 异步任务执行 `start_frpc`。
/// `prefs` 由 setup hook 顶层算一次后传入（详见 `src-tauri/AGENTS.md §5.2.1`）；
/// `spawn_scheduler` 的分钟 tick 仍每次重读 prefs 以支持热加载，与本函数
/// 不同源，不要为一致性把 tick 也改成传 prefs。
pub fn maybe_catch_up_start(app: &AppHandle, prefs: &prefs::Prefs) {
    if !in_window(&prefs.schedule, Local::now()) {
        return;
    }
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        try_fire_start(&app_clone).await;
    });
}

/// setup hook 调用：派生长驻 tokio 任务，每分钟整点 tick 一次。
pub fn spawn_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(secs_to_next_minute())).await;
            tick(&app).await;
        }
    });
}

async fn tick(app: &AppHandle) {
    let prefs = prefs::load(app);
    let schedule = &prefs.schedule;
    if !schedule.enabled {
        return;
    }
    let now = Local::now();
    if !schedule
        .weekdays
        .get(weekday_idx(now.weekday()))
        .copied()
        .unwrap_or(false)
    {
        return;
    }
    // 区间判定：`[start, stop)` 内每分钟都尝试启动，`>= stop` 每分钟都尝试
    // 停止。重复触发由 `try_fire_start` / `try_fire_stop` 内部 `child.is_some()`
    // 幂等检查拦截。相比 `cur == start_time` 精确等值，能容忍 sleep 抖动 /
    // OS 冻结跨分钟——产品语义是「区间内大概触发」，不要求整分精度。
    let cur = fmt_hhmm(now);
    if cur >= schedule.start_time && cur < schedule.stop_time {
        try_fire_start(app).await;
    } else if cur >= schedule.stop_time {
        try_fire_stop(app);
    }
}

/// 触发启动：已运行则跳过（幂等）；未配置服务端则记日志跳过。
async fn try_fire_start(app: &AppHandle) {
    let state = match app.try_state::<crate::frpc_state::FrpcState>() {
        Some(s) => s,
        None => return,
    };
    let already_running = state
        .child
        .lock()
        .map(|g| g.is_some())
        .unwrap_or_else(|e| {
            eprintln!("[scheduler] FrpcState.child mutex 中毒：{e}");
            false
        });
    if already_running {
        return;
    }
    let args = match crate::config::load_config(app.clone()) {
        Ok(Some(c)) => c,
        _ => {
            crate::frpc_state::emit_log(
                app,
                "system",
                "调度触发启动，但尚未配置服务端，跳过".into(),
            );
            return;
        }
    };
    let app_clone = app.clone();
    if let Err(e) = crate::process::start_frpc(app_clone, state, args).await {
        crate::frpc_state::emit_log(app, "system", format!("调度启动失败：{e}"));
    }
}

/// 触发停止：未运行则跳过（幂等）。
fn try_fire_stop(app: &AppHandle) {
    let state = match app.try_state::<crate::frpc_state::FrpcState>() {
        Some(s) => s,
        None => return,
    };
    let already_running = state
        .child
        .lock()
        .map(|g| g.is_some())
        .unwrap_or_else(|e| {
            eprintln!("[scheduler] FrpcState.child mutex 中毒：{e}");
            false
        });
    if !already_running {
        return;
    }
    if let Err(e) = crate::process::stop_frpc(app.clone(), state) {
        crate::frpc_state::emit_log(app, "system", format!("调度停止失败：{e}"));
    }
}
