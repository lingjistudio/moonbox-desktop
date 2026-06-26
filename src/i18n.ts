import { createI18n } from "vue-i18n";

import zhCN from "./locales/zh-CN";
import en from "./locales/en";

/**
 * 应用支持的语言集合。新增语言时：
 * 1. 在 `locales/` 下新增 `<locale>.ts` 字典；
 * 2. 把语言 code 加入 `AppLocale` 联合类型与 `SUPPORTED_LOCALES`；
 * 3. 在 `LOCALE_LABELS` 中补充面向用户的展示名（用该语言自身的写法）。
 */
export type AppLocale = "zh-CN" | "en";

/** 默认语言。首次启动 / prefs 未存语言时使用。 */
export const DEFAULT_LOCALE: AppLocale = "zh-CN";

/** 支持的语言列表（用于切换 UI 渲染选项）。 */
export const SUPPORTED_LOCALES: AppLocale[] = ["zh-CN", "en"];

/** 各语言在切换 UI 上的展示标签（用该语言自身写法）。 */
export const LOCALE_LABELS: Record<AppLocale, string> = {
  "zh-CN": "简体中文",
  en: "English",
};

/**
 * vue-i18n 实例。
 * - `legacy: false`：走 Composition API（`useI18n` / `$t` 都支持）；
 * - `globalInjection: true`：模板里 `$t` 直接可用，无需每个组件 `useI18n`；
 * - `fallbackLocale: zh-CN`：英文 key 缺失时自动回退到中文，避免显示 key 字面量。
 */
export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: DEFAULT_LOCALE,
  fallbackLocale: DEFAULT_LOCALE,
  messages: {
    "zh-CN": zhCN,
    en,
  },
});

/**
 * 设置当前语言。会同步刷新所有响应式依赖 `$t()` 的视图。
 * 调用方负责把对应值持久化到 prefs.language（详见 `commands/prefs.ts`）。
 */
export function setLocale(locale: AppLocale): void {
  i18n.global.locale.value = locale;
}

/** 读取当前生效的语言 code。 */
export function getLocale(): AppLocale {
  return i18n.global.locale.value as AppLocale;
}

/**
 * 把任意字符串规约为受支持的 AppLocale；非法值回退到默认语言。
 * 用于 prefs 读取后赋值给 i18n 实例。
 */
export function normalizeLocale(raw: unknown): AppLocale {
  if (typeof raw === "string" && (SUPPORTED_LOCALES as string[]).includes(raw)) {
    return raw as AppLocale;
  }
  return DEFAULT_LOCALE;
}
