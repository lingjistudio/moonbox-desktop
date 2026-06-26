<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { Settings } from "@lucide/vue";

import { config, isConfigured, frpcStatus } from "../state";
import { startFrpc, stopFrpc } from "../commands/frpc";
import CircleButton from "../components/home/CircleButton.vue";
import ProxyList from "../components/home/ProxyList.vue";
import GuideCard from "../components/home/GuideCard.vue";
import SystemStatus from "../components/home/SystemStatus.vue";

const { t: $t } = useI18n();
const emit = defineEmits<{ settings: [] }>();

const error = ref("");

async function onToggle() {
  error.value = "";
  const s = frpcStatus.value;
  if (s === "connecting" || s === "connected") {
    const err = await stopFrpc();
    if (err) error.value = err;
  } else {
    // stopped / error：先确保没有残留子进程，再启动
    if (s === "error") {
      await stopFrpc().catch(() => undefined);
    }
    const err = await startFrpc();
    if (err) error.value = err;
  }
}
</script>

<template>
  <div class="home-view">
    <button
      class="home-settings-btn"
      @click="emit('settings')"
      :title="$t('home_settings_title')"
      :aria-label="$t('home_settings_title')"
    >
      <Settings :size="18" />
    </button>
    <div class="home-body">
      <CircleButton :disabled="!isConfigured()" @click="onToggle" />
      <GuideCard v-if="!isConfigured()" @settings="emit('settings')" />
      <ProxyList
        :proxies="config.proxies"
        :server-addr="config.server_addr"
      />
      <div v-if="error" class="error-msg">{{ error }}</div>
    </div>
    <SystemStatus />
  </div>
</template>

<style scoped>
.home-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  position: relative;
}

/* 系统设置齿轮：浮在主页内容区右上角 */
.home-settings-btn {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 5;
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
.home-settings-btn:hover {
  background: hsl(var(--accent));
  color: hsl(var(--foreground));
}

.home-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  /* 滚动不传染外层；触底不弹跳 */
  overscroll-behavior: contain;
}

.error-msg {
  color: hsl(var(--destructive));
  font-size: 12px;
  padding: 8px 12px;
  background-color: hsl(var(--destructive) / 0.08);
  border-radius: calc(var(--radius) - 2px);
}
</style>
