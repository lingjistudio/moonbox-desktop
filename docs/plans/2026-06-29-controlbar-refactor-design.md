# HomeView ControlBar 重构 — 设计文档

> 日期：2026-06-29
> 状态：设计阶段，待主公裁决后进入实施

## 背景与目标

`HomeView` 当前布局中，圆形大按钮的 `ripple-wrapper` 占 360×360，而按钮本身只有 160×160，
二者合计占据主面板 ~400px 的垂直空间（占 740px 窗口高度的 54%），导致访问地址列表
被挤在 ~196px 内，仅能展示 3–4 条就要滚动。

**目标**：把大圆按钮替换为顶部 ~96px 的横向控制条 `ControlBar`，释放空间给访问地址列表
（容量从 3–4 条提升到 9–10 条双行 / ~15 条紧凑），同时保留圆形按钮作为视觉锚点。

## 选型对比（已与主公确认方向）

| 维度 | 当前 | 新方案 |
|---|---|---|
| 按钮尺寸 | 160×160 | 80×80 |
| 按钮动效 | Canvas 水波纹粒子（`useParticles`） | CSS `box-shadow` 呼吸光晕 |
| 按钮所在容器 | 360×360 ripple-wrapper | 横向控制条（高度 ~96px） |
| 设置齿轮位置 | `position: absolute; top: 10px; right: 10px` 浮动 | ControlBar 右槽固定 |
| ProxyList 可用高度 | ~196px | ~500px |
| 双行布局容量 | 3–4 条 | 9–10 条 |
| 紧凑布局容量 | ~6 条 | ~15 条 |

### 排除的方向（备忘）

- **卡片化状态条**：完全去掉圆形按钮，视觉锚点丢失
- **FAB 浮动按钮**：状态语义承载困难
- **左右分栏**：400px 窗口宽度不足
- **保守缩小（240/120）**：主公已表达希望探索更彻底的方案

## 组件结构变化

### 新组件层级

```
HomeView
├── ControlBar（新组件，纵向 ~96px）
│   ├── RippleCircle（80×80 按钮 + CSS 呼吸光晕）
│   ├── StatusText（双行：主标 + 副提示）
│   └── SettingsBtn（30×30 图标按钮，右槽）
├── GuideCard（v-if 未配置）
├── ProxyList
└── SystemStatus
```

### 关键文件变更

| 文件 | 操作 | 说明 |
|---|---|---|
| `src/components/home/ControlBar.vue` | 新建 | 主件，承载按钮 + 文案 + 齿轮 |
| `src/components/home/CircleButton.vue` | 删除 | 整文件移除 |
| `src/composables/useParticles.ts` | 删除 | 粒子不再需要 |
| `src/views/HomeView.vue` | 改 | 用 ControlBar 替换 CircleButton + 浮动齿轮 |
| `src/CLAUDE.md`（即 `src/AGENTS.md`） | 改 | §5.4 子件列表更新（增 ControlBar，删 CircleButton） |

## ControlBar.vue 设计规格

### 模板骨架

```vue
<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { computed } from "vue";
import { Settings } from "@lucide/vue";
import { frpcStatus } from "../../state/runtime";
import type { FrpcStatus } from "../../types";

defineProps<{ disabled: boolean }>();
defineEmits<{ click: []; settings: [] }>();

const { t: $t } = useI18n();

/** 与现 CircleButton 4 态文案同构，沿用现有 i18n key */
const STATUS_KEYS: Record<FrpcStatus, { label: string; hint: string; aria: string }> = {
  stopped:    { label: "home_btn_stopped",    hint: "home_btn_hint_stopped",    aria: "home_btn_aria_stopped" },
  connecting: { label: "home_btn_connecting", hint: "home_btn_hint_connecting", aria: "home_btn_aria_connecting" },
  connected:  { label: "home_btn_connected",  hint: "home_btn_hint_connected",  aria: "home_btn_aria_connected" },
  error:      { label: "home_btn_error",      hint: "home_btn_hint_error",      aria: "home_btn_aria_error" },
};

const buttonClass = computed(() => `toggle-${frpcStatus.value}`);
const statusLabel = computed(() => $t(STATUS_KEYS[frpcStatus.value].label));
const statusHint  = computed(() => $t(STATUS_KEYS[frpcStatus.value].hint));
const toggleAria  = computed(() => $t(STATUS_KEYS[frpcStatus.value].aria));
</script>

<template>
  <section class="control-bar">
    <div class="control-main">
      <button
        class="ripple-circle"
        :class="buttonClass"
        :disabled="disabled"
        :aria-label="toggleAria"
        @click="$emit('click')"
      >
        <!-- 沿用现 CircleButton.svg 256×256 viewBox，width/height 26 -->
        <svg class="toggle-icon" xmlns="..." viewBox="0 0 256 256" width="26" height="26">…</svg>
      </button>
      <div class="control-text">
        <div class="control-label">{{ statusLabel }}</div>
        <div class="control-hint">{{ statusHint }}</div>
      </div>
    </div>
    <button
      class="control-settings-btn"
      @click="$emit('settings')"
      :title="$t('home_settings_title')"
      :aria-label="$t('home_settings_title')"
    >
      <Settings :size="18" />
    </button>
  </section>
</template>
```

### CSS 规格

```css
.control-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  flex-shrink: 0;
}

.control-main {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
}

/* 80×80 圆形按钮：状态色 + 呼吸光晕 */
.ripple-circle {
  flex-shrink: 0;
  width: 80px;
  height: 80px;
  border-radius: 50%;
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: opacity 0.15s, filter 0.15s;
  font-family: inherit;
  color: hsl(var(--primary-foreground));
}
.ripple-circle:hover:not(:disabled) { filter: brightness(1.08); }
.ripple-circle:disabled { opacity: 0.5; cursor: not-allowed; }

/* 4 态语义色（与现 CircleButton 一致） */
.toggle-stopped    { background: hsl(var(--muted-foreground)); }
.toggle-connecting { background: hsl(var(--warning)); }
.toggle-connected  { background: hsl(var(--success)); }
.toggle-error      { background: hsl(var(--destructive)); }

/* CSS 呼吸光晕取代 Canvas 粒子 */
.toggle-connected {
  animation: glow-pulse-success 3s ease-in-out infinite;
}
.toggle-connecting {
  animation: glow-pulse-warning 1s ease-in-out infinite;
}
/* stopped / error：无动画，避免误导 */

@keyframes glow-pulse-success {
  0%, 100% { box-shadow: 0 0 0 0 hsl(var(--success) / 0.5); }
  50%      { box-shadow: 0 0 0 12px hsl(var(--success) / 0); }
}
@keyframes glow-pulse-warning {
  0%, 100% { box-shadow: 0 0 0 0 hsl(var(--warning) / 0.6); }
  50%      { box-shadow: 0 0 0 8px hsl(var(--warning) / 0); }
}

.toggle-icon {
  color: inherit;
}

/* 文案双行 */
.control-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.control-label {
  font-size: 14px;
  font-weight: 700;
  letter-spacing: 0.3px;
}
.control-hint {
  font-size: 11px;
  font-weight: 500;
  color: hsl(var(--muted-foreground));
  margin-top: 1px;
}

/* 齿轮按钮：沿用现 home-settings-btn 视觉 */
.control-settings-btn {
  flex-shrink: 0;
  width: 30px;
  height: 30px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}
.control-settings-btn:hover {
  background: hsl(var(--accent));
  color: hsl(var(--foreground));
}
```

### 控制条总高度计算

```
12 padding-top
+ 80 (button max height)
+ 12 padding-bottom
= ~104px（含 gap；最坏情况 ~96–104px）
```

### 文案切换矩阵

| 状态 | 主标 (`home_btn_*`) | 副提示 (`home_btn_hint_*`) |
|---|---|---|
| stopped | 已停止 | 点击启动 |
| connecting | 连接中 | 正在连接服务端 |
| connected | 已连接 | 点击停止服务 |
| error | 连接错误 | 点击查看日志 |

> 4 态 i18n key 全部复用现有 `home_btn_*` / `home_btn_hint_*` / `home_btn_aria_*`，
> 不新增 key。`home_settings_title` 沿用齿轮 title/aria-label。

## HomeView.vue 改动

### 删除

```ts
import CircleButton from "../components/home/CircleButton.vue";
```

```html
<button class="home-settings-btn" @click="emit('settings')">…</button>
```

### 新增

```ts
import ControlBar from "../components/home/ControlBar.vue";
```

```html
<ControlBar :disabled="!isConfigured()" @click="onToggle" @settings="emit('settings')" />
```

### CSS 删除

- `.home-settings-btn` 全部规则
- 不再有"齿轮浮动"相关 absolute 定位

## useParticles.ts / CircleButton.vue 处置

- `useParticles.ts`：**整文件删除**（YAGNI，仅被 CircleButton 引用）
- `CircleButton.vue`：**整文件删除**
- 删除后需 `grep -r "useParticles" src/` 确认无悬挂引用

## 验证清单

- [ ] 4 态按钮视觉：stopped / connecting / connected / error 颜色正确
- [ ] connecting 黄色脉冲 ~1s/次（明确"正在连接"语义）
- [ ] connected 绿色呼吸 ~3s/次（静默优雅）
- [ ] stopped / error 无动画（避免误导）
- [ ] 按钮 `disabled` 态（未配置时）视觉正确
- [ ] 文案双行：主标 + 副提示，4 态切换正确
- [ ] 列表可见容量实测：双行 ~9-10 条 / 紧凑 ~15 条（无需滚动）
- [ ] 齿轮点击 → emit `settings` → App.vue 切换到 settings 视图
- [ ] 全局快捷键（Cmd/Ctrl+W/M、Esc）仍工作
- [ ] macOS 交通灯 + Windows 窗口控制按钮无冲突
- [ ] 跨平台视觉一致（macOS / Windows）
- [ ] `pnpm tauri dev` 启动后目测无误
- [ ] `pnpm build` 通过
- [ ] TypeScript 编译无新增错误（strict / noUnusedLocals）

## 风险点

1. **粒子系统残留引用**：`useParticles` 仅被 `CircleButton.vue` 引用，删除两文件后
   必须 grep 确认无悬挂 import。
2. **CLAUDE.md §5.4 同步**：子件列表增删（CircleButton → ControlBar，新增 useParticles 删除说明）。
3. **CLAUDE.md §5.5 历史漂移**（**非任务问题，不修改，仅记录**）：
   文档描述"TitleBar 右槽在 home 视图显示设置齿轮"，实际 TitleBar.vue 右槽只有 Windows 窗口控制，
   设置齿轮由 HomeView 自己浮动。这是历史漂移，按"专注原则"不夹带修复，待后续触碰时再处理。
4. **齿轮迁移交互一致性**：从 `position: absolute` 浮动改为 ControlBar 右槽，需确认
   `emit('settings')` 链路无变化（HomeView → App.vue 切视图）。
5. **窗口高度自适应**：当前 home-body 用 `flex: 1; overflow-y: auto` 自动滚动，
   ControlBar 高度变化后此机制应自动适配，无需额外逻辑。

## 非任务问题（仅提示）

- CLAUDE.md §5.5 描述与实际实现漂移（TitleBar 右槽未按文档显示齿轮）
- `useParticles.ts` 内 `BUTTON_RADIUS = 80` 是与 CircleButton 160×160 配套的硬编码，
  删除后此常量自然消亡，不影响其他模块

## 后续流程

设计完成后：
1. 主公确认设计 → 本文档提交到 git
2. 使用 `superpowers:writing-plans` 出详细实施计划（含 TDD 测试点）
3. 实施前用 `superpowers:using-git-worktrees` 隔离工作区