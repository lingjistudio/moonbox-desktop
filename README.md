# Moonbox Desktop

> 跨平台、面向非技术用户的 FRP 桌面客户端。基于 [Tauri v2](https://tauri.app) 构建，支持 macOS 与 Windows。

[English](#english) · [简体中文](#简体中文)

---

## English

A friendly desktop GUI for the [frp](https://github.com/fatedier/frp) reverse proxy.
You bring your own frps server (or any community frps you trust) — Moonbox Desktop
takes care of the rest: configuration, lifecycle, connection health, auto-update,
and a polished tray-resident experience.

### Highlights

- **Visual proxy management** — TCP / UDP / HTTP / HTTPS rules in a single tab.
- **One-click start/stop** — a circular button reflects 4 states
  (stopped / connecting / connected / error); the connected state is derived
  from real frpc evidence, not optimistic flags.
- **Live endpoint health** — local port reachability is polled every 3s so you
  catch broken tunnels before they bite you.
- **System tray resident** — closing the window hides to tray while frpc keeps
  running in the background.
- **Launch at login + silent start** — auto-launch can hide straight to tray.
- **Scheduled connect** — pick weekdays and start/stop times; hot-reloaded
  each minute by the scheduler.
- **Engine self-update** — frpc is fetched from upstream GitHub releases,
  SHA256-verified, then atomically swapped without reinstalling the app.
- **App self-update** — built on `tauri-plugin-updater`, signed and reproducible.
- **Bundled frpc sidecar** — users never need to install frp separately.

### Install

Pre-built binaries are published on the
[GitHub Releases page](https://github.com/lingjistudio/moonbox-desktop/releases).

| Platform | Download |
| --- | --- |
| macOS (Apple Silicon) | `Moonbox-Desktop_<version>_aarch64.dmg` |
| macOS (Intel) | `Moonbox-Desktop_<version>_x64.dmg` |
| Windows (x64) | `Moonbox-Desktop_<version>_x64-setup.exe` |

> **macOS first-launch note:** the app is ad-hoc signed but **not** notarized
> (no Apple Developer certificate). On first launch, right-click the app →
> **Open** → confirm in the dialog. Alternatively, after dragging to
> `/Applications`, run `xattr -cr "/Applications/Moonbox Desktop.app"` to drop
> the quarantine attribute.

### Build from source

```bash
pnpm install
pnpm sync:frpc        # download frpc sidecar binaries
pnpm tauri dev        # local dev
pnpm tauri build      # production build for current platform
```

> Requires Node.js, pnpm, Rust toolchain, and platform-specific build tools.
> See [CONTRIBUTING.md](./CONTRIBUTING.md) for details.

### License

[MIT](./LICENSE).

---

## 简体中文

面向非技术用户的 [frp](https://github.com/fatedier/frp) 桌面客户端。
你只需要提供一台运行了 frps 的服务器（自建或社区公开均可），剩下的交给 Moonbox Desktop：
配置生成、子进程生命周期、连接健康检查、自动更新、托盘常驻等开箱即用。

### 核心特性

- **可视化管理代理规则**：TCP / UDP / HTTP / HTTPS 一面板搞定，主页实时显示本地端口连通性
- **一键启停 frpc**：圆形大按钮分 4 态（已停止 / 连接中 / 已连接 / 连接错误），「已连接」由 frpc 自身证据支撑而非乐观标记
- **端点健康轮询**：每 3 秒探测一次本地端口可达性，提前发现隧道断裂
- **系统托盘常驻**：关闭窗口默认隐藏到托盘，frpc 继续后台运行
- **开机启动 + 静默启动**：自启时直接隐藏到托盘
- **定时连接**：按星期多选 + 起止时间，调度器每分钟热加载
- **引擎自更新**：从 frp 上游 GitHub Release 拉取，SHA256 校验后原子替换，无需重装应用
- **应用本体自更新**：基于 `tauri-plugin-updater` 的「重启并安装」
- **内置 frpc 二进制**：通过 Tauri sidecar 机制打包，用户无需单独安装

### 下载

预构建包发布在 [GitHub Releases](https://github.com/lingjistudio/moonbox-desktop/releases)。

| 平台 | 文件 |
| --- | --- |
| macOS (Apple Silicon) | `Moonbox-Desktop_<version>_aarch64.dmg` |
| macOS (Intel) | `Moonbox-Desktop_<version>_x64.dmg` |
| Windows (x64) | `Moonbox-Desktop_<version>_x64-setup.exe` |

> **macOS 首次打开提示**：本应用为 ad-hoc 签名，**未做 Apple 公证**（无 Apple Developer 证书）。
> 首次打开请右键点击应用 → **打开** → 在弹出对话框中点 **打开**；
> 或将应用拖入 `/Applications` 后执行 `xattr -cr "/Applications/Moonbox Desktop.app"`
> 去掉隔离属性。Intel Mac 可直接双击运行。

### 从源码构建

```bash
pnpm install
pnpm sync:frpc        # 下载 frpc 二进制
pnpm tauri dev        # 本地开发联调
pnpm tauri build      # 当前平台打包
```

> 依赖：Node.js、pnpm、Rust 工具链、各平台构建工具链。详见 [CONTRIBUTING.md](./CONTRIBUTING.md)。

### 许可证

[MIT](./LICENSE)。

> 本项目与 [fatedier/frp](https://github.com/fatedier/frp) 项目相互独立，
> frp 的发布与许可归原项目所有；Moonbox Desktop 仅作为其桌面客户端。
