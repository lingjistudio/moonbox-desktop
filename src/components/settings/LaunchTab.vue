<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import { prefs } from "../../state";
import { setAutoLaunch, savePrefs } from "../../commands/prefs";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";
import ScheduleSection from "./ScheduleSection.vue";

const { t } = useI18n();
const { toast, showToast } = useToast();

const launching = ref(false);
const savingSilent = ref(false);
const savingAutoConnect = ref(false);

/** 依赖开机启动的子开关（静默启动 / 开机自动连接）共享同一启用条件。 */
const launchDependent = computed(() => prefs.auto_launch);

async function onToggleAutoLaunch() {
  if (launching.value) return;
  const next = !prefs.auto_launch;
  launching.value = true;
  prefs.auto_launch = next;
  const err = await setAutoLaunch(next);
  launching.value = false;
  if (err) {
    // 回滚：setAutoLaunch 内部已按 OS 实际状态回填，但失败时还需还原
    prefs.auto_launch = !next;
    showToast(err, "error", 3500);
    return;
  }
  showToast(
    next ? t("msg_auto_launch_on") : t("msg_auto_launch_off"),
    "success",
    1500,
  );
}

async function onToggleSilentStart() {
  if (savingSilent.value || !launchDependent.value) return;
  const next = !prefs.silent_start;
  savingSilent.value = true;
  prefs.silent_start = next;
  const err = await savePrefs();
  savingSilent.value = false;
  if (err) {
    prefs.silent_start = !next;
    showToast(t("msg_save_failed", { err }), "error", 3500);
    return;
  }
  showToast(
    next ? t("msg_silent_start_on") : t("msg_silent_start_off"),
    "success",
    1500,
  );
}

async function onToggleAutoConnect() {
  if (savingAutoConnect.value || !launchDependent.value) return;
  const next = !prefs.auto_connect;
  savingAutoConnect.value = true;
  prefs.auto_connect = next;
  const err = await savePrefs();
  savingAutoConnect.value = false;
  if (err) {
    prefs.auto_connect = !next;
    showToast(t("msg_save_failed", { err }), "error", 3500);
    return;
  }
  showToast(
    next ? t("msg_auto_connect_on") : t("msg_auto_connect_off"),
    "success",
    1500,
  );
}

function autoLaunchDesc(): string {
  return prefs.auto_launch
    ? t("launch_auto_launch_on_desc")
    : t("launch_auto_launch_off_desc");
}

function silentStartDesc(): string {
  if (!prefs.auto_launch) {
    return t("launch_blocked_dependency");
  }
  return prefs.silent_start
    ? t("launch_silent_start_on_desc")
    : t("launch_silent_start_off_desc");
}

function autoConnectDesc(): string {
  if (!prefs.auto_launch) {
    return t("launch_blocked_dependency");
  }
  return prefs.auto_connect
    ? t("launch_auto_connect_on_desc")
    : t("launch_auto_connect_off_desc");
}
</script>

<template>
  <div class="tab-pane">
    <!-- 启动 -->
    <section class="card section-card">
      <div class="section-title">{{ $t("launch_section") }}</div>

      <div class="pref-row">
        <div class="pref-header">
          <div class="pref-title">{{ $t("launch_auto_launch") }}</div>
          <button
            type="button"
            class="toggle"
            role="switch"
            :aria-checked="prefs.auto_launch"
            :disabled="launching"
            :class="{ on: prefs.auto_launch }"
            @click="onToggleAutoLaunch"
          >
            <span class="toggle-knob" />
          </button>
        </div>
        <div class="pref-desc">{{ autoLaunchDesc() }}</div>
      </div>

      <div class="pref-divider" />

      <div class="pref-row" :class="{ disabled: !launchDependent }">
        <div class="pref-header">
          <div class="pref-title">{{ $t("launch_silent_start") }}</div>
          <button
            type="button"
            class="toggle"
            role="switch"
            :aria-checked="prefs.silent_start"
            :disabled="!launchDependent || savingSilent"
            :class="{ on: launchDependent && prefs.silent_start }"
            @click="onToggleSilentStart"
          >
            <span class="toggle-knob" />
          </button>
        </div>
        <div class="pref-desc">{{ silentStartDesc() }}</div>
      </div>

      <div class="pref-divider" />

      <div class="pref-row" :class="{ disabled: !launchDependent }">
        <div class="pref-header">
          <div class="pref-title">{{ $t("launch_auto_connect") }}</div>
          <button
            type="button"
            class="toggle"
            role="switch"
            :aria-checked="prefs.auto_connect"
            :disabled="!launchDependent || savingAutoConnect"
            :class="{ on: launchDependent && prefs.auto_connect }"
            @click="onToggleAutoConnect"
          >
            <span class="toggle-knob" />
          </button>
        </div>
        <div class="pref-desc">{{ autoConnectDesc() }}</div>
      </div>
    </section>

    <!-- 定时连接 -->
    <ScheduleSection />

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
.pref-row.disabled {
  opacity: 0.5;
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
</style>
