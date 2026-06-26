<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Eye, EyeOff } from "@lucide/vue";
import { builtinProviders, config } from "../../state";
import { saveConfig } from "../../commands/config";
import type { Provider } from "../../types";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";

const { t: $t } = useI18n();
const { toast, showToast } = useToast();

const CUSTOM_ID = "custom";

/** 当前可选服务商列表（内置 + 自定义） */
const providers = computed<Provider[]>(() => [
  ...builtinProviders.value,
  {
    id: CUSTOM_ID,
    name: (config.custom_name ?? "").trim() || $t("provider_custom_fallback"),
    builtin: false,
    server_addr: config.server_addr,
    server_port: config.server_port,
    user: config.user,
    username_required: true,
  },
]);

/** 推断初始选中的服务商：已保存的优先；地址命中内置则用该内置；默认落到第一个内置 */
function resolveInitialId(): string {
  if (config.provider_id) {
    if (builtinProviders.value.some((p) => p.id === config.provider_id)) return config.provider_id;
    if (config.provider_id === CUSTOM_ID) return CUSTOM_ID;
  }
  const hit = builtinProviders.value.find(
    (p) => p.server_addr === config.server_addr && p.server_port === config.server_port,
  );
  return hit ? hit.id : (builtinProviders.value[0]?.id ?? CUSTOM_ID);
}

const form = reactive({
  provider_id: resolveInitialId(),
  custom_name: config.custom_name ?? "",
  server_addr: config.server_addr,
  server_port: config.server_port,
  token: config.token ?? "",
  user: config.user ?? "",
});

const saving = ref(false);
const showPassword = ref(false);
function togglePassword() {
  showPassword.value = !showPassword.value;
}

/** 当前选中的服务商对象（内置只读，自定义可编辑） */
const currentProvider = computed<Provider | undefined>(() =>
  providers.value.find((p) => p.id === form.provider_id),
);

const isBuiltin = computed(() => currentProvider.value?.builtin === true);
const isCustom = computed(() => form.provider_id === CUSTOM_ID);
const isUsernameRequired = computed(() => currentProvider.value?.username_required === true);

/** 切换到内置服务商时，把地址/端口同步到表单（只读显示） */
watch(
  () => form.provider_id,
  (id) => {
    const p = providers.value.find((x) => x.id === id);
    if (!p) return;
    form.server_addr = p.server_addr;
    form.server_port = p.server_port;
    form.user = p.user ?? "";
  },
  { immediate: true },
);

function onlyNumber(e: KeyboardEvent) {
  if (e.ctrlKey || e.metaKey) return;
  const allowed = ["Backspace", "Delete", "ArrowLeft", "ArrowRight", "Tab", "Enter"];
  if (!/^\d$/.test(e.key) && !allowed.includes(e.key)) {
    e.preventDefault();
  }
}

function validate(): string | null {
  if (isCustom.value) {
    if (!form.custom_name.trim()) return $t("provider_err_custom_name");
    if (!form.server_addr.trim()) return $t("provider_err_server_addr");
    if (!form.server_port || form.server_port <= 0) return $t("provider_err_server_port");
  } else {
    if (!form.server_addr.trim()) return $t("provider_err_server_addr");
    if (!form.server_port || form.server_port <= 0) return $t("provider_err_server_port");
    if (isUsernameRequired.value && !form.user.trim()) return $t("provider_err_user");
  }
  return null;
}

async function onSave() {
  const err = validate();
  if (err) {
    showToast(err, "error");
    return;
  }
  saving.value = true;
  config.provider_id = form.provider_id === CUSTOM_ID ? CUSTOM_ID : form.provider_id;
  config.custom_name = isCustom.value ? form.custom_name.trim() : "";
  config.server_addr = form.server_addr.trim();
  config.server_port = Number(form.server_port);
  config.token = form.token.trim();
  config.user = form.user.trim();
  const e = await saveConfig();
  saving.value = false;
  if (e) showToast($t("msg_save_failed", { err: e }), "error", 4000);
  else showToast($t("msg_save_success"), "success", 1200);
}
</script>

<template>
  <div class="tab-pane">
    <section class="card section-card">
      <div class="section-title">{{ $t("provider_section_title") }}</div>
      <div class="form-grid">
        <div class="provider-row">
          <label class="form-item provider-select">
            <span class="label">{{ $t("provider_label") }}</span>
            <select class="input select" v-model="form.provider_id">
              <option
                v-for="p in providers"
                :key="p.id"
                :value="p.id"
              >
                {{ p.name }}
              </option>
            </select>
          </label>
          <label v-if="isCustom" class="form-item provider-name">
            <span class="label">{{ $t("provider_label_custom_name") }}</span>
            <input
              class="input"
              v-model="form.custom_name"
              :placeholder="$t('provider_ph_custom_name')"
              maxlength="32"
            />
          </label>
        </div>

        <label class="form-item">
          <span class="label">{{ $t("provider_label_server_addr") }}</span>
          <input
            class="input"
            v-model="form.server_addr"
            :placeholder="$t('provider_ph_server_addr')"
            :readonly="isBuiltin"
            :disabled="isBuiltin"
            :class="{ readonly: isBuiltin }"
          />
        </label>
        <label class="form-item">
          <span class="label">{{ $t("provider_label_server_port") }}</span>
          <input
            class="input"
            v-model.number="form.server_port"
            type="number"
            min="1"
            max="65535"
            :readonly="isBuiltin"
            :disabled="isBuiltin"
            :class="{ readonly: isBuiltin }"
            @keydown="onlyNumber"
          />
        </label>
        <label v-if="isCustom || isUsernameRequired" class="form-item">
          <span class="label">{{ $t("provider_label_user") }}</span>
          <input
            class="input"
            v-model="form.user"
            :placeholder="isUsernameRequired ? $t('provider_ph_user_required') : $t('provider_ph_user_optional')"
          />
        </label>
        <div class="form-item span-2">
          <span class="label">{{ $t("provider_label_token") }}</span>
          <div class="password-field">
            <input
              class="input password-input"
              v-model="form.token"
              :type="showPassword ? 'text' : 'password'"
              :placeholder="$t('provider_ph_token')"
              autocomplete="off"
            />
            <button
              type="button"
              class="password-toggle"
              :aria-label="showPassword ? $t('provider_hide_password') : $t('provider_show_password')"
              :title="showPassword ? $t('provider_hide_password') : $t('provider_show_password')"
              @click="togglePassword"
            >
              <EyeOff v-if="showPassword" :size="16" aria-hidden="true" />
              <Eye v-else :size="16" aria-hidden="true" />
            </button>
          </div>
        </div>
      </div>
    </section>

    <footer class="tab-footer">
      <button class="btn btn-primary" @click="onSave" :disabled="saving">
        {{ saving ? $t("common_saving") : $t("common_save") }}
      </button>
    </footer>

    <Toast :toast="toast" />
  </div>
</template>

<style scoped>
.tab-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.section-card {
  padding: 14px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 12px;
}
.form-grid {
  display: grid;
  grid-template-columns: 3fr 1fr;
  gap: 10px;
}
.provider-row {
  grid-column: span 2;
  display: flex;
  gap: 10px;
}
.provider-select {
  flex: 1;
  min-width: 0;
}
.provider-name {
  flex: 2;
  min-width: 0;
}
.form-item {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.form-item.span-2 {
  grid-column: span 2;
}
.label {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 500;
}
.input.readonly,
.input:read-only,
.input:disabled {
  background: hsl(var(--muted));
  color: hsl(var(--muted-foreground));
  cursor: not-allowed;
}
.password-field {
  position: relative;
  display: flex;
  align-items: center;
}
.password-input {
  padding-right: 32px;
}
.password-toggle {
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border: none;
  background: transparent;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  border-radius: calc(var(--radius) - 2px);
  transition: color 0.15s;
}
.password-toggle:hover {
  color: hsl(var(--foreground));
}
.password-toggle:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px hsl(var(--ring) / 0.12);
}
.select {
  appearance: none;
  -webkit-appearance: none;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'><path d='M3 4.5l3 3 3-3' fill='none' stroke='%23999' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/></svg>");
  background-repeat: no-repeat;
  background-position: right 10px center;
  padding-right: 28px;
  cursor: pointer;
}
.tab-footer {
  display: flex;
  justify-content: flex-end;
  padding: 12px 0 0;
}
</style>