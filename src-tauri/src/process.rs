//! frpc 子进程生命周期管理：启动 / 停止 / 状态查询 / 日志查询 / 孤儿清理。

use std::path::Path;

use sysinfo::System;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use crate::config::{build_toml, frpc_config_path};
use crate::frpc_state::{
    emit_log, emit_status, mark_connecting, reset_to_stopped, FrpcState, LogEntry, StatusPayload,
};
use crate::types::StartArgs;

/// 返回当前环形缓冲里的全部日志，供独立日志窗口打开时一次性拉取历史。
#[tauri::command]
pub fn get_logs(state: State<'_, FrpcState>) -> Result<Vec<LogEntry>, String> {
    let buf = state.logs.lock().map_err(|e| e.to_string())?;
    Ok(buf.iter().cloned().collect())
}

#[tauri::command]
pub async fn start_frpc(
    app: AppHandle,
    state: State<'_, FrpcState>,
    args: StartArgs,
) -> Result<(), String> {
    // 1. 解析 frpc.toml 路径（同步、纯路径计算，开销可忽略）
    let cfg_path = frpc_config_path(&app)?;

    // 2. 把两条阻塞工作丢到 blocking 线程池**并行**跑：
    //    - build_toml + fs::write：sync 字符串拼接 + 磁盘 IO
    //    - reap_orphan_frpc：sysinfo 全进程表扫描 + 命中后 sleep(200ms)
    //    两者彼此独立（reap 只比对 argv 路径，不依赖文件内容），
    //    不在 async 任务里直接执行会污染 tokio worker 线程，并阻塞 UI 响应。
    let write_path = cfg_path.clone();
    let write_handle = tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let toml = build_toml(&args)?;
        std::fs::write(&write_path, toml).map_err(|e| format!("写入配置失败：{e}"))
    });
    let reap_path = cfg_path.clone();
    let reap_handle = tauri::async_runtime::spawn_blocking(move || reap_orphan_frpc(&reap_path));

    write_handle
        .await
        .map_err(|e| format!("配置任务异常：{e}"))??;
    emit_log(
        &app,
        "system",
        format!("已生成配置：{}", cfg_path.display()),
    );
    let reaped = reap_handle
        .await
        .map_err(|e| format!("孤儿清理任务异常：{e}"))?;
    if reaped > 0 {
        emit_log(
            &app,
            "system",
            format!("已清理 {reaped} 个残留 frpc 进程（可能源自上次异常退出）"),
        );
    }

    // 3. 进入"启动"临界区：check + spawn sidecar + write child 原子化，
    //    避免两个并发 start_frpc 各自 spawn 一个 frpc 子进程导致端口冲突 /
    //    state 引用错乱 / 孤儿进程。spawn sidecar 是同步 syscall（fork + exec），
    //    耗时仅毫秒级，在 child 锁内执行对其他读 child 的线程影响可忽略。
    let mut rx = {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("核心引擎已在运行中".into());
        }
        let sidecar_command = app
            .shell()
            .sidecar("frpc")
            .map_err(|e| format!("找不到核心引擎：{e}"))?
            .args(["-c", cfg_path.to_string_lossy().as_ref()])
            .env("FRPC_LOG_LEVEL", "warn");
        match sidecar_command.spawn() {
            Ok((rx, child)) => {
                *guard = Some(child);
                rx
            }
            Err(e) => {
                return Err(format!("启动核心引擎失败：{e}"));
            }
        }
    };

    mark_connecting(state.inner());
    emit_status(&app, state.inner());
    // 日志时序：先前 emit "已生成配置" + "已清理 N 个" 在写盘 / reap 之后；
    // 此处发 "核心引擎已启动" 表示 sidecar 已成功 spawn；之后 stdout/stderr
    // 来自下方 spawn 的 IO 任务，与当前状态机无关（mark_connecting bump 的
    // poll_gen 已被新 polling task 接管，旧 task 在下次循环自动 break）。
    emit_log(&app, "system", "核心引擎已启动".into());

    // 4. 异步读取子进程输出
    let app_for_thread = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    let s = String::from_utf8_lossy(&bytes).to_string();
                    for line in s.lines() {
                        if !line.is_empty() {
                            emit_log(&app_for_thread, "stdout", line.to_string());
                        }
                    }
                }
                CommandEvent::Stderr(bytes) => {
                    let s = String::from_utf8_lossy(&bytes).to_string();
                    for line in s.lines() {
                        if !line.is_empty() {
                            emit_log(&app_for_thread, "stderr", line.to_string());
                        }
                    }
                }
                CommandEvent::Terminated(payload) => {
                    // 非预期退出判定：非 0 退出码或被信号 kill。
                    //
                    // 主动停止路径（`stop_frpc` / ExitRequested 兜底）会先在
                    // `state.child.lock().take()` 把 child 置 None，**再**调用
                    // `child.kill()` 触发 Terminated。时序保证 Terminated 到达时
                    // state.child 必为 None。故 child=Some 即说明非主动停止——
                    // frpc 自发崩溃 / OS 强杀 / OOM 等场景。
                    //
                    // 已知 TOCTOU 边界（接受不修，概率极低）：用户点 stop 与 frpc
                    // 自发崩溃几乎同时发生时，stop_frpc 的 take() 还没跑完，IO
                    // task 读到 child=Some → 上报一次"假崩溃"。代价是一次误报事件，
                    // 不影响功能正确性。如未来误报频率上升，可在 FrpcState 加
                    // `stop_in_progress: AtomicBool` 显式标记。
                    emit_log(
                        &app_for_thread,
                        "system",
                        format!(
                            "核心引擎已退出，code={:?}, signal={:?}",
                            payload.code, payload.signal
                        ),
                    );
                    if let Some(state) = app_for_thread.try_state::<FrpcState>() {
                        if let Ok(mut guard) = state.child.lock() {
                            *guard = None;
                        }
                        reset_to_stopped(state.inner());
                        emit_status(&app_for_thread, state.inner());
                    }
                    break;
                }
                CommandEvent::Error(err) => {
                    emit_log(&app_for_thread, "stderr", format!("核心引擎进程错误：{err}"));
                }
                _ => {}
            }
        }
    });

    // 5. 启动连接状态轮询任务：3s 间隔探测 frpc webServer `/api/status`
    let app_for_poll = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::frpc_state::poll_conn_state(app_for_poll).await;
    });

    Ok(())
}

#[tauri::command]
pub fn stop_frpc(app: AppHandle, state: State<'_, FrpcState>) -> Result<(), String> {
    let child = {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        guard.take()
    };
    if let Some(child) = child {
        child
            .kill()
            .map_err(|e| format!("停止核心引擎失败：{e}"))?;
        reset_to_stopped(state.inner());
        emit_log(&app, "system", "已停止核心引擎".into());
        emit_status(&app, state.inner());
    }
    // child 为 None 时幂等返回 Ok：既兼容前端"未运行也点停止"的路径，
    // 也让退出钩子可以无条件调用而不污染日志。
    Ok(())
}

#[tauri::command]
pub fn frpc_running(state: State<'_, FrpcState>) -> Result<bool, String> {
    let guard = state.child.lock().map_err(|e| e.to_string())?;
    Ok(guard.is_some())
}

#[tauri::command]
pub fn frpc_status(state: State<'_, FrpcState>) -> Result<StatusPayload, String> {
    let status = state
        .conn
        .lock()
        .map(|g| g.as_str().to_string())
        .map_err(|e| e.to_string())?;
    let error = state
        .error_msg
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    Ok(StatusPayload { status, error })
}

/// 杀掉孤儿 frpc 后给 OS 回收 fd / 端口的等待时间（经验值，覆盖 TIME_WAIT）。
const REAP_SETTLE_MS: u64 = 200;

/// 启动 frpc 前清理可能残留的孤儿 frpc 进程，避免 127.0.0.1:7400 端口被占。
///
/// 适用场景：主进程异常退出（`tauri dev` 热重载触发 cargo SIGKILL 旧二进制
/// / 应用崩溃 / `kill -9`）时，`ExitRequested` 兜底来不及跑，frpc sidecar
/// 被 reparent 到 init/launchd 成为孤儿。下次启动时新 frpc 与孤儿抢同一个
/// 7400 端口，必然 bind 失败退出。
///
/// 匹配规则：进程名为 `frpc`（Windows 上为 `frpc.exe`）**且** 命令行参数中
/// 存在等于本应用 `app_config_dir/frpc.toml` 路径的项——精准锁定本应用派生
/// 的 frpc，不会误伤用户机器上其他用途的 frpc 进程。
pub fn reap_orphan_frpc(cfg_path: &Path) -> u32 {
    let sys = System::new_all();
    let cfg_str = cfg_path.to_string_lossy();
    let mut killed = 0u32;

    for (pid, process) in sys.processes() {
        // 跨平台兼容：Unix 上 `name()` 通常为 `frpc`，Windows 上可能为
        // `frpc.exe`（或某些 sysinfo 版本会带绝对路径）。这里大小写不敏感
        // 比较末尾基本名，覆盖最常见的两种情况；argv 路径过滤保证不会误杀。
        let name = process.name();
        if !is_frpc_image(name) {
            continue;
        }
        let cmd = process.cmd();
        if cmd.iter().any(|arg| arg.to_string_lossy() == cfg_str.as_ref()) {
            if process.kill() {
                eprintln!("[reap] 杀掉孤儿 frpc pid={:?} image={:?}", pid, name);
                killed += 1;
            }
        }
    }

    if killed > 0 {
        // 给 OS 一点时间回收 fd 与端口，避免下一行 spawn 又抢到 TIME_WAIT
        std::thread::sleep(std::time::Duration::from_millis(REAP_SETTLE_MS));
    }

    killed
}

/// 判断进程映像名是否为 frpc（覆盖 Unix `frpc` 与 Windows `frpc.exe`）。
///
/// sysinfo 0.32 的 `Process::name()` 返回 `&OsStr`：
/// - Unix：进程基本名（如 `frpc`）
/// - Windows：sysinfo 内部已 strip 绝对路径，但**保留 `.exe` 后缀**
///
/// 此处用大小写不敏感比较末尾路径段，兼容以上两种与未来 sysinfo 版本可能
/// 出现的细微差异。
fn is_frpc_image(name: &std::ffi::OsStr) -> bool {
    let base = name.to_string_lossy();
    let base = base.rsplit(['/', '\\']).next().unwrap_or(&base);
    base.eq_ignore_ascii_case("frpc") || base.eq_ignore_ascii_case("frpc.exe")
}
