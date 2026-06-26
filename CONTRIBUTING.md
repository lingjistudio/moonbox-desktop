# 贡献指南

感谢你对 Moonbox Desktop 的关注！本文档说明如何本地搭建开发环境，以及如何参与贡献。

## 一、本地开发环境

### 1.1 依赖

| 工具 | 最低版本 | 备注 |
| --- | --- | --- |
| Node.js | LTS（>= 20） | 前端构建 |
| pnpm | >= 9 | 包管理器（必须，不要用 npm / yarn） |
| Rust | stable | 后端构建 |
| Tauri CLI | v2 | 通过 `pnpm tauri` 调用，无需全局安装 |
| Python | >= 3.8 | 仅在跑 `scripts/check-icons.py` 校验图标时需要 |

macOS 还需 Xcode Command Line Tools；Windows 需 Visual Studio C++ Build Tools（WebView2 runtime）。

### 1.2 首次启动

```bash
git clone https://github.com/lingjistudio/moonbox-desktop.git
cd moonbox-desktop
pnpm install
pnpm sync:frpc            # 下载当前平台的 frpc sidecar
pnpm tauri dev            # 启动联调
```

`pnpm sync:frpc` 默认下载**当前平台**的 frpc；如需补齐其他平台（macOS Intel /
Windows ARM64 等），运行：

```bash
pnpm sync:frpc -- --all --force
```

二进制按平台目标命名后落到 `src-tauri/binaries/`（已加入 `.gitignore`，不入库）。

### 1.3 常用脚本

| 命令 | 用途 |
| --- | --- |
| `pnpm dev` | 仅前端开发服务器（http://localhost:1420） |
| `pnpm tauri dev` | 前后端联调（推荐） |
| `pnpm build` | 前端构建 |
| `pnpm tauri build` | 当前平台打包 |
| `pnpm sync:frpc` | 同步 frpc sidecar 二进制 |
| `./scripts/check-icons.py` | 校验 `docs/app-icons/` 下图标规范 |

## 二、代码约定

- 前端约定见 [`src/AGENTS.md`](./src/AGENTS.md)
- 后端约定见 [`src-tauri/AGENTS.md`](./src-tauri/AGENTS.md)
- 顶层架构与跨层术语见 [`AGENTS.md`](./AGENTS.md)

核心原则：**简洁、优雅、反抽象、直指本质**。三行相似代码胜过一处过早抽象；
只有出现第 3 个同构实现时才抽公共 helper（Rule of Three）。

## 三、提交规范

### 3.1 Commit message

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
<type>(<scope>): <subject>

[optional body]
[optional footer]
```

常用 type：

- `feat`：新功能
- `fix`：bug 修复
- `refactor`：重构（不改变外部行为）
- `docs`：文档变更
- `chore`：构建 / 工具 / 依赖等杂项
- `test`：测试
- `ci`：CI 配置

### 3.2 PR 准则

- 一个 PR 只解决一件事，**严禁夹带无关改动**
- 跨模块改动先开 Issue 讨论
- 文档与代码同步更新（特别是改了命令名 / 事件名 / 字段）
- 改了 `docs/app-icons/**` 时 PR 必须跑过 `./scripts/check-icons.py --strict`

## 四、Release 流程

正式 Release 由 **GitHub Actions** 自动完成，触发条件：推送形如 `v*.*.*` 的 tag。

### 4.1 发布步骤

```bash
# 1. 升版本号（三处 / 四处同步，见 AGENTS.md §版本号同步规范）
#    - src-tauri/tauri.conf.json
#    - package.json
#    - src-tauri/Cargo.toml
#    - src/composables/useAppUpdate.ts (APP_VERSION)

# 2. 跑 cargo check 刷新 Cargo.lock
cd src-tauri && cargo check && cd ..

# 3. commit + tag + push
git commit -am "chore(release): bump version to x.y.z"
git tag vx.y.z
git push origin main vx.y.z
```

推送 tag 后 `.github/workflows/release.yml` 会：

1. 在 macos-latest (Apple Silicon + Intel 双 target) 与 windows-latest runner 上并行构建
2. 用 GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` 签名 updater 包
3. 把 `.dmg` / `.exe` / `.app.tar.gz` 等产物上传到 Release
4. 生成 `latest.json` manifest 上传到 Release（供客户端 updater 拉取）

### 4.2 Updater 签名密钥

应用本体自更新依赖 Tauri updater 签名。仓库 `tauri.conf.json` 的 `pubkey` 字段
保存**公钥**（任何人可见）；对应的**私钥**作为 GitHub Secret
`TAURI_SIGNING_PRIVATE_KEY` 仅 maintainer 持有。

如需轮换签名密钥：

```bash
pnpm tauri signer generate -w ~/.tauri/moonbox-desktop.key
# 把生成的公钥贴到 src-tauri/tauri.conf.json 的 plugins.updater.pubkey
# 把私钥作为 GitHub Secret 配置到仓库设置 → Secrets and variables → Actions
```

> ⚠️ 私钥泄露后必须立即轮换：旧版本无法撤销，但新版本可以用新密钥签名，
> 客户端从旧版本升级到泄露后第一个签名版本时就会切换信任。

## 五、行为准则

参与本项目的每一位贡献者都需要遵守 [MIT 行为准则](https://www.contributor-covenant.org/version/2/1/code_of_conduct/)。

简而言之：保持友善、就事论事、对新手友好。
