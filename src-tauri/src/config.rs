//! 客户端配置生成（TOML）与持久化（`tauri-plugin-store`）。

use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::types::{is_supported_proxy_type, StartArgs};

const CONFIG_STORE_FILE: &str = "config.store.json";
const KEY_START_ARGS: &str = "start_args";

const FRPC_CONFIG_FILENAME: &str = "frpc.toml";

/// 返回 `app_config_dir/frpc.toml` 路径，必要时创建父目录。
pub fn frpc_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let cfg_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法获取配置目录：{e}"))?;
    std::fs::create_dir_all(&cfg_dir).map_err(|e| e.to_string())?;
    Ok(cfg_dir.join(FRPC_CONFIG_FILENAME))
}

/// 把 `StartArgs` 序列化为 frpc.toml 文本，包含字段合法性校验。
///
/// **顺序约束**：`webServer.*` 必须在 `[[proxies]]` 之前（TOML 数组表语法），
/// 否则 frpc 报 `unknown field "webServer"`。
pub fn build_toml(args: &StartArgs) -> Result<String, String> {
    if args.server_addr.is_empty() || args.server_port == 0 {
        return Err("请填写正确的服务商地址与端口".into());
    }
    if args.proxies.is_empty() {
        return Err("请至少添加一条代理规则".into());
    }

    let mut proxies_toml = String::new();
    for p in &args.proxies {
        if p.name.is_empty() || p.proxy_type.is_empty() {
            return Err("代理配置中存在空字段".into());
        }
        if !is_supported_proxy_type(&p.proxy_type) {
            return Err(format!(
                "不支持的代理类型：{}（仅支持 tcp/udp/http/https）",
                p.proxy_type
            ));
        }
        proxies_toml.push_str(&format!(
            "[[proxies]]\nname = \"{}\"\ntype = \"{}\"\nlocalIP = \"{}\"\nlocalPort = {}\nremotePort = {}\n\n",
            escape_toml(&p.name),
            escape_toml(&p.proxy_type),
            escape_toml(&p.local_ip),
            p.local_port,
            p.remote_port,
        ));
    }

    Ok(format!(
        "serverAddr = \"{}\"\nserverPort = {}\nloginFailExit = false\ntransport.dialServerTimeout = 30\ntransport.heartbeatInterval = 30\ntransport.heartbeatTimeout = 90\nwebServer.addr = \"127.0.0.1\"\nwebServer.port = 7400\nwebServer.user = \"admin\"\nwebServer.password = \"admin\"\n{}\n{}\n{}\n",
        escape_toml(&args.server_addr),
        args.server_port,
        args.token
            .as_deref()
            .map(|t| format!("auth.token = \"{}\"\n", escape_toml(t)))
            .unwrap_or_default(),
        args.user
            .as_deref()
            .map(|u| format!("user = \"{}\"\n", escape_toml(u)))
            .unwrap_or_default(),
        proxies_toml,
    ))
}

/// TOML 字符串值转义：处理 `\` 与 `"`，并把控制字符（`\n` `\r` `\t`）
/// 替换为空格——避免用户在名称字段中粘入换行 / Tab 破坏 frpc.toml 解析。
fn escape_toml(s: &str) -> String {
    s.replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

/// 保存客户端配置到 `tauri-plugin-store`（`config.store.json`）。
#[tauri::command]
pub fn save_config(app: AppHandle, args: StartArgs) -> Result<(), String> {
    let store = app
        .store(CONFIG_STORE_FILE)
        .map_err(|e| format!("无法访问配置存储：{e}"))?;
    let value = serde_json::to_value(&args).map_err(|e| format!("序列化配置失败：{e}"))?;
    store.set(KEY_START_ARGS, value);
    store.save().map_err(|e| format!("保存配置失败：{e}"))
}

/// 读取已保存的客户端配置。
#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<Option<StartArgs>, String> {
    let store = app
        .store(CONFIG_STORE_FILE)
        .map_err(|e| format!("无法访问配置存储：{e}"))?;
    let Some(value) = store.get(KEY_START_ARGS) else {
        return Ok(None);
    };
    let args: StartArgs =
        serde_json::from_value(value).map_err(|e| format!("解析配置失败：{e}"))?;
    Ok(Some(args))
}
