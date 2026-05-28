<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { X, Check, Eye, EyeOff, Sparkles, Search, Link2 } from "@lucide/vue";
import { useProvidersStore } from "../stores/providers";
import type { ProviderView } from "../tauri";

const store = useProvidersStore();

type Tab = "claude" | "universal";
const tab = ref<Tab>("claude");
const filter = ref("");
const revealKey = ref(false);
const saving = ref(false);
const localErr = ref<string | null>(null);

// 选中的预设 id（"" = 自定义配置）
const selectedId = ref<string>("");

const form = reactive({
  id: undefined as string | undefined,
  name: "",
  note: "",
  websiteUrl: "",
  tokenField: "ANTHROPIC_AUTH_TOKEN",
  baseUrl: "",
  apiKey: "",
  model: "",
  fullUrl: false,
  configJson: '{\n  "env": {}\n}',
});

// 预设列表（排除自定义占位，自定义单独首位渲染）
const presets = computed(() => store.providers.filter((p) => p.isPreset));
const filteredPresets = computed(() => {
  const q = filter.value.trim().toLowerCase();
  if (!q) return presets.value;
  return presets.value.filter(
    (p) => p.name.toLowerCase().includes(q) || p.baseUrl.toLowerCase().includes(q)
  );
});

onMounted(() => {
  if (!store.providers.length) store.refresh();
  initFromTarget();
  window.addEventListener("keydown", onEsc);
});
onBeforeUnmount(() => window.removeEventListener("keydown", onEsc));
function onEsc(e: KeyboardEvent) {
  if (e.key === "Escape") store.closeAdd();
}

// 进入时若带 target（点预设/编辑）→ 预填
function initFromTarget() {
  const t = store.addTarget;
  if (t) {
    applyProvider(t);
  } else {
    selectCustom();
  }
}
watch(
  () => store.addTarget,
  (t) => {
    if (t) applyProvider(t);
  }
);

function applyProvider(p: ProviderView) {
  selectedId.value = p.isPreset ? p.id : "";
  form.id = p.id;
  form.name = p.name;
  form.note = p.note || "";
  form.websiteUrl = p.websiteUrl || "";
  form.tokenField = p.tokenField || "ANTHROPIC_AUTH_TOKEN";
  form.baseUrl = p.baseUrl || "";
  form.apiKey = p.authToken || "";
  form.model = p.model || "";
  form.configJson = JSON.stringify(
    p.settingsConfig && typeof p.settingsConfig === "object"
      ? p.settingsConfig
      : { env: {} },
    null,
    2
  );
  localErr.value = null;
}

function selectCustom() {
  selectedId.value = "";
  form.id = undefined;
  form.name = "";
  form.note = "";
  form.websiteUrl = "";
  form.tokenField = "ANTHROPIC_AUTH_TOKEN";
  form.baseUrl = "";
  form.apiKey = "";
  form.model = "";
  form.configJson = '{\n  "env": {}\n}';
  localErr.value = null;
}

function pickPreset(p: ProviderView) {
  applyProvider(p);
}

const isOauth = computed(
  () => selectedId.value === "codex" || selectedId.value === "github-copilot"
);

// ── config JSON 同步 ──
function parseCfg(): any {
  try {
    return JSON.parse(form.configJson || "{}");
  } catch {
    return null;
  }
}
function writeCfg(cfg: any) {
  form.configJson = JSON.stringify(cfg, null, 2);
}
function setEnv(key: string, val: string) {
  const cfg = parseCfg() ?? { env: {} };
  if (!cfg.env || typeof cfg.env !== "object") cfg.env = {};
  if (val) cfg.env[key] = val;
  else delete cfg.env[key];
  writeCfg(cfg);
}

function onApiKey() {
  setEnv(form.tokenField, form.apiKey.trim());
}
function onBaseUrl() {
  setEnv("ANTHROPIC_BASE_URL", form.baseUrl.trim());
}
function onTokenFieldSwitch(field: string) {
  // 切换字段：把旧字段的值搬到新字段
  const cfg = parseCfg() ?? { env: {} };
  if (!cfg.env) cfg.env = {};
  delete cfg.env["ANTHROPIC_AUTH_TOKEN"];
  delete cfg.env["ANTHROPIC_API_KEY"];
  if (form.apiKey.trim()) cfg.env[field] = form.apiKey.trim();
  form.tokenField = field;
  writeCfg(cfg);
}

function formatJson() {
  const cfg = parseCfg();
  if (cfg) writeCfg(cfg);
  else localErr.value = "JSON 格式有误，无法格式化";
}

// ── 5 个开关 ──
const toggles = computed(() => {
  const c = parseCfg() ?? {};
  const env = c.env ?? {};
  return {
    hideAttribution: c?.attribution?.commit === "" && c?.attribution?.pr === "",
    teammates: env.CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS === "1",
    enableToolSearch: env.ENABLE_TOOL_SEARCH === "true" || env.ENABLE_TOOL_SEARCH === "1",
    effortMax: env.CLAUDE_CODE_EFFORT_LEVEL === "max",
    disableAutoUpgrade: env.DISABLE_AUTOUPDATER === "1",
  };
});
function toggle(key: string, checked: boolean) {
  const cfg = parseCfg() ?? { env: {} };
  if (!cfg.env || typeof cfg.env !== "object") cfg.env = {};
  switch (key) {
    case "hideAttribution":
      if (checked) cfg.attribution = { commit: "", pr: "" };
      else delete cfg.attribution;
      break;
    case "teammates":
      if (checked) cfg.env.CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS = "1";
      else delete cfg.env.CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS;
      break;
    case "enableToolSearch":
      if (checked) cfg.env.ENABLE_TOOL_SEARCH = "true";
      else delete cfg.env.ENABLE_TOOL_SEARCH;
      break;
    case "effortMax":
      if (checked) cfg.env.CLAUDE_CODE_EFFORT_LEVEL = "max";
      else delete cfg.env.CLAUDE_CODE_EFFORT_LEVEL;
      break;
    case "disableAutoUpgrade":
      if (checked) cfg.env.DISABLE_AUTOUPDATER = "1";
      else delete cfg.env.DISABLE_AUTOUPDATER;
      break;
  }
  writeCfg(cfg);
}
const toggleDefs = [
  { key: "hideAttribution", label: "隐藏 AI 署名" },
  { key: "teammates", label: "Teammates 模式" },
  { key: "enableToolSearch", label: "启用 Tool Search" },
  { key: "effortMax", label: "最大强度思考" },
  { key: "disableAutoUpgrade", label: "禁用自动升级" },
] as const;

async function submit() {
  localErr.value = null;
  if (!form.name.trim()) {
    localErr.value = "请填写供应商名称";
    return;
  }
  const cfg = parseCfg();
  if (cfg === null) {
    localErr.value = "配置 JSON 格式有误";
    return;
  }
  saving.value = true;
  const id = await store.save({
    id: form.id,
    name: form.name,
    note: form.note,
    websiteUrl: form.websiteUrl,
    tokenField: form.tokenField,
    model: form.model.trim(),
    settingsConfig: cfg,
  });
  saving.value = false;
  if (id) {
    // 有 key（或官方）→ 立即启用
    const hasToken = !!(cfg.env && (cfg.env.ANTHROPIC_AUTH_TOKEN || cfg.env.ANTHROPIC_API_KEY));
    if (hasToken || selectedId.value === "claude-official") {
      await store.switchTo(id);
    }
    store.closeAdd();
  } else {
    localErr.value = store.error;
  }
}

function dotColor(p: ProviderView) {
  return p.color || "#e8833a";
}
</script>

<template>
  <Teleport to="body">
    <div class="modal-overlay" @click="store.closeAdd()">
      <div class="modal" @click.stop>
        <div class="m-accent" />
        <header class="m-head">
          <div class="m-title">{{ form.id && !selectedId ? "编辑供应商" : "添加新供应商" }}</div>
          <button class="icon-btn" @click="store.closeAdd()"><X :size="17" :stroke-width="1.8" /></button>
        </header>

        <!-- tabs -->
        <div class="m-tabs">
          <button class="m-tab" :class="{ on: tab === 'claude' }" @click="tab = 'claude'">
            Claude 供应商
          </button>
          <button class="m-tab" :class="{ on: tab === 'universal' }" @click="tab = 'universal'">
            统一供应商
          </button>
        </div>

        <div class="m-body">
          <p v-if="tab === 'universal'" class="universal-note">
            轻量版用 <b>pi</b> 内核直连各家 API（Anthropic Messages 协议）。填好端点、Key 与模型即可在多供应商间切换。
          </p>

          <!-- 预设网格 -->
          <div class="grid-head">
            <span class="sec-title">预设供应商</span>
            <div class="grid-search">
              <Search :size="12" :stroke-width="1.8" />
              <input v-model="filter" placeholder="搜索…" />
            </div>
          </div>
          <div class="preset-grid">
            <button
              class="chip"
              :class="{ on: selectedId === '' && !form.id }"
              @click="selectCustom"
            >
              <span class="chip-plus">+</span> 自定义配置
            </button>
            <button
              v-for="p in filteredPresets"
              :key="p.id"
              class="chip"
              :class="{ on: selectedId === p.id }"
              @click="pickPreset(p)"
            >
              <span class="chip-dot" :style="{ background: dotColor(p) }" />
              {{ p.name }}
            </button>
          </div>

          <!-- OAuth 提示 -->
          <div v-if="isOauth" class="oauth-note">
            <b>{{ form.name }}</b> 说 OpenAI 协议，pi 轻量版未内置其 OAuth 授权路由。建议改用兼容 Anthropic Messages 协议的端点接入。
          </div>

          <!-- 表单 -->
          <template v-else>
            <div class="f-grid">
              <label class="field">
                <span class="f-lab">供应商名称</span>
                <input v-model="form.name" placeholder="例如：Claude 官方" />
              </label>
              <label class="field">
                <span class="f-lab">备注</span>
                <input v-model="form.note" placeholder="例如：公司专用账号" />
              </label>
            </div>

            <label class="field">
              <span class="f-lab">官网链接</span>
              <input v-model="form.websiteUrl" placeholder="https://example.com（可选）" />
            </label>

            <label class="field">
              <span class="f-lab">API Key</span>
              <div class="key-wrap">
                <input
                  v-model="form.apiKey"
                  :type="revealKey ? 'text' : 'password'"
                  placeholder="只需要填这里，下方配置会自动填充"
                  autocomplete="off"
                  @input="onApiKey"
                />
                <button class="icon-btn sm" @click="revealKey = !revealKey">
                  <component :is="revealKey ? EyeOff : Eye" :size="15" :stroke-width="1.8" />
                </button>
              </div>
              <div class="field-toggle">
                <button :class="{ sel: form.tokenField === 'ANTHROPIC_AUTH_TOKEN' }" @click="onTokenFieldSwitch('ANTHROPIC_AUTH_TOKEN')">AUTH_TOKEN</button>
                <button :class="{ sel: form.tokenField === 'ANTHROPIC_API_KEY' }" @click="onTokenFieldSwitch('ANTHROPIC_API_KEY')">API_KEY</button>
              </div>
            </label>

            <label class="field">
              <span class="f-lab row">
                请求地址
                <span class="url-toggle" :class="{ on: form.fullUrl }" @click="form.fullUrl = !form.fullUrl">
                  <Link2 :size="11" :stroke-width="2" /> 完整 URL
                  <span class="sw"><span class="knob" /></span>
                </span>
              </span>
              <input v-model="form.baseUrl" placeholder="https://your-api-endpoint.com" @input="onBaseUrl" />
              <p class="hint">💡 填写兼容 Anthropic Messages 协议的服务端点（cc-switch 端点即可），不要以斜杠结尾</p>
            </label>

            <label class="field">
              <span class="f-lab">模型 ID</span>
              <input v-model="form.model" placeholder="例如：claude-sonnet-4-5 / glm-4.6 / deepseek-chat" />
              <p class="hint">💡 pi 用 <code>--model 供应商/模型</code> 选模型。填该供应商 API 接受的模型名（留空用预设默认）。</p>
            </label>

            <!-- 配置 JSON + 开关 -->
            <div class="cfg-head">
              <span class="f-lab">配置 JSON</span>
              <button class="fmt-btn" @click="formatJson"><Sparkles :size="12" :stroke-width="1.8" /> 格式化</button>
            </div>
            <div class="toggles">
              <label v-for="t in toggleDefs" :key="t.key" class="tg">
                <input
                  type="checkbox"
                  :checked="(toggles as any)[t.key]"
                  @change="toggle(t.key, ($event.target as HTMLInputElement).checked)"
                />
                <span>{{ t.label }}</span>
              </label>
            </div>
            <textarea v-model="form.configJson" class="json-editor" spellcheck="false" rows="6" />
          </template>

          <div v-if="localErr" class="err">{{ localErr }}</div>
        </div>

        <footer class="m-foot">
          <button class="btn-cancel" @click="store.closeAdd()">取消</button>
          <button class="btn-add" :disabled="saving || isOauth" @click="submit">
            <Check :size="14" :stroke-width="2.4" /> {{ saving ? "保存中…" : "添加" }}
          </button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 400;
  background: rgba(20, 20, 25, 0.28);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  animation: ov 160ms ease;
}
@keyframes ov {
  from { opacity: 0; }
}
.modal {
  width: min(740px, 96vw);
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 16px;
  box-shadow: var(--shadow-lg), 0 0 0 1px var(--hairline);
  overflow: hidden;
  animation: pop 200ms cubic-bezier(0.16, 1, 0.3, 1);
}
@keyframes pop {
  from { opacity: 0; transform: translateY(12px) scale(0.98); }
}
.m-accent {
  height: 3px;
  background: linear-gradient(90deg, var(--primary) 0%, var(--gold) 55%, var(--vermilion) 100%);
}
.m-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 15px 18px 12px;
}
.m-title {
  font-family: var(--serif);
  font-size: 17px;
  font-weight: 600;
  color: var(--ink);
  letter-spacing: 1.5px;
}
.icon-btn {
  border: none;
  background: transparent;
  color: var(--muted);
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.icon-btn:hover { background: var(--selection-bg); color: var(--text); }
.icon-btn.sm { width: 26px; height: 26px; }

.m-tabs {
  display: flex;
  gap: 4px;
  padding: 0 18px;
  border-bottom: 1px solid var(--border-soft);
}
.m-tab {
  padding: 9px 16px;
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 13px;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
}
.m-tab.on {
  color: var(--primary-deep);
  font-weight: 600;
  border-bottom-color: var(--primary);
}

.m-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 14px 18px 4px;
}
.universal-note {
  margin: 0 0 12px;
  font-size: 11.5px;
  color: var(--text-2);
  background: var(--primary-soft);
  border-radius: 8px;
  padding: 8px 11px;
  line-height: 1.6;
}

.grid-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 9px;
}
.sec-title {
  font-family: var(--serif);
  font-size: 11.5px;
  letter-spacing: 1.5px;
  color: var(--dim);
}
.grid-search {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 9px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--bg-soft);
  color: var(--muted);
}
.grid-search:focus-within { border-color: var(--primary); }
.grid-search input {
  border: none;
  background: transparent;
  font-size: 12px;
  width: 120px;
  color: var(--text);
}
.grid-search input:focus { outline: none; }

.preset-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  max-height: 168px;
  overflow-y: auto;
  padding: 2px 2px 4px;
  margin-bottom: 14px;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 11px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-soft);
  color: var(--text-2);
  font-size: 12px;
  transition: border-color 120ms ease, background 120ms ease, color 120ms ease;
}
.chip:hover { border-color: var(--border-strong); color: var(--text); }
.chip.on {
  border-color: var(--primary);
  background: var(--primary-soft);
  color: var(--primary-deep);
  font-weight: 500;
}
.chip-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.chip-plus {
  font-size: 13px;
  color: var(--primary);
  font-weight: 700;
  line-height: 1;
}

.oauth-note {
  font-size: 12px;
  color: var(--text-2);
  background: #e8833a14;
  border: 1px solid #e8833a44;
  border-radius: 9px;
  padding: 12px 14px;
  line-height: 1.7;
  margin-bottom: 8px;
}

.f-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin-bottom: 12px;
}
.f-lab {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
}
.f-lab.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.field input,
.json-editor {
  width: 100%;
  padding: 9px 11px;
  border: 1px solid var(--border);
  border-radius: 9px;
  font-size: 13px;
  background: var(--bg-soft);
  color: var(--text);
}
.field input:focus,
.json-editor:focus {
  outline: none;
  border-color: var(--primary);
  background: var(--panel);
}
.key-wrap {
  display: flex;
  gap: 6px;
  align-items: center;
}
.key-wrap input { flex: 1; }
.field-toggle {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}
.field-toggle button {
  border: 1px solid var(--border);
  background: var(--panel);
  color: var(--muted);
  font-size: 10.5px;
  font-family: var(--mono);
  padding: 3px 9px;
  border-radius: 6px;
}
.field-toggle button.sel {
  border-color: var(--primary);
  background: var(--primary-soft);
  color: var(--primary-deep);
}
.url-toggle {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 400;
  color: var(--muted);
  cursor: pointer;
  user-select: none;
}
.url-toggle.on { color: var(--primary); }
.sw {
  width: 26px;
  height: 15px;
  border-radius: 8px;
  background: var(--border-strong);
  position: relative;
  transition: background 140ms ease;
}
.url-toggle.on .sw { background: var(--primary); }
.knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 11px;
  height: 11px;
  border-radius: 50%;
  background: #fff;
  transition: transform 140ms ease;
}
.url-toggle.on .knob { transform: translateX(11px); }
.hint {
  margin: 7px 0 0;
  font-size: 11px;
  color: var(--gold);
  background: #a78c4f12;
  border: 1px solid #a78c4f44;
  border-radius: 8px;
  padding: 7px 10px;
}

.cfg-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.fmt-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: 1px solid var(--border);
  background: var(--panel);
  color: var(--text-2);
  font-size: 11.5px;
  padding: 4px 10px;
  border-radius: 7px;
}
.fmt-btn:hover { border-color: var(--primary); color: var(--primary); }
.toggles {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  margin-bottom: 9px;
}
.tg {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-2);
  cursor: pointer;
}
.tg input { accent-color: var(--primary); width: 14px; height: 14px; }
.json-editor {
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.6;
  resize: vertical;
  min-height: 120px;
}
.err {
  margin: 10px 0 4px;
  font-size: 12px;
  color: var(--vermilion);
  background: var(--vermilion-soft);
  border-radius: 8px;
  padding: 8px 11px;
}

.m-foot {
  display: flex;
  justify-content: flex-end;
  gap: 9px;
  padding: 12px 18px;
  border-top: 1px solid var(--border-soft);
  background: var(--bg-soft);
}
.btn-cancel,
.btn-add {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--border);
  background: var(--panel);
  color: var(--text-2);
  font-size: 13px;
  padding: 8px 18px;
  border-radius: 9px;
}
.btn-cancel:hover { background: var(--selection-bg); }
.btn-add {
  background: var(--ink);
  color: #fff;
  border-color: var(--ink);
}
.btn-add:hover { background: var(--primary); border-color: var(--primary); }
.btn-add:disabled { opacity: 0.5; }
</style>
