#!/usr/bin/env bash
# 同步 frpc 二进制到 src-tauri/binaries/，命名严格匹配 Tauri externalBin 约定。
# 版本从仓库根目录 .env 的 FRPC_VERSION 读取，必须与
# src-tauri/src/frpc_update.rs 的 BUNDLED_FRPC_VERSION 保持一致。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="$REPO_ROOT/src-tauri/binaries"
DOWNLOAD_HOST="https://github.com/fatedier/frp/releases/download"

# 依赖检查
for cmd in curl tar; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "缺少依赖：$cmd" >&2; exit 1; }
done
# unzip 仅在下载 Windows 包时需要；提前校验以尽早失败
command -v unzip >/dev/null 2>&1 || { echo "缺少依赖：unzip（解压 Windows 包需要）" >&2; exit 1; }

# ---------- 参数解析 ----------
DO_ALL=0
FORCE=0
for arg in "$@"; do
  case "$arg" in
    --all)   DO_ALL=1 ;;
    --force) FORCE=1 ;;
    --)      ;;
    --help|-h)
      cat <<EOF
用法: sync-frpc.sh [--all] [--force] [--help]
  无参数   下载当前平台的 frpc
  --all    下载全部 4 个平台（macOS ARM/Intel + Windows x64/ARM64）
  --force  覆盖已存在的二进制
  --help   显示本帮助
EOF
      exit 0 ;;
    *) echo "未知参数: $arg (用 --help 查看用法)" >&2; exit 2 ;;
  esac
done

# ---------- 读取版本 ----------
if [[ ! -f "$REPO_ROOT/.env" ]]; then
  if [[ -f "$REPO_ROOT/.env.example" ]]; then
    cp "$REPO_ROOT/.env.example" "$REPO_ROOT/.env"
    echo "已从 .env.example 创建 .env"
  else
    echo "错误：找不到 .env 或 .env.example" >&2
    exit 1
  fi
fi
# shellcheck disable=SC1090
source "$REPO_ROOT/.env"
if [[ -z "${FRPC_VERSION:-}" ]]; then
  echo "错误：.env 中未定义 FRPC_VERSION" >&2
  exit 1
fi
VERSION="$FRPC_VERSION"
echo "frpc 版本：$VERSION"

mkdir -p "$BIN_DIR"

# ---------- 平台映射 ----------
# 每行：tauri_target_name|github_asset_suffix|is_windows
TARGETS=(
  "frpc-aarch64-apple-darwin|darwin_arm64.tar.gz|0"
  "frpc-x86_64-apple-darwin|darwin_amd64.tar.gz|0"
  "frpc-x86_64-pc-windows-msvc.exe|windows_amd64.zip|1"
  "frpc-aarch64-pc-windows-msvc.exe|windows_arm64.zip|1"
)

detect_current_target() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os/$arch" in
    Darwin/arm64)    echo "frpc-aarch64-apple-darwin|darwin_arm64.tar.gz|0" ;;
    Darwin/x86_64)   echo "frpc-x86_64-apple-darwin|darwin_amd64.tar.gz|0" ;;
    MINGW*/x86_64|MSYS*/x86_64|CYGWIN*/x86_64)
                     echo "frpc-x86_64-pc-windows-msvc.exe|windows_amd64.zip|1" ;;
    MINGW*/aarch64|MSYS*/aarch64|CYGWIN*/aarch64)
                     echo "frpc-aarch64-pc-windows-msvc.exe|windows_arm64.zip|1" ;;
    *) echo "当前平台 $os/$arch 暂不被自动探测支持，请用 --all 下载全部" >&2; exit 1 ;;
  esac
}

# 下载并安装单个目标
install_target() {
  local entry="$1"
  local target_name asset_suffix is_win
  # 旧版 bash（macOS 3.2）在 set -e + local + <<< heredoc 组合下不可靠，
  # 用命令替换 + IFS 切割更稳
  target_name="${entry%%|*}"
  local rest="${entry#*|}"
  asset_suffix="${rest%%|*}"
  is_win="${entry##*|}"

  local dest="$BIN_DIR/$target_name"
  if [[ -f "$dest" && $FORCE -eq 0 ]]; then
    echo "  [跳过] $target_name 已存在 (用 --force 覆盖)"
    return 0
  fi

  local asset="frp_${VERSION}_${asset_suffix}"
  local url="${DOWNLOAD_HOST}/v${VERSION}/${asset}"

  local tmp_dir
  tmp_dir="$(mktemp -d)"

  echo "  [下载] $url"
  if ! curl -fL --progress-bar -o "$tmp_dir/$asset" "$url"; then
    rm -rf "$tmp_dir"
    echo "下载失败。若网络受限，可设置 http_proxy / https_proxy 后重试，例如：" >&2
    echo "  http_proxy=http://127.0.0.1:7897 https_proxy=http://127.0.0.1:7897 pnpm sync:frpc" >&2
    return 1
  fi

  # 解压并取出 frpc
  local rc=0
  (
    cd "$tmp_dir"
    if [[ "$is_win" == "1" ]]; then
      unzip -o "$asset" >/dev/null
      local extracted
      extracted="$(find . -maxdepth 1 -type d -name 'frp_*' | head -1)"
      [[ -z "$extracted" ]] && { echo "解压后未找到 frp 目录" >&2; exit 1; }
      cp "$extracted/frpc.exe" "$dest"
    else
      tar -xzf "$asset"
      local extracted
      extracted="$(find . -maxdepth 1 -type d -name 'frp_*' | head -1)"
      [[ -z "$extracted" ]] && { echo "解压后未找到 frp 目录" >&2; exit 1; }
      cp "$extracted/frpc" "$dest"
      chmod +x "$dest"
    fi
  ) || rc=$?
  rm -rf "$tmp_dir"
  [[ $rc -ne 0 ]] && return $rc

  # 粗校验
  local size
  size="$(stat -f%z "$dest" 2>/dev/null || stat -c%s "$dest")"
  if [[ $size -lt 1048576 ]]; then
    echo "警告：$target_name 体积仅 $size 字节，可能损坏" >&2
    return 1
  fi
  local size_mb=$(( size / 1024 / 1024 ))
  echo "  [完成] $target_name  ($size_mb MB)"
}

# ---------- 执行 ----------
if [[ $DO_ALL -eq 1 ]]; then
  echo "下载全部 4 个平台..."
  for t in "${TARGETS[@]}"; do
    install_target "$t" || exit 1
  done
else
  echo "探测当前平台..."
  current="$(detect_current_target)"
  install_target "$current" || exit 1
fi

echo "完成。文件位于 src-tauri/binaries/"
