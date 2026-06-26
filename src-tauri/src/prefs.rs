//! 应用级偏好：开机启动（OS 启动项）+ 静默启动（自启时隐藏到托盘）
//! + 开机自动连接（自启时自动拉起 frpc）+ 定时连接（按星期多选 + 起止时间）。
//!
//! 与 `config.rs` 的 `StartArgs`（服务商 / Token / 代理规则）解耦：
//! - `Prefs` 走 `tauri-plugin-store` 的 `prefs.json`
//! - `StartArgs` 走同一个插件的 `config.store.json`
//!
//! `set_auto_launch` 始终以 OS 实际状态回填 `Prefs.auto_launch`，避免插件
//! 在某些平台 silently 失败导致 UI 与系统状态不一致。

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "prefs.json";
const KEY_AUTO_LAUNCH: &str = "auto_launch";
const KEY_SILENT_START: &str = "silent_start";
const KEY_AUTO_CONNECT: &str = "auto_connect";
const KEY_SCHEDULE: &str = "schedule";
const KEY_LANGUAGE: &str = "language";
const DEFAULT_LANGUAGE: &str = "zh-CN";

/// 定时连接配置：按星期多选 + 启动/断开时间（"HH:MM"）。
///
/// `weekdays` 下标语义：0=周一 … 6=周日；与 `chrono::Weekday::num_days_from_monday`
/// 的取值一一对应，方便调度器直接索引。
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct Schedule {
    /// 主开关；false 时调度器与补跑逻辑均跳过
    #[serde(default)]
    pub enabled: bool,
    /// 一周 7 天的启用状态，下标 0=周一 … 6=周日
    #[serde(default)]
    pub weekdays: [bool; 7],
    /// 启动时间，24 小时制 "HH:MM"
    #[serde(default = "default_start_time")]
    pub start_time: String,
    /// 断开时间，24 小时制 "HH:MM"
    #[serde(default = "default_stop_time")]
    pub stop_time: String,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: false,
            weekdays: [false; 7],
            start_time: default_start_time(),
            stop_time: default_stop_time(),
        }
    }
}

fn default_start_time() -> String {
    "08:00".into()
}

fn default_stop_time() -> String {
    "18:00".into()
}

/// 应用级偏好（与服务端 frpc 配置 `StartArgs` 解耦）。
///
/// 注意：去掉 `Copy` 因为 `Schedule` 含 `String`；调用方用引用或 `.clone()`。
#[derive(Deserialize, Serialize, Clone, Default, Debug, PartialEq)]
pub struct Prefs {
    /// 开机启动
    #[serde(default)]
    pub auto_launch: bool,
    /// 静默启动：开机自启时隐藏到系统托盘
    #[serde(default)]
    pub silent_start: bool,
    /// 开机自动连接：仅当 OS 自启（`--auto-launched`）时自动拉起 frpc；
    /// 与 `schedule` 正交——本字段只决定「开机那一瞬间是否启动」，
    /// 之后 frpc 的死活由 `schedule` / 用户手动操作接管
    #[serde(default)]
    pub auto_connect: bool,
    /// 定时连接配置
    #[serde(default)]
    pub schedule: Schedule,
    /// 界面语言 code；缺省/非法值由前端 `i18n.ts::normalizeLocale` 兜底
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    DEFAULT_LANGUAGE.into()
}

pub fn load(app: &AppHandle) -> Prefs {
    let Ok(store) = app.store(STORE_FILE) else {
        return Prefs::default();
    };
    let auto_launch = store
        .get(KEY_AUTO_LAUNCH)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let silent_start = store
        .get(KEY_SILENT_START)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let auto_connect = store
        .get(KEY_AUTO_CONNECT)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let schedule = store
        .get(KEY_SCHEDULE)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let language = store
        .get(KEY_LANGUAGE)
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(default_language);
    Prefs {
        auto_launch,
        silent_start,
        auto_connect,
        schedule,
        language,
    }
}

pub fn save(app: &AppHandle, prefs: &Prefs) -> Result<(), String> {
    let store = app
        .store(STORE_FILE)
        .map_err(|e| format!("无法访问偏好存储：{e}"))?;
    store.set(KEY_AUTO_LAUNCH, prefs.auto_launch);
    store.set(KEY_SILENT_START, prefs.silent_start);
    store.set(KEY_AUTO_CONNECT, prefs.auto_connect);
    let schedule_val = serde_json::to_value(&prefs.schedule)
        .map_err(|e| format!("序列化调度配置失败：{e}"))?;
    store.set(KEY_SCHEDULE, schedule_val);
    store.set(KEY_LANGUAGE, prefs.language.clone());
    store.save().map_err(|e| format!("保存偏好失败：{e}"))
}

/// 校验调度配置。`enabled=false` 时一律放行——让用户随时能"先关掉再说"，
/// 不被卡在历史脏数据上；启用态下检查星期 / 时间合法性。
///
/// 当前版本不支持跨夜（`start_time >= stop_time` 拒绝），覆盖绝大多数工作时段
/// 场景；如需跨夜可在后续版本中扩展为窗口判断。
pub fn validate_schedule(s: &Schedule) -> Result<(), String> {
    if !s.enabled {
        return Ok(());
    }
    if !s.weekdays.iter().any(|x| *x) {
        return Err("请至少选择一天".into());
    }
    if !is_hhmm(&s.start_time) || !is_hhmm(&s.stop_time) {
        return Err("时间格式需为 HH:MM".into());
    }
    if s.start_time == s.stop_time {
        return Err("启动与断开时间不能相同".into());
    }
    if s.start_time > s.stop_time {
        return Err("启动时间需早于断开时间（暂不支持跨夜）".into());
    }
    Ok(())
}

/// 简洁的 "HH:MM" 校验：避免引入正则 crate。
fn is_hhmm(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() == 5
        && bytes[2] == b':'
        && bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && {
            let h = (bytes[0] - b'0') as u32 * 10 + (bytes[1] - b'0') as u32;
            let m = (bytes[3] - b'0') as u32 * 10 + (bytes[4] - b'0') as u32;
            h < 24 && m < 60
        }
}

#[tauri::command]
pub fn get_prefs(app: AppHandle) -> Result<Prefs, String> {
    Ok(load(&app))
}

#[tauri::command]
pub fn save_prefs(app: AppHandle, prefs: Prefs) -> Result<(), String> {
    validate_schedule(&prefs.schedule)?;
    save(&app, &prefs)
}

#[tauri::command]
pub fn set_auto_launch(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager
            .enable()
            .map_err(|e| format!("开启开机启动失败：{e}"))?;
    } else {
        manager
            .disable()
            .map_err(|e| format!("关闭开机启动失败：{e}"))?;
    }
    let actual = manager.is_enabled().unwrap_or(false);
    let mut prefs = load(&app);
    prefs.auto_launch = actual;
    save(&app, &prefs)?;
    Ok(actual)
}

#[tauri::command]
pub fn get_auto_launch(app: AppHandle) -> Result<bool, String> {
    let manager = app.autolaunch();
    Ok(manager.is_enabled().unwrap_or(false))
}

/// setup 钩子调用：当本次启动由 OS 自启触发（`--auto-launched` 参数）且
/// 用户已开启静默启动时，隐藏主窗口到系统托盘。
///
/// 用户主动双击启动时无该参数，故不会触发隐藏。
/// `prefs` / `auto_launched` 由 setup hook 顶层算一次后传入（详见
/// `src-tauri/AGENTS.md §5.2.1`），**不要**在此函数内重新 load / 扫 argv。
pub fn maybe_silent_start(app: &AppHandle, prefs: &Prefs, auto_launched: bool) {
    if !auto_launched {
        return;
    }
    if !prefs.silent_start {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// setup 钩子调用：当本次启动由 OS 自启触发（`--auto-launched` 参数）且
/// 用户已开启开机自动连接时，自动拉起 frpc 子进程。
///
/// 仅在 OS 自启场景触发；用户主动双击启动不带该参数，不会自连。
/// 与 `scheduler::maybe_catch_up_start` 正交：两者都先查 `child.is_some()`，
/// 已跑则跳过——避免后到的 `start_frpc` 因互斥锁返 `"核心引擎已在运行中"`
/// 而产生误导性「开机自动连接失败」日志。未配置 `StartArgs`（首次启动 / 只
/// 配了开机自动连接还没填服务商）时 `emit_log` 提示后跳过——用户在「日志」Tab 可见。
///
/// 不阻塞 setup：内部 spawn 异步任务执行 `start_frpc`。
/// `prefs` / `auto_launched` 由 setup hook 顶层算一次后传入（详见
/// `src-tauri/AGENTS.md §5.2.1`），**不要**在此函数内重新 load / 扫 argv。
pub fn maybe_auto_connect(app: &AppHandle, prefs: &Prefs, auto_launched: bool) {
    if !auto_launched {
        return;
    }
    if !prefs.auto_connect {
        return;
    }
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        fire_auto_connect(&app_clone).await;
    });
}

/// `maybe_auto_connect` 的 spawn 体：与 `scheduler::try_fire_start` 同构——
/// 先查 frpc 状态幂等跳过，再加载 `StartArgs` + `start_frpc`，失败 / 缺配置
/// 时 `emit_log` 提示。`&AppHandle` 借用即可，避免在闭包内多次 clone。
async fn fire_auto_connect(app: &AppHandle) {
    let state = match app.try_state::<crate::frpc_state::FrpcState>() {
        Some(s) => s,
        None => return,
    };
    let already_running = state
        .child
        .lock()
        .map(|g| g.is_some())
        .unwrap_or_else(|e| {
            eprintln!("[prefs] FrpcState.child mutex 中毒：{e}");
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
                "开机自动连接已开启，但尚未配置服务端，跳过".into(),
            );
            return;
        }
    };
    if let Err(e) = crate::process::start_frpc(app.clone(), state, args).await {
        crate::frpc_state::emit_log(app, "system", format!("开机自动连接失败：{e}"));
    }
}
