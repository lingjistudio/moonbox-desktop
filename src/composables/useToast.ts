import { ref } from "vue";

export type ToastType = "success" | "error";

export function useToast() {
  const toast = ref<{ msg: string; type: ToastType } | null>(null);
  let timer: ReturnType<typeof setTimeout> | null = null;

  function showToast(msg: string, type: ToastType = "success", duration = 2500) {
    if (timer) clearTimeout(timer);
    toast.value = { msg, type };
    timer = setTimeout(() => {
      toast.value = null;
      timer = null;
    }, duration);
  }

  return { toast, showToast };
}
