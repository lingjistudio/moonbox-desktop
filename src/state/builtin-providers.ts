import { ref } from "vue";

import type { Provider } from "../types";

/**
 * 内置服务商清单：从打包根下的 builtin-providers.json 异步加载。
 * 加载失败时保持空数组，不阻塞应用；UI 通过响应式感知。
 */
export const builtinProviders = ref<Provider[]>([]);

let builtinProvidersLoaded = false;

/** 从 builtin-providers.json 加载内置服务商清单；失败静默，仅 console.warn。 */
export async function loadBuiltinProviders(): Promise<void> {
  if (builtinProvidersLoaded) return;
  builtinProvidersLoaded = true;
  try {
    const res = await fetch("./builtin-providers.json", { cache: "no-cache" });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    if (Array.isArray(data)) {
      builtinProviders.value = (data as Provider[]).map((p) => ({
        ...p,
        user: p.user ?? "",
        username_required: p.username_required === true,
      }));
    }
  } catch (e) {
    console.warn("[builtinProviders] 加载失败：", e);
  }
}

void loadBuiltinProviders();
