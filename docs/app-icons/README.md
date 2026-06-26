# 图标资源规范

> 本目录（`docs/app-icons/`）存放所有图标的**设计源文件**。
> 打包时由 Tauri 工具链自动转成最终各档产物（写入 `src-tauri/icons/`）。
>
> 更新任何图标后，**必须先跑** `scripts/check-icons.py` 校验合规，再提交。

## 目录资产清单

| 文件 | 用途 | 关键规范 |
| --- | --- | --- |
| `icon_256.svg` | 矢量源 | Figma / Sketch 导出 |
| `icon_64.png` | APP Icon @64 | 64 × 64 px |
| `icon_71.png` | APP Icon @71 | 71 × 71 px |
| `icon_150.png` | APP Icon @150 | 150 × 150 px |
| `icon_300.png` | APP Icon @300 | 300 × 300 px |
| `icon_512.png` | APP Icon @512 | 512 × 512 px |
| `icon_512x512@2x-macos.png` | APP Icon @2x macOS | 1024 × 1024 px |
| `icon_512x512@2x-windows.png` | APP Icon @2x Windows | 1024 × 1024 px |
| `tray_macos_44.png` | **macOS 菜单栏托盘图标** | 见下「托盘图标规范」 |
| `tray_windows_64.png` | **Windows 通知区托盘图标** | 见下「托盘图标规范」 |

## APP Icon 规范

- 画布方形、内容居中、四角圆角可保留（外层打包由 Tauri 处理）
- 整张图必须使用同一品牌主色与背景，**禁止在源文件中混用不同色系**
- 各档位尺寸严格匹配上表；矢量 → 位图重采样应在矢量源上完成，**禁止把低档位放大冒充高档位**

## 托盘图标规范（关键！）

托盘图标会被缩放到 22 × 22 pt 渲染，**一切彩色 / 渐变 / 阴影 / 圆角背板在缩放后都会变糊**。
两个平台规范不同，**必须分开设计、分开导出**。

### macOS 菜单栏（`tray_macos_44.png`）

依据：[Apple HIG — Menu bar extras](https://developer.apple.com/design/human-interface-guidelines/menu-bars)

| 属性 | 要求 |
| --- | --- |
| 格式 | PNG 32 位 RGBA |
| 画布 | **44 × 44 px**（@2x） |
| 实际图形 | 约 **34 × 34 px**，四周留 5 px 安全边距 |
| 配色 | **纯黑 `#000000`** + 透明背景 |
| 禁止 | 任何彩色 / 渐变 / 阴影 / 圆角背板 |
| 反色 | 启用 `icon_as_template(true)` 后系统按明暗主题自动反色（深色模式显白） |

### Windows 通知区（`tray_windows_64.png`）

依据：[Microsoft — Notification area icons](https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/app-icons)

| 属性 | 要求 |
| --- | --- |
| 格式 | ICO（多尺寸 16/20/24/32/40/64 px 内嵌）或 PNG |
| 画布 | **64 × 64 px**（Tauri 运行时缩放至目标尺寸） |
| 实际图形 | 约 **56 × 56 px**，四周留 4 px 边距 |
| 配色 | **纯白 `#FFFFFF`** + 透明背景 |
| 禁止 | 任何彩色 / 渐变 / 阴影 / 圆角背板 |

## 更新工作流

```bash
# 1. 设计师导出 / 替换 docs/app-icons/ 下的 PNG / SVG
# 2. 跑规范检查
./scripts/check-icons.py

# 严格模式（CI 用）：任何警告都视为失败
./scripts/check-icons.py --strict

# 3. 通过后再提交；打包时 Tauri 工具链自动同步到 src-tauri/icons/
```

**违反规范的常见原因**：

| 现象 | 根因 | 修复 |
| --- | --- | --- |
| macOS 托盘在浅色菜单栏显示为黑方块 | 白底未删除 | 在源文件中删除背景图层，**只留纯黑线条 + 透明**，重新导出 PNG |
| Windows 托盘在深色任务栏看不见 | 黑色 / 彩色填充 | 改为纯白 `#FFFFFF` + 透明 |
| 菜单栏图标糊成一团 | 用了完整 APP Icon | 重画简化版，**只保留 1~2 个核心轮廓元素** |
| 4 角不透明（脚本报错「背板未清除」）| Figma 导出时漏了 alpha 通道 | 导出设置选 **PNG / 32 位 / 含 alpha** |

## 为什么有专门的托盘图标

APP Icon 缩到 22 px 会糊、彩色会让菜单栏出现"贴纸"违和感。
托盘图标是另一个独立设计资产，**不要图省事直接拿 APP Icon 缩放当托盘用**。
