<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";

import { prefs } from "../../state";
import { savePrefs } from "../../commands/prefs";
import {
  setLocale,
  normalizeLocale,
  SUPPORTED_LOCALES,
  LOCALE_LABELS,
  type AppLocale,
} from "../../i18n";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";

const { t } = useI18n();
const { toast, showToast } = useToast();

const savingLang = ref(false);

async function onChangeLanguage(next: AppLocale) {
  if (next === prefs.language) return;
  if (savingLang.value) return;
  savingLang.value = true;
  // 先改写 prefs；切换是 i18n 自己的事（响应式），再保存到后端
  const prev = prefs.language;
  prefs.language = next;
  setLocale(next);
  const err = await savePrefs();
  savingLang.value = false;
  if (err) {
    // 回滚
    prefs.language = prev;
    setLocale(normalizeLocale(prev));
    showToast(t("msg_save_failed", { err }), "error", 3500);
    return;
  }
  showToast(
    t("msg_language_changed", { lang: LOCALE_LABELS[next] }),
    "success",
    1500,
  );
}
</script>

<template>
  <div class="tab-pane">
    <!-- 界面 -->
    <section class="card section-card">
      <div class="section-title">{{ $t("interface_section") }}</div>

      <div class="pref-row">
        <div class="pref-header">
          <div class="pref-title">{{ $t("interface_language") }}</div>
          <div class="lang-segmented" role="radiogroup" :aria-label="$t('interface_language')">
            <button
              v-for="code in SUPPORTED_LOCALES"
              :key="code"
              type="button"
              role="radio"
              class="lang-btn"
              :class="{ active: prefs.language === code }"
              :aria-checked="prefs.language === code"
              :disabled="savingLang"
              @click="onChangeLanguage(code)"
            >{{ LOCALE_LABELS[code] }}</button>
          </div>
        </div>
        <div class="pref-desc">{{ $t("interface_language_desc") }}</div>
      </div>
    </section>

    <Toast :toast="toast" />
  </div>
</template>

<style scoped>
.tab-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  gap: 12px;
}
.section-card {
  padding: 14px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 12px;
}
.pref-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 0;
}
.pref-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.pref-title {
  font-size: 13px;
  font-weight: 500;
  color: hsl(var(--foreground));
}
.pref-desc {
  font-size: 11px;
  color: hsl(var(--muted-foreground));
  line-height: 1.5;
}

/* 界面语言分段控件 */
.lang-segmented {
  display: inline-flex;
  background: hsl(var(--muted));
  padding: 3px;
  border-radius: var(--radius);
  gap: 2px;
  flex-shrink: 0;
}
.lang-btn {
  padding: 4px 10px;
  border-radius: calc(var(--radius) - 3px);
  font-size: 12px;
  font-weight: 500;
  background: transparent;
  border: none;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: color 0.15s, background-color 0.15s, box-shadow 0.15s;
}
.lang-btn:hover:not(.active):not(:disabled) {
  color: hsl(var(--foreground));
}
.lang-btn.active {
  background: hsl(var(--background));
  color: hsl(var(--foreground));
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}
.lang-btn:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}
.lang-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px hsl(var(--ring) / 0.35);
}
</style>
