<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { frpcStatus } from "../../state/runtime";
import type { FrpcStatus } from "../../types";
import { useParticles } from "../../composables/useParticles";

defineProps<{ disabled: boolean }>();
defineEmits<{ click: [] }>();

const { t: $t } = useI18n();

/** 圆形按钮的 CSS 类名后缀，与 styles.css 中 .toggle-* 对应 */
const STATUS_KEYS: Record<FrpcStatus, { label: string; hint: string; aria: string }> = {
  stopped:    { label: "home_btn_stopped",    hint: "home_btn_hint_stopped",    aria: "home_btn_aria_stopped" },
  connecting: { label: "home_btn_connecting", hint: "home_btn_hint_connecting", aria: "home_btn_aria_connecting" },
  connected:  { label: "home_btn_connected",  hint: "home_btn_hint_connected",  aria: "home_btn_aria_connected" },
  error:      { label: "home_btn_error",      hint: "home_btn_hint_error",      aria: "home_btn_aria_error" },
};

const buttonClass = computed(() => `toggle-${frpcStatus.value}`);
const statusLabel = computed(() => $t(STATUS_KEYS[frpcStatus.value].label));
const statusHint = computed(() => $t(STATUS_KEYS[frpcStatus.value].hint));
const toggleAria = computed(() => $t(STATUS_KEYS[frpcStatus.value].aria));

/** Canvas 波纹粒子系统（connected 中心扩散 / connecting 外圈波动） */
const { canvas: particleCanvas } = useParticles(frpcStatus);
</script>

<template>
  <section class="control-section">
    <div class="ripple-wrapper">
      <canvas ref="particleCanvas" class="particle-canvas"></canvas>
      <button
        class="big-toggle"
        :class="buttonClass"
        :aria-label="toggleAria"
        :disabled="disabled"
        @click="$emit('click')"
      >
        <svg
          class="toggle-icon"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 256 256"
          fill="none"
          width="46"
          height="46"
          aria-hidden="true"
        >
          <path d="M128.15 15.4738C133.798 15.4738 138.377 20.0527 138.377 25.7012L138.377 66.6096C138.377 72.2581 133.798 76.837 128.15 76.837C122.502 76.837 117.923 72.2581 117.923 66.6096L117.923 25.7012C117.923 20.0527 122.502 15.4738 128.15 15.4738Z" fill="currentColor"/>
          <path d="M128.15 179.042C133.798 179.042 138.377 183.621 138.377 189.269L138.377 230.177C138.377 235.826 133.798 240.405 128.15 240.405C122.502 240.405 117.923 235.826 117.923 230.177L117.923 189.269C117.923 183.621 122.502 179.042 128.15 179.042Z" fill="currentColor"/>
          <path d="M48.3858 48.0749C52.3796 44.0811 58.8551 44.0811 62.8494 48.0749L91.7923 77.0178C95.786 81.0121 95.786 87.4876 91.7923 91.4813C87.7985 95.4751 81.323 95.4751 77.3287 91.4813L48.3858 62.5385C44.3921 58.5442 44.3921 52.0687 48.3858 48.0749Z" fill="currentColor"/>
          <path d="M164.585 164.274C168.579 160.281 175.055 160.281 179.049 164.274L207.992 193.217C211.985 197.212 211.985 203.687 207.992 207.681C203.998 211.675 197.522 211.675 193.528 207.681L164.585 178.738C160.592 174.744 160.592 168.268 164.585 164.274Z" fill="currentColor"/>
          <path d="M15.7847 127.839C15.7847 122.191 20.3636 117.612 26.0121 117.612L66.9205 117.612C72.5689 117.612 77.1479 122.191 77.1479 127.839C77.1479 133.487 72.5689 138.066 66.9205 138.066L26.0121 138.066C20.3636 138.066 15.7847 133.487 15.7847 127.839Z" fill="currentColor"/>
          <path d="M179.353 127.839C179.353 122.191 183.932 117.612 189.58 117.612L230.488 117.612C236.137 117.612 240.716 122.191 240.716 127.839C240.716 133.487 236.137 138.066 230.488 138.066L189.58 138.066C183.932 138.066 179.353 133.487 179.353 127.839Z" fill="currentColor"/>
          <path d="M91.7923 164.274C95.786 168.268 95.786 174.744 91.7923 178.738L62.8494 207.681C58.8551 211.675 52.3796 211.675 48.3858 207.681C44.3921 203.687 44.3921 197.212 48.3858 193.217L77.3287 164.274C81.323 160.281 87.7985 160.281 91.7923 164.274Z" fill="currentColor"/>
          <path d="M207.992 48.0749C211.985 52.0687 211.985 58.5442 207.992 62.5385L179.049 91.4813C175.055 95.4751 168.579 95.4751 164.585 91.4813C160.592 87.4876 160.592 81.0121 164.585 77.0178L193.528 48.0749C197.522 44.0811 203.998 44.0811 207.992 48.0749Z" fill="currentColor"/>
          <ellipse cx="150.881" cy="128.095" rx="18.592" ry="18.485" fill="currentColor"/>
          <ellipse cx="105.522" cy="128.095" rx="18.592" ry="18.485" fill="currentColor"/>
        </svg>
        <span class="toggle-label">{{ statusLabel }}</span>
        <span class="toggle-hint">{{ statusHint }}</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.control-section {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 28px 0 12px;
  flex-shrink: 0;
}
.ripple-wrapper {
  position: relative;
  width: 360px;
  height: 360px;
  display: flex;
  align-items: center;
  justify-content: center;
}
/* Canvas 波纹覆盖整个 wrapper（360×360，留出按钮外足够扩散空间） */
.particle-canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.big-toggle {
  position: relative;
  z-index: 1;
  width: 160px;
  height: 160px;
  border-radius: 50%;
  border: none;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  cursor: pointer;
  transition: opacity 0.15s, filter 0.15s;
  font-weight: 600;
  color: hsl(var(--primary-foreground));
  font-family: inherit;
}
/* 三态 + 错误态：单色语义色，互不相同便于一眼区分 */
.toggle-stopped    { background: hsl(var(--muted-foreground)); }
.toggle-connecting { background: hsl(var(--warning)); }
.toggle-connected  { background: hsl(var(--success)); }
.toggle-error      { background: hsl(var(--destructive)); }

.big-toggle:hover:not(:disabled) {
  filter: brightness(1.08);
}
.big-toggle:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.toggle-icon {
  margin-bottom: 2px;
  color: inherit;
}
.toggle-label {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.5px;
}
.toggle-hint {
  font-size: 11px;
  font-weight: 500;
  opacity: 0.85;
  margin-top: 1px;
}
</style>
