#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
图标规范校验脚本 —— Moonbox Desktop

校验范围（默认根目录 docs/app-icons/）：
  - 系统托盘图标（macOS / Windows）—— 尺寸、纯色、4 角透明（无背板）
  - APP Icon —— 各档尺寸（width x height）匹配命名约定

触发时机：
  - 设计师 / 开发更新了 docs/app-icons/ 下的任何图标后
  - 提交代码前 / 在 PR 中作为 CI 检查项
  - 同步到 src-tauri/icons/ 之前

退出码：
  0 = 全部合规
  1 = 发现违规（详见 stdout）

用法：
  ./scripts/check-icons.py                     # 校验默认目录
  ./scripts/check-icons.py --root <dir>        # 自定义图标目录
  ./scripts/check-icons.py --strict            # 严格模式：缺失文件也视为违规（默认仅警告）
  ./scripts/check-icons.py --help

依赖：Python 3.8+、Pillow（pip install Pillow）
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Optional

try:
    from PIL import Image  # type: ignore
except ImportError:
    print("缺少依赖 Pillow。请先安装：  pip install Pillow", file=sys.stderr)
    sys.exit(2)


# ---------------------------------------------------------------------------
# 规范定义
# ---------------------------------------------------------------------------

# macOS 菜单栏模板图：纯黑 + 透明，无背板。系统按明暗模式自动反色。
# 参考：Apple Human Interface Guidelines — Menu bar extras
TRAY_MACOS = {
    "file": "tray_macos_44.png",
    "size": (44, 44),
    "color_rgb": (0, 0, 0),
    "label": "macOS 菜单栏模板图",
}

# Windows 通知区图标：单色白 + 透明，无背板。系统按明暗模式自动调整可见度。
# 参考：Microsoft — Notification area icons
TRAY_WINDOWS = {
    "file": "tray_windows_64.png",
    "size": (64, 64),
    "color_rgb": (255, 255, 255),
    "label": "Windows 通知区图标",
}

# APP Icon 命名 → 期望尺寸
# 说明：SVG 是矢量源文件，不在尺寸校验范围；只校验存在性
APP_ICONS: list[tuple[str, Optional[tuple[int, int]]]] = [
    ("icon_64.png", (64, 64)),
    ("icon_71.png", (71, 71)),
    ("icon_150.png", (150, 150)),
    ("icon_300.png", (300, 300)),
    ("icon_512.png", (512, 512)),
    ("icon_512x512@2x-macos.png", (1024, 1024)),
    ("icon_512x512@2x-windows.png", (1024, 1024)),
    ("icon_256.svg", None),  # 矢量源，仅校验存在性
]

# 不透明判定阈值（> ALPHA_OPAQUE 视为不透明）
ALPHA_OPAQUE = 250
# 完全透明判定阈值（< ALPHA_TRANSPARENT 视为透明）
ALPHA_TRANSPARENT = 50
# 抗锯齿灰度判定：RGB 三通道差异 ≤ TOLERANCE 视为"接近目标色"
TOLERANCE = 10


# ---------------------------------------------------------------------------
# 校验工具
# ---------------------------------------------------------------------------

class Report:
    """收集校验结果，统一输出格式"""

    def __init__(self) -> None:
        self.passed: list[str] = []
        self.warnings: list[str] = []
        self.failed: list[str] = []

    def ok(self, msg: str) -> None:
        self.passed.append(msg)

    def warn(self, msg: str) -> None:
        self.warnings.append(msg)

    def fail(self, msg: str) -> None:
        self.failed.append(msg)

    @property
    def has_failure(self) -> bool:
        return bool(self.failed)

    def summary(self, strict: bool) -> int:
        print()
        print("=" * 64)
        for msg in self.passed:
            print(f"  ✅ {msg}")
        for msg in self.warnings:
            print(f"  ⚠️  {msg}")
        for msg in self.failed:
            print(f"  ❌ {msg}")
        print("=" * 64)

        fails = len(self.failed)
        warns = len(self.warnings)
        oks = len(self.passed)
        print(f"结果：{oks} 通过 / {warns} 警告 / {fails} 失败")

        if fails > 0:
            return 1
        if strict and warns > 0:
            print("（严格模式下，警告视为失败）")
            return 1
        return 0


def is_color_match(rgb: tuple[int, int, int], target: tuple[int, int, int]) -> bool:
    """RGB 三通道差均 ≤ TOLERANCE 视为匹配（含抗锯齿灰）"""
    return all(abs(a - b) <= TOLERANCE for a, b in zip(rgb, target))


def check_tray_icon(path: Path, spec: dict, report: Report) -> None:
    """校验托盘图标：尺寸 + 纯色 + 4 角透明 + 无背板"""
    label = spec["label"]
    if not path.exists():
        report.fail(f"{label}：文件缺失 {path}")
        return

    try:
        img = Image.open(path).convert("RGBA")
    except Exception as e:
        report.fail(f"{label}：无法解析 ({e})")
        return

    w, h = img.size
    exp_w, exp_h = spec["size"]
    if (w, h) != (exp_w, exp_h):
        report.fail(f"{label}：尺寸 {w}x{h} ≠ 期望 {exp_w}x{exp_h}")
        return

    px = img.load()
    target = spec["color_rgb"]

    # 检查 4 角透明（无背板的关键指标）
    corners = [px[0, 0], px[w - 1, 0], px[0, h - 1], px[w - 1, h - 1]]
    opaque_corners = [i for i, c in enumerate(corners) if c[3] > ALPHA_TRANSPARENT]
    if opaque_corners:
        report.fail(f"{label}：四角存在不透明像素（背板未清除），违规角索引 {opaque_corners}")
        return

    # 统计不透明像素的颜色分布
    total_opaque = 0
    mismatch_pixels = 0
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if a > ALPHA_OPAQUE:
                total_opaque += 1
                if not is_color_match((r, g, b), target):
                    mismatch_pixels += 1

    if total_opaque == 0:
        report.fail(f"{label}：没有任何不透明像素（图标是空的？）")
        return

    if mismatch_pixels > 0:
        # 进一步判断是否含"应被替换的彩色 / 白底 / 黑底"
        target_name = "纯黑(0,0,0)" if target == (0, 0, 0) else "纯白(255,255,255)"
        opposite = (255, 255, 255) if target == (0, 0, 0) else (0, 0, 0)
        opposite_count = 0
        for y in range(h):
            for x in range(w):
                r, g, b, a = px[x, y]
                if a > ALPHA_OPAQUE and is_color_match((r, g, b), opposite):
                    opposite_count += 1
        if opposite_count > 0:
            hint = (
                f"检测到 {opposite_count} 个像素为反色"
                f"({'白底未清除' if target == (0,0,0) else '黑底未清除'})；"
                "应在源文件中删除背景图层后重新导出透明 PNG"
            )
        else:
            hint = "可能含彩色像素，请检查源文件是否被错误栅格化"
        report.fail(
            f"{label}：{mismatch_pixels}/{total_opaque} 不透明像素偏离{target_name}；{hint}"
        )
        return

    report.ok(f"{label}：{w}x{h}，纯色合规，4 角透明（{total_opaque} 个不透明像素全部匹配目标色）")


def check_app_icon(path: Path, expected_size: Optional[tuple[int, int]], report: Report) -> None:
    """校验 APP Icon：存在性 + 尺寸（SVG 跳过尺寸）"""
    if not path.exists():
        report.fail(f"APP Icon 缺失：{path.name}")
        return

    if path.suffix.lower() == ".svg":
        report.ok(f"APP Icon {path.name}：存在（矢量源，跳过尺寸校验）")
        return

    try:
        img = Image.open(path)
        w, h = img.size
    except Exception as e:
        report.fail(f"APP Icon {path.name}：无法解析 ({e})")
        return

    if expected_size is None:
        report.ok(f"APP Icon {path.name}：{w}x{h}（未设期望尺寸，仅校验可解析）")
        return

    exp_w, exp_h = expected_size
    if (w, h) != (exp_w, exp_h):
        report.fail(f"APP Icon {path.name}：尺寸 {w}x{h} ≠ 期望 {exp_w}x{exp_h}")
        return

    report.ok(f"APP Icon {path.name}：{w}x{h}")


# ---------------------------------------------------------------------------
# 主入口
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description="校验 Moonbox Desktop 的托盘图标与 APP Icon 是否符合平台规范。",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="详细规范见 docs/app-icons/README.md 或 scripts/check-icons.py 文件头注释。",
    )
    parser.add_argument(
        "--root",
        default="docs/app-icons",
        help="图标根目录（默认：仓库根目录下的 docs/app-icons）",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="严格模式：缺失文件 / 警告也视为失败",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    if not root.is_dir():
        print(f"错误：图标目录不存在 {root}", file=sys.stderr)
        return 2

    print(f"校验目录：{root}")
    print(f"严格模式：{'开启' if args.strict else '关闭'}")
    print()

    report = Report()

    # 托盘图标
    print("[1/2] 系统托盘图标")
    check_tray_icon(root / TRAY_MACOS["file"], TRAY_MACOS, report)
    check_tray_icon(root / TRAY_WINDOWS["file"], TRAY_WINDOWS, report)

    # APP Icon
    print()
    print("[2/2] APP Icon")
    for fname, expected in APP_ICONS:
        check_app_icon(root / fname, expected, report)

    return report.summary(args.strict)


if __name__ == "__main__":
    sys.exit(main())
