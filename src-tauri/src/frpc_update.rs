//! frpc 核心引擎自更新：GitHub Release → SHA256 校验 → 提取 tar.gz 中的
//! frpc → 写 pending → 重启时原子替换到 sidecar 路径并 `frpc -v` 验证。
//!
//! 关键状态文件：`app_config_dir()/frpc_update.json`（`current` / `pending`）。
//! 跨平台二进制命名、原子写策略与 macOS ad-hoc 签名等见 `src-tauri/AGENTS.md §7`。

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

/// 打包内置的 frpc 版本，必须与 src-tauri/binaries/ 中的二进制保持同步
const BUNDLED_FRPC_VERSION: &str = "0.69.1";
const GITHUB_API_LATEST: &str = "https://api.github.com/repos/fatedier/frp/releases/latest";
const FRP_DOWNLOAD_HOST: &str = "https://github.com/fatedier/frp/releases/download";

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PendingUpdate {
    pub version: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct UpdateState {
    pub current: String,
    pub pending: Option<PendingUpdate>,
}

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: String,
    pub asset_name: String,
    pub asset_size: u64,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    #[serde(default)]
    size: u64,
}

fn state_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("frpc_update.json"))
}

fn load_state(app: &AppHandle) -> Result<UpdateState, String> {
    let path = state_path(app)?;
    if !path.exists() {
        return Ok(UpdateState {
            current: BUNDLED_FRPC_VERSION.to_string(),
            pending: None,
        });
    }
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut state: UpdateState = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    if state.current.is_empty() {
        state.current = BUNDLED_FRPC_VERSION.to_string();
    }
    Ok(state)
}

fn save_state(app: &AppHandle, state: &UpdateState) -> Result<(), String> {
    let path = state_path(app)?;
    let s = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(&path, s).map_err(|e| e.to_string())
}

/// 返回 (os_part, arch_part)，匹配 frp 官方 release 命名（如 darwin_arm64）
fn platform_asset_suffix() -> Result<(&'static str, &'static str), String> {
    use std::env::consts;
    let os = match consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        "linux" => "linux",
        other => return Err(format!("不支持的操作系统: {other}")),
    };
    let arch = match consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        other => return Err(format!("不支持的架构: {other}")),
    };
    Ok((os, arch))
}

/// 返回 Tauri sidecar 实际打包后的目标三元组文件名（如 frpc-aarch64-apple-darwin）
fn sidecar_target_name() -> Result<String, String> {
    use std::env::consts;
    let target = match (consts::OS, consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        _ => return Err(format!("不支持的平台 {}/{}", consts::OS, consts::ARCH)),
    };
    let exe = if consts::OS == "windows" { ".exe" } else { "" };
    Ok(format!("frpc-{}{}", target, exe))
}

/// 解析 sidecar 在 resource_dir 中的实际路径
fn sidecar_path(app: &AppHandle) -> Result<PathBuf, String> {
    let resource = app.path().resource_dir().map_err(|e| e.to_string())?;
    let name = sidecar_target_name()?;
    Ok(resource.join("binaries").join(name))
}

/// 返回 (asset_name, asset_url, checksums_url)
fn asset_urls(version: &str) -> Result<(String, String, String), String> {
    let (os, arch) = platform_asset_suffix()?;
    let v = version.trim_start_matches('v');
    let asset_name = format!("frp_{}_{}_{}.tar.gz", v, os, arch);
    let asset_url = format!("{}/v{}/{}", FRP_DOWNLOAD_HOST, v, asset_name);
    let checksums_url = format!("{}/v{}/frp_{}_checksums.txt", FRP_DOWNLOAD_HOST, v, v);
    Ok((asset_name, asset_url, checksums_url))
}

fn build_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(format!(
            "MoonProxy/{} (frpc-updater)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| e.to_string())
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn parse_checksums(content: &str, asset_name: &str) -> Option<String> {
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(hash), Some(name)) = (parts.next(), parts.next()) {
            if name == asset_name {
                return Some(hash.to_lowercase());
            }
        }
    }
    None
}

/// 从 tar.gz 字节流中提取 frpc 二进制
fn extract_frpc_from_tar(tar_gz_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let gz = flate2::read::GzDecoder::new(tar_gz_bytes);
    let mut archive = tar::Archive::new(gz);
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?;
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == "frpc" || name == "frpc.exe" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            return Ok(buf);
        }
    }
    Err("压缩包中未找到 frpc".into())
}

/// 写入临时文件再原子 rename，避免半写状态
fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("frpc_tmp");
    std::fs::write(&tmp, data).map_err(|e| format!("写入临时文件失败: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("替换文件失败: {e}"))?;
    Ok(())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("设置可执行权限失败: {e}"))
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn ad_hoc_codesign(path: &Path) {
    let _ = std::process::Command::new("codesign")
        .args(["-s", "-", "-f", &path.to_string_lossy()])
        .output();
}

#[cfg(not(target_os = "macos"))]
fn ad_hoc_codesign(_path: &Path) {}

#[tauri::command]
pub async fn get_frpc_version(app: AppHandle) -> Result<String, String> {
    let state = load_state(&app)?;
    Ok(state.current)
}

#[tauri::command]
pub async fn check_frpc_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let state = load_state(&app)?;
    let release = {
        let client = build_http_client()?;
        let resp = client
            .get(GITHUB_API_LATEST)
            .send()
            .await
            .map_err(|e| format!("请求 GitHub 失败: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("GitHub 返回状态: {}", resp.status()));
        }
        resp.json::<GithubRelease>()
            .await
            .map_err(|e| format!("解析 GitHub 响应失败: {e}"))?
    };

    let latest_raw = release.tag_name.trim_start_matches('v').to_string();
    let current_raw = state.current.trim_start_matches('v').to_string();
    let latest_sv = semver::Version::parse(&latest_raw).map_err(|e| e.to_string())?;
    let current_sv = semver::Version::parse(&current_raw)
        .map_err(|e| e.to_string())?;
    if latest_sv <= current_sv {
        return Ok(None);
    }

    let (asset_name, _, _) = asset_urls(&latest_raw)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| format!("GitHub 未找到资产 {asset_name}"))?;

    Ok(Some(UpdateInfo {
        current_version: current_raw,
        latest_version: latest_raw,
        release_notes: release.body,
        asset_name: asset.name.clone(),
        asset_size: asset.size,
    }))
}

#[tauri::command]
pub async fn download_frpc_update(app: AppHandle, version: String) -> Result<(), String> {
    let v = version.trim_start_matches('v').to_string();
    let (asset_name, asset_url, checksums_url) = asset_urls(&v)?;
    let client = build_http_client()?;

    let checksums = client
        .get(&checksums_url)
        .send()
        .await
        .map_err(|e| format!("下载 checksums 失败: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let expected = parse_checksums(&checksums, &asset_name)
        .ok_or_else(|| "checksums.txt 中未找到对应资产".to_string())?;

    let bytes = client
        .get(&asset_url)
        .send()
        .await
        .map_err(|e| format!("下载资产失败: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(format!("SHA256 校验失败: 期望 {expected}, 实际 {actual}"));
    }

    let frpc_bytes = extract_frpc_from_tar(&bytes)?;

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let pending_name = if std::env::consts::OS == "windows" {
        format!("frpc_pending_{}.exe", v)
    } else {
        format!("frpc_pending_{}", v)
    };
    let pending_path = dir.join(&pending_name);
    atomic_write(&pending_path, &frpc_bytes)?;
    set_executable(&pending_path)?;

    let mut state = load_state(&app)?;
    state.pending = Some(PendingUpdate {
        version: v.clone(),
        path: pending_path.to_string_lossy().to_string(),
    });
    save_state(&app, &state)?;

    let _ = app.emit("frpc://update-downloaded", serde_json::json!({ "version": v }));
    Ok(())
}

/// 把 pending 二进制原子替换到 sidecar 路径，并调用 frpc -v 验证
pub fn apply_pending_update(app: &AppHandle) -> Result<Option<String>, String> {
    let mut state = load_state(app)?;
    let pending = match state.pending.clone() {
        Some(p) => p,
        None => return Ok(None),
    };

    let pending_path = Path::new(&pending.path);
    if !pending_path.exists() {
        state.pending = None;
        save_state(app, &state)?;
        return Err("pending 二进制丢失，请重新下载".into());
    }

    let dest = sidecar_path(app)?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let bytes = std::fs::read(pending_path).map_err(|e| format!("读取 pending 失败: {e}"))?;
    atomic_write(&dest, &bytes)?;
    set_executable(&dest)?;
    ad_hoc_codesign(&dest);

    let output = std::process::Command::new(&dest)
        .arg("-v")
        .output()
        .map_err(|e| format!("frpc -v 执行失败: {e}"))?;
    if !output.status.success() {
        return Err("frpc -v 验证失败，二进制可能损坏".into());
    }

    let _ = std::fs::remove_file(pending_path);

    state.current = pending.version.clone();
    state.pending = None;
    save_state(app, &state)?;

    Ok(Some(pending.version))
}

#[tauri::command]
pub fn apply_pending_frpc_update(app: AppHandle) -> Result<Option<String>, String> {
    apply_pending_update(&app)
}
