<script setup lang="ts">
import { reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Trash2 } from "@lucide/vue";
import { config } from "../../state";
import { saveConfig } from "../../commands/config";
import type { ProxyConfig } from "../../types";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";

const { t: $t } = useI18n();
const { toast, showToast } = useToast();

/** 代理规则数量上限；达到后隐藏「添加代理」按钮。 */
const MAX_PROXIES = 5;

const form = reactive({
  proxies: (config.proxies.length
    ? config.proxies.map((p) => ({ ...p }))
    : [{ name: "", type: "tcp", local_ip: "127.0.0.1", local_port: 80, remote_port: 0 }]
  ) as ProxyConfig[],
});

const saving = ref(false);

function onlyNumber(e: KeyboardEvent) {
  if (e.ctrlKey || e.metaKey) return;
  const allowed = ['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'Tab', 'Enter'];
  if (!/^\d$/.test(e.key) && !allowed.includes(e.key)) {
    e.preventDefault();
  }
}

function addProxy() {
  form.proxies.push({
    name: "",
    type: "tcp",
    local_ip: "127.0.0.1",
    local_port: 80,
    remote_port: 0,
  });
}

function removeProxy(index: number) {
  form.proxies.splice(index, 1);
}

function validate(): string | null {
  if (form.proxies.length === 0) return $t("proxy_err_min");
  for (let i = 0; i < form.proxies.length; i++) {
    const p = form.proxies[i];
    if (!p.name.trim() || !p.local_ip.trim()) return $t("proxy_err_incomplete", { n: i + 1 });
    if (p.local_port <= 0 || p.remote_port <= 0) return $t("proxy_err_port", { n: i + 1 });
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
  config.proxies = form.proxies.map((p) => ({
    name: p.name.trim(),
    type: p.type,
    local_ip: p.local_ip.trim(),
    local_port: Number(p.local_port),
    remote_port: Number(p.remote_port),
  }));
  const e = await saveConfig();
  saving.value = false;
  if (e) showToast($t("msg_save_failed", { err: e }), "error", 4000);
  else showToast($t("msg_save_success"), "success", 1200);
}
</script>

<template>
  <div class="tab-pane">
    <section class="card section-card">
      <div class="section-title">{{ $t("proxy_section_title") }}</div>
      <div v-for="(p, i) in form.proxies" :key="i" class="proxy-card">
        <div class="proxy-head">
          <span class="proxy-idx">#{{ i + 1 }}</span>
          <button class="btn btn-ghost btn-sm btn-icon" @click="removeProxy(i)" :title="$t('proxy_remove')" :aria-label="$t('proxy_remove')">
            <Trash2 :size="15" style="color:hsl(var(--destructive))" />
          </button>
        </div>
        <div class="proxy-grid">
          <label class="form-item">
            <span class="label">{{ $t("proxy_label_type") }}</span>
            <select class="input proxy-type" v-model="p.type">
              <option value="tcp">TCP</option>
              <option value="udp">UDP</option>
              <option value="http">HTTP</option>
              <option value="https">HTTPS</option>
            </select>
          </label>
          <label class="form-item span-3">
            <span class="label">{{ $t("proxy_label_name") }}</span>
            <input class="input" v-model="p.name" :placeholder="$t('proxy_ph_name')" />
          </label>
          <label class="form-item span-2">
            <span class="label">{{ $t("proxy_label_local_ip") }}</span>
            <input class="input" v-model="p.local_ip" :placeholder="$t('proxy_ph_local_ip')" />
          </label>
          <label class="form-item">
            <span class="label">{{ $t("proxy_label_local_port") }}</span>
            <input class="input" v-model.number="p.local_port" type="number" min="1" max="65535" :placeholder="$t('proxy_ph_local_port')" @keydown="onlyNumber" />
          </label>
          <label class="form-item">
            <span class="label">{{ $t("proxy_label_remote_port") }}</span>
            <input class="input" v-model.number="p.remote_port" type="number" min="1" max="65535" :placeholder="$t('proxy_ph_remote_port')" @keydown="onlyNumber" />
          </label>
        </div>
      </div>
      <div class="proxy-add-row">
        <button
          v-if="form.proxies.length < MAX_PROXIES"
          class="btn btn-outline btn-sm"
          @click="addProxy"
        >{{ $t("proxy_add") }}</button>
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
.form-item {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.form-item.span-2 {
  grid-column: span 2;
}
.form-item.span-3 {
  grid-column: span 3;
}
.label {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 500;
}
.proxy-card {
  background-color: hsl(var(--muted) / 0.5);
  border: 1px solid hsl(var(--border));
  border-radius: calc(var(--radius) - 2px);
  padding: 10px;
  margin-bottom: 8px;
}
.proxy-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 10px;
}
.proxy-idx {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 600;
  min-width: 24px;
}
.proxy-type {
  appearance: none;
  padding-right: 28px;
  background-image: linear-gradient(45deg, transparent 50%, hsl(var(--muted-foreground)) 50%),
    linear-gradient(135deg, hsl(var(--muted-foreground)) 50%, transparent 50%);
  background-position: calc(100% - 14px) 50%, calc(100% - 9px) 50%;
  background-size: 5px 5px;
  background-repeat: no-repeat;
}
.proxy-grid {
  display: grid;
  grid-template-columns: 1.4fr 1fr 1fr 1fr;
  gap: 8px;
}
.proxy-add-row {
  display: flex;
  justify-content: flex-start;
  margin-top: 4px;
}
.tab-footer {
  display: flex;
  justify-content: flex-end;
  padding: 12px 0 0;
}
</style>
