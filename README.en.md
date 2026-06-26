# Moonbox Desktop

> A friendly, cross-platform desktop GUI for the [frp](https://github.com/fatedier/frp) reverse proxy, built with [Tauri v2](https://tauri.app). Supports macOS and Windows.

**[简体中文](./README.md)** · English

---

A friendly desktop GUI for the [frp](https://github.com/fatedier/frp) reverse proxy.
You bring your own frps server (or any community frps you trust) — Moonbox Desktop
takes care of the rest: configuration, lifecycle, connection health, auto-update,
and a polished tray-resident experience.

## Highlights

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

## Install

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

## Build from source

```bash
pnpm install
pnpm sync:frpc        # download frpc sidecar binaries
pnpm tauri dev        # local dev
pnpm tauri build      # production build for current platform
```

> Requires Node.js, pnpm, Rust toolchain, and platform-specific build tools.
> See [CONTRIBUTING.md](./CONTRIBUTING.md) for details.

## License

[MIT](./LICENSE).
