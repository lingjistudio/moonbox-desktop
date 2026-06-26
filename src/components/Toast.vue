<script setup lang="ts">
import type { ToastType } from "../composables/useToast";

defineProps<{ toast: { msg: string; type: ToastType } | null }>();
</script>

<template>
  <Transition name="toast">
    <div v-if="toast" class="toast" :class="`toast-${toast.type}`" role="status">
      {{ toast.msg }}
    </div>
  </Transition>
</template>

<style scoped>
.toast {
  position: fixed;
  top: 48px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 1000;
  padding: 10px 18px;
  border-radius: var(--radius);
  font-size: 13px;
  font-weight: 500;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  pointer-events: none;
  max-width: 90%;
  text-align: center;
}
.toast-success {
  background-color: hsl(var(--success));
  color: hsl(var(--success-foreground));
}
.toast-error {
  background-color: hsl(var(--destructive));
  color: hsl(var(--destructive-foreground));
}
.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.25s ease, transform 0.25s ease;
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translate(-50%, -8px);
}
</style>
