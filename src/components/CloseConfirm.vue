<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { exit } from "@tauri-apps/plugin-process";

const { t } = useI18n();

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [value: boolean] }>();

function close() {
  emit("update:modelValue", false);
}

async function minimize() {
  close();
  await getCurrentWindow().hide();
}

async function exitApp() {
  close();
  await exit(0);
}

function onKeydown(e: KeyboardEvent) {
  if (!props.modelValue) return;
  if (e.key === "Escape") {
    e.preventDefault();
    close();
  } else if (e.key === "Enter") {
    e.preventDefault();
    minimize();
  }
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onUnmounted(() => window.removeEventListener("keydown", onKeydown));
</script>

<template>
  <Teleport to="body">
    <div
      v-if="modelValue"
      class="close-mask"
      @click.self="close"
    >
      <div
        class="close-card"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="close-title"
        aria-describedby="close-body"
      >
        <button
          class="close-x"
          @click="close"
          :aria-label="t('common_cancel')"
          :title="t('common_cancel')"
        >
          ×
        </button>
        <div id="close-title" class="close-title">
          <span class="close-icon" aria-hidden="true">ⓘ</span>
          <span>{{ t("close_confirm_title") }}</span>
        </div>
        <p id="close-body" class="close-body">
          {{ t("close_confirm_body") }}
        </p>
        <div class="close-actions">
          <button class="btn btn-outline" @click="minimize" autofocus>{{ t("close_confirm_minimize") }}</button>
          <button class="btn btn-destructive" @click="exitApp">{{ t("close_confirm_exit") }}</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.close-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.close-card {
  position: relative;
  width: 320px;
  max-width: calc(100vw - 32px);
  background: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: var(--radius);
  padding: 20px 20px 16px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.2);
}
.close-x {
  position: absolute;
  top: 6px;
  right: 6px;
  background: transparent;
  border: none;
  color: hsl(var(--muted-foreground));
  font-size: 18px;
  line-height: 1;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: default;
}
.close-x:hover {
  background: hsl(var(--accent));
}
.close-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 10px;
  padding-right: 24px;
}
.close-icon {
  font-size: 16px;
  color: hsl(var(--primary));
}
.close-body {
  font-size: 12px;
  line-height: 1.6;
  color: hsl(var(--muted-foreground));
  margin: 0 0 16px;
}
.close-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>
