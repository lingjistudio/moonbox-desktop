<script setup lang="ts">
import { onMounted, onUnmounted, watch } from "vue";
import { useI18n } from "vue-i18n";

import { frpcStatus, frpcError } from "../../state/runtime";
import { recentlyApplied, downloadedPending } from "../../composables/useFrpcUpdate";
import { appUpdateAvailable, appUpdatePending } from "../../composables/useAppUpdate";

defineEmits<{ install: [] }>();

const { t: $t } = useI18n();

/** 引擎升级成功横幅自动消失：最近一次启动时升级到新版本，8s 后自动收起 */
let appliedBannerTimer: ReturnType<typeof setTimeout> | null = null;

function dismissAppliedBanner() {
  recentlyApplied.value = null;
  if (appliedBannerTimer) {
    clearTimeout(appliedBannerTimer);
    appliedBannerTimer = null;
  }
}

function startAppliedTimer() {
  if (appliedBannerTimer) return;
  appliedBannerTimer = setTimeout(() => {
    recentlyApplied.value = null;
    appliedBannerTimer = null;
  }, 8000);
}

onMounted(() => {
  // 启动时若 initFrpcVersion 已经置好 recentlyApplied（async 完成早于本 onMounted
  // 的概率较低，但 watch 兜底；这里同步路径作为补位）
  if (recentlyApplied.value) startAppliedTimer();
});

onUnmounted(() => {
  if (appliedBannerTimer) clearTimeout(appliedBannerTimer);
});

// initFrpcVersion 异步完成后 recentlyApplied 才会有值——这里兜底启动 timer
watch(recentlyApplied, (v) => {
  if (v) startAppliedTimer();
});
</script>

<template>
  <!-- frpc 连接错误：顶部红色错误条 -->
  <div v-if="frpcStatus === 'error'" class="error-banner">
    <span>{{ frpcError || $t('banner_connect_failed') }}</span>
  </div>

  <!-- 软件本体：已下载待安装 → 强提示；否则仅展示有新版本（软提示） -->
  <div v-if="appUpdatePending" class="update-banner banner-app">
    <span>{{ $t('banner_app_downloaded', { version: appUpdateAvailable?.version }) }}</span>
    <button class="banner-btn" @click="$emit('install')">{{ $t('banner_app_install_btn') }}</button>
  </div>
  <div v-else-if="appUpdateAvailable" class="update-banner banner-app-soft">
    <span>{{ $t('banner_app_soft', { version: appUpdateAvailable.version }) }}</span>
  </div>

  <!-- 引擎：本次启动刚应用的新版本 → 8s 自动消失；否则仅展示已下载待应用 -->
  <div v-if="recentlyApplied" class="update-banner banner-applied">
    <span>{{ $t('banner_engine_applied', { version: recentlyApplied }) }}</span>
    <button class="banner-close" @click="dismissAppliedBanner">×</button>
  </div>
  <div v-else-if="downloadedPending" class="update-banner banner-pending">
    <span>{{ $t('banner_engine_pending', { version: downloadedPending }) }}</span>
  </div>
</template>

<style scoped>
.error-banner {
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 500;
  color: hsl(var(--destructive-foreground));
  background-color: hsl(var(--destructive));
  display: flex;
  align-items: center;
  gap: 6px;
}
.update-banner {
  padding: 6px 12px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.banner-applied {
  background-color: hsl(var(--primary) / 0.12);
  color: hsl(var(--primary));
  border-bottom: 1px solid hsl(var(--primary) / 0.25);
}
.banner-pending {
  background-color: hsl(var(--muted));
  color: hsl(var(--muted-foreground));
  border-bottom: 1px solid hsl(var(--border));
}
.banner-app {
  background-color: hsl(var(--primary));
  color: hsl(var(--primary-foreground));
  font-weight: 600;
}
.banner-app-soft {
  background-color: hsl(var(--destructive) / 0.12);
  color: hsl(var(--destructive));
  border-bottom: 1px solid hsl(var(--destructive) / 0.25);
  justify-content: center;
}
.banner-btn {
  background: hsl(var(--primary-foreground) / 0.2);
  color: inherit;
  border: 1px solid hsl(var(--primary-foreground) / 0.4);
  border-radius: calc(var(--radius) - 3px);
  padding: 2px 10px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s;
}
.banner-btn:hover {
  background: hsl(var(--primary-foreground) / 0.3);
}
.banner-close {
  background: transparent;
  border: none;
  color: inherit;
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  padding: 0 4px;
  opacity: 0.7;
}
.banner-close:hover {
  opacity: 1;
}
</style>
