<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import { prefs } from "../../state";
import { savePrefs } from "../../commands/prefs";
import type { Schedule } from "../../types";
import { useToast } from "../../composables/useToast";

const { t } = useI18n();
const { showToast } = useToast();

const WEEKDAY_LABELS = [
  "weekday_short_mon",
  "weekday_short_tue",
  "weekday_short_wed",
  "weekday_short_thu",
  "weekday_short_fri",
  "weekday_short_sat",
  "weekday_short_sun",
];

/** 编辑副本：只在「保存」时写回 prefs；与 ProviderTab / ProxyTab 一致 */
const form = ref<Schedule>(cloneSchedule(prefs.schedule));
const savingSchedule = ref(false);

function cloneSchedule(s: Schedule): Schedule {
  return {
    enabled: s.enabled,
    weekdays: [...s.weekdays] as Schedule["weekdays"],
    start_time: s.start_time,
    stop_time: s.stop_time,
  };
}

function toggleWeekday(i: number) {
  if (!form.value.enabled) return;
  form.value.weekdays[i] = !form.value.weekdays[i];
}

function onTimeChange(e: Event, key: "start_time" | "stop_time") {
  const v = (e.target as HTMLInputElement).value;
  if (v) form.value[key] = v;
}

/**
 * 校验：返回 `null` 表示通过，否则返回错误文案。
 * 启用态下：至少一天 + 时间合法 + 起早于止（不支持跨夜）。
 */
const validationError = computed<string | null>(() => {
  if (!form.value.enabled) return null;
  if (!form.value.weekdays.some((x) => x)) return t("schedule_err_no_day");
  if (!form.value.start_time || !form.value.stop_time) return t("schedule_err_no_time");
  if (form.value.start_time === form.value.stop_time)
    return t("schedule_err_same_time");
  if (form.value.start_time > form.value.stop_time)
    return t("schedule_err_order");
  return null;
});

function scheduleDesc(): string {
  if (!form.value.enabled) {
    return t("schedule_desc_disabled");
  }
  const days = form.value.weekdays
    .map((on, i) => (on ? t(WEEKDAY_LABELS[i]) : null))
    .filter(Boolean)
    .join("、");
  return t("schedule_summary", {
    days: days || t("schedule_desc_unselected"),
    start: form.value.start_time,
    stop: form.value.stop_time,
  });
}

async function onSaveSchedule() {
  if (validationError.value) {
    showToast(validationError.value, "error", 3000);
    return;
  }
  savingSchedule.value = true;
  prefs.schedule = cloneSchedule(form.value);
  const err = await savePrefs();
  savingSchedule.value = false;
  if (err) {
    showToast(t("msg_save_failed", { err }), "error", 3500);
    return;
  }
  showToast(t("msg_schedule_saved"), "success", 1500);
}

/** prefs 在外部被刷新（如重新 loadPrefs）时同步副本 */
onMounted(() => {
  form.value = cloneSchedule(prefs.schedule);
});
</script>

<template>
  <section class="card section-card">
    <div class="section-title">{{ $t("schedule_section") }}</div>

    <div class="pref-row">
      <div class="pref-text">
        <div class="pref-title">{{ $t("schedule_enable") }}</div>
        <div class="pref-desc">{{ scheduleDesc() }}</div>
      </div>
      <button
        type="button"
        class="toggle"
        role="switch"
        :aria-checked="form.enabled"
        :class="{ on: form.enabled }"
        @click="form.enabled = !form.enabled"
      >
        <span class="toggle-knob" />
      </button>
    </div>

    <div class="pref-divider" />

    <div class="pref-row vertical" :class="{ disabled: !form.enabled }">
      <div class="pref-text">
        <div class="pref-title">{{ $t("schedule_days_label") }}</div>
        <div class="pref-desc">{{ $t("schedule_days_desc") }}</div>
      </div>
      <div class="weekday-picker">
        <button
          v-for="(key, i) in WEEKDAY_LABELS"
          :key="i"
          type="button"
          class="day"
          :class="{ on: form.weekdays[i] }"
          :disabled="!form.enabled"
          :aria-pressed="form.weekdays[i]"
          :aria-label="$t(key)"
          @click="toggleWeekday(i)"
        >{{ $t(key) }}</button>
      </div>
    </div>

    <div class="pref-divider" />

    <div class="pref-row" :class="{ disabled: !form.enabled }">
      <div class="pref-text">
        <div class="pref-title">{{ $t("schedule_time_label") }}</div>
        <div class="pref-desc">{{ $t("schedule_time_desc") }}</div>
      </div>
      <div class="time-row">
        <input
          type="time"
          class="time-input"
          :value="form.start_time"
          :disabled="!form.enabled"
          @change="onTimeChange($event, 'start_time')"
        />
        <span class="time-sep">→</span>
        <input
          type="time"
          class="time-input"
          :value="form.stop_time"
          :disabled="!form.enabled"
          @change="onTimeChange($event, 'stop_time')"
        />
      </div>
    </div>

    <div class="pref-divider" />

    <div class="pref-actions">
      <button
        type="button"
        class="btn btn-primary btn-sm"
        :disabled="!!validationError || savingSchedule"
        @click="onSaveSchedule"
      >{{ savingSchedule ? $t("common_saving") : $t("common_save") }}</button>
      <span v-if="validationError" class="pref-hint">{{ validationError }}</span>
    </div>
  </section>
</template>

<style scoped>
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
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 4px 0;
}
.pref-row.vertical {
  align-items: flex-start;
  flex-direction: column;
  gap: 8px;
}
.pref-row.disabled {
  opacity: 0.5;
}
.pref-text {
  flex: 1;
  min-width: 0;
}
.pref-title {
  font-size: 13px;
  font-weight: 500;
  color: hsl(var(--foreground));
  margin-bottom: 4px;
}
.pref-desc {
  font-size: 11.5px;
  color: hsl(var(--muted-foreground));
  line-height: 1.5;
}
.pref-divider {
  height: 1px;
  background: hsl(var(--border));
  margin: 10px 0;
}
.toggle {
  --w: 36px;
  --h: 20px;
  --pad: 2px;
  width: var(--w);
  height: var(--h);
  border-radius: calc(var(--h) / 2);
  background: hsl(var(--muted-foreground) / 0.4);
  border: none;
  padding: 0;
  cursor: pointer;
  position: relative;
  flex-shrink: 0;
  transition: background-color 0.18s ease;
}
.toggle.on {
  background: hsl(var(--primary));
}
.toggle:disabled {
  cursor: not-allowed;
}
.toggle-knob {
  position: absolute;
  top: var(--pad);
  left: var(--pad);
  width: calc(var(--h) - var(--pad) * 2);
  height: calc(var(--h) - var(--pad) * 2);
  border-radius: 50%;
  background: hsl(var(--background));
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  transition: transform 0.18s cubic-bezier(0.4, 0, 0.2, 1);
}
.toggle.on .toggle-knob {
  transform: translateX(calc(var(--w) - var(--h)));
}
.toggle:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px hsl(var(--ring) / 0.35);
}
.weekday-picker {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  width: 100%;
}
.day {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--card));
  color: hsl(var(--muted-foreground));
  font-size: 11px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s, border-color 0.15s;
  padding: 0;
}
.day.on {
  background: hsl(var(--primary));
  color: hsl(var(--primary-foreground));
  border-color: hsl(var(--primary));
}
.day:disabled {
  cursor: not-allowed;
}
.day:focus-visible {
  outline: 2px solid hsl(var(--ring));
  outline-offset: 2px;
}
.time-row {
  display: flex;
  align-items: center;
  gap: 6px;
}
.time-input {
  font: inherit;
  padding: 4px 6px;
  border: 1px solid hsl(var(--border));
  border-radius: 4px;
  background: hsl(var(--input) / 0.4);
  color: hsl(var(--foreground));
  width: 92px;
}
.time-input:disabled {
  cursor: not-allowed;
}
.time-input:focus-visible {
  outline: none;
  border-color: hsl(var(--ring));
  box-shadow: 0 0 0 3px hsl(var(--ring) / 0.35);
}
.time-sep {
  color: hsl(var(--muted-foreground));
  font-size: 12px;
}
.pref-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 12px;
}
.pref-hint {
  color: hsl(var(--destructive));
  font-size: 11px;
}
</style>
