<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import {
  X,
  Check,
  Eye,
  EyeOff,
  Sparkles,
  Search,
  Plus,
  Trash2,
  Terminal,
  Globe,
  Server,
} from "@lucide/vue";

const props = withDefaults(defineProps<{ inline?: boolean }>(), { inline: false });
const emit = defineEmits<{ (e: "close"): void }>();

// ── UI 状态 ──
type Tab = "preset" | "stdio" | "remote";
const tab = ref<Tab>("preset");
const filter = ref("");
const revealKey = ref(false);
const saving = ref(false);
const localErr = ref<string | null>(null);

// 选中的预设 id（"" = 自定义）
const selectedId = ref<string>("");

// 预设清单（纯前端假数据，方便试验切换）
type Preset = {
  id: string;
  name: string;
  desc: string;
  transport: "stdio" | "remote";
  command?: string;
  args?: string[];
  url?: string;
  envKey: string; // 主 key 字段名
  envKeyHint: string;
  websiteUrl?: string;
  color: string;
};
const presets: Preset[] = [
  {
    id: "minimax",
    name: "MiniMax(海螺视频)",
    desc: "官方 stdio MCP — uvx 一行启",
    transport: "stdio",
    command: "uvx",
    args: ["minimax-mcp"],
    envKey: "MINIMAX_API_KEY",
    envKeyHint: "sk-…",
    websiteUrl: "https://www.minimaxi.com/",
    color: "#7b5cff",
  },
  {
    id: "ark-jimeng",
    name: "火山方舟 · 即梦",
    desc: "Remote MCP,走方舟托管",
    transport: "remote",
    url: "https://ark.cn-beijing.volces.com/mcp/jimeng",
    envKey: "ARK_API_KEY",
    envKeyHint: "ak-…",
    websiteUrl: "https://www.volcengine.com/product/ark",
    color: "#e8833a",
  },
  {
    id: "bailian-tongyi",
    name: "百炼 · 通义万相",
    desc: "Remote MCP,阿里云百炼托管",
    transport: "remote",
    url: "https://bailian.aliyun.com/mcp/wanx",
    envKey: "DASHSCOPE_API_KEY",
    envKeyHint: "sk-…",
    websiteUrl: "https://bailian.console.aliyun.com/",
    color: "#2c4661",
  },
  {
    id: "github",
    name: "GitHub MCP",
    desc: "官方 stdio,代码/issue/PR 操作",
    transport: "stdio",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
    envKey: "GITHUB_PERSONAL_ACCESS_TOKEN",
    envKeyHint: "ghp_…",
    websiteUrl: "https://github.com/modelcontextprotocol/servers",
    color: "#24292f",
  },
  {
    id: "filesystem",
    name: "Filesystem",
    desc: "官方 stdio,沙箱文件读写",
    transport: "stdio",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem", "."],
    envKey: "",
    envKeyHint: "无需 key",
    color: "#a78c4f",
  },
];

const filteredPresets = computed(() => {
  const q = filter.value.trim().toLowerCase();
  if (!q) return presets;
  return presets.filter(
    (p) => p.name.toLowerCase().includes(q) || p.desc.toLowerCase().includes(q)
  );
});

// ── 表单 ──
type EnvRow = { key: string; value: string };
const form = reactive({
  name: "",
  note: "",
  websiteUrl: "",
  transport: "stdio" as "stdio" | "remote",
  command: "",
  argsRaw: "",
  url: "",
  apiKey: "",
  envKeyField: "MINIMAX_API_KEY",
  extraEnv: [] as EnvRow[],
  enabled: true,
});

function selectCustom() {
  selectedId.value = "";
  form.name = "";
  form.note = "";
  form.websiteUrl = "";
  form.transport = tab.value === "remote" ? "remote" : "stdio";
  form.command = "";
  form.argsRaw = "";
  form.url = "";
  form.apiKey = "";
  form.envKeyField = "API_KEY";
  form.extraEnv = [];
  form.enabled = true;
  localErr.value = null;
}

function pickPreset(p: Preset) {
  selectedId.value = p.id;
  form.name = p.name;
  form.note = p.desc;
  form.websiteUrl = p.websiteUrl || "";
  form.transport = p.transport;
  form.command = p.command || "";
  form.argsRaw = (p.args || []).join(" ");
  form.url = p.url || "";
  form.apiKey = "";
  form.envKeyField = p.envKey || "API_KEY";
  form.extraEnv = [];
  form.enabled = true;
  tab.value = p.transport === "remote" ? "remote" : "stdio";
  localErr.value = null;
}

function addEnvRow() {
  form.extraEnv.push({ key: "", value: "" });
}
function removeEnvRow(i: number) {
  form.extraEnv.splice(i, 1);
}

// 工具放行 token 预览(MCP 接入要进 allowed_tools 的形式)
const allowToken = computed(() => {
  const slug = (form.name || "server")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "") || "server";
  return `mcp__${slug}`;
});

// 实时合成 --mcp-config 风格 JSON 预览
const previewJson = computed(() => {
  const env: Record<string, string> = {};
  if (form.apiKey.trim() && form.envKeyField.trim()) {
    env[form.envKeyField.trim()] = form.apiKey.trim();
  }
  for (const row of form.extraEnv) {
    if (row.key.trim()) env[row.key.trim()] = row.value;
  }
  const slug = allowToken.value.replace(/^mcp__/, "");
  const server: Record<string, unknown> = form.transport === "stdio"
    ? {
        command: form.command || "uvx",
        args: form.argsRaw.trim() ? form.argsRaw.trim().split(/\s+/) : [],
        env,
      }
    : {
        url: form.url || "https://example.com/mcp",
        headers: form.apiKey.trim()
          ? { Authorization: `Bearer ${form.apiKey.trim()}` }
          : {},
      };
  return JSON.stringify({ mcpServers: { [slug]: server } }, null, 2);
});

// ── 关闭/键位 ──
onMounted(() => {
  selectCustom();
  window.addEventListener("keydown", onEsc);
});
onBeforeUnmount(() => window.removeEventListener("keydown", onEsc));
function onEsc(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}

function close() {
  emit("close");
}

async function submit() {
  localErr.value = null;
  if (!form.name.trim()) {
    localErr.value = "请填写 MCP 服务名称";
    return;
  }
  if (form.transport === "stdio" && !form.command.trim()) {
    localErr.value = "stdio 模式下请填写启动命令(如 uvx / npx)";
    return;
  }
  if (form.transport === "remote" && !form.url.trim()) {
    localErr.value = "Remote 模式下请填写服务 URL";
    return;
  }
  saving.value = true;
  // 原型:打印到控制台让你看效果,不连后端
  // eslint-disable-next-line no-console
  console.log("[McpConfigModal] 保存", {
    form: { ...form },
    allowToken: allowToken.value,
    mcpConfig: JSON.parse(previewJson.value),
  });
  await new Promise((r) => setTimeout(r, 220));
  saving.value = false;
  close();
}
</script>

<template>
  <Teleport to="body" :disabled="props.inline">
    <div
      :class="props.inline ? 'mcp-panel' : 'modal-overlay'"
      @click="props.inline ? undefined : close"
    >
      <div
        :class="props.inline ? 'mcp-inner' : 'modal'"
        @click.stop
      >
        <div class="m-accent" />
        <header class="m-head">
          <div class="m-title">
            <Server :size="15" :stroke-width="1.8" />
            {{ props.inline ? 'MCP 服务配置' : '添加 MCP 服务' }}
            <span class="m-sub">原型 — 仅本地预览,不连后端</span>
          </div>
          <button v-if="!props.inline" class="icon-btn" @click="close">
            <X :size="17" :stroke-width="1.8" />
          </button>
        </header>

        <!-- tabs -->
        <div class="m-tabs">
          <button class="m-tab" :class="{ on: tab === 'preset' }" @click="tab = 'preset'">
            预设
          </button>
          <button
            class="m-tab"
            :class="{ on: tab === 'stdio' }"
            @click="tab = 'stdio'; form.transport = 'stdio'"
          >
            <Terminal :size="12" :stroke-width="1.8" /> 本地 stdio
          </button>
          <button
            class="m-tab"
            :class="{ on: tab === 'remote' }"
            @click="tab = 'remote'; form.transport = 'remote'"
          >
            <Globe :size="12" :stroke-width="1.8" /> Remote HTTP
          </button>
        </div>

        <div class="m-body">
          <!-- 预设 chips -->
          <div class="grid-head">
            <span class="sec-title">预设 MCP 服务</span>
            <div class="grid-search">
              <Search :size="12" :stroke-width="1.8" />
              <input v-model="filter" placeholder="搜索…" />
            </div>
          </div>
          <div class="preset-grid">
            <button
              class="chip"
              :class="{ on: selectedId === '' }"
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
              <span class="chip-dot" :style="{ background: p.color }" />
              {{ p.name }}
              <span class="chip-tag">{{ p.transport === "stdio" ? "stdio" : "remote" }}</span>
            </button>
          </div>

          <!-- 表单 -->
          <div class="f-grid">
            <label class="field">
              <span class="f-lab">服务名称</span>
              <input v-model="form.name" placeholder="例如:MiniMax 海螺视频" />
            </label>
            <label class="field">
              <span class="f-lab">备注</span>
              <input v-model="form.note" placeholder="例如:用于视频生成" />
            </label>
          </div>

          <label class="field">
            <span class="f-lab">官网链接</span>
            <input v-model="form.websiteUrl" placeholder="https://...(可选)" />
          </label>

          <!-- stdio 字段 -->
          <template v-if="form.transport === 'stdio'">
            <div class="f-grid">
              <label class="field">
                <span class="f-lab">启动命令</span>
                <input v-model="form.command" placeholder="uvx / npx / python" />
              </label>
              <label class="field">
                <span class="f-lab">参数(空格分隔)</span>
                <input v-model="form.argsRaw" placeholder="minimax-mcp" />
              </label>
            </div>
          </template>

          <!-- remote 字段 -->
          <template v-else>
            <label class="field">
              <span class="f-lab">服务 URL</span>
              <input v-model="form.url" placeholder="https://ark.cn-beijing.volces.com/mcp/jimeng" />
            </label>
          </template>

          <!-- API Key -->
          <label class="field">
            <span class="f-lab">API Key</span>
            <div class="key-wrap">
              <input
                v-model="form.apiKey"
                :type="revealKey ? 'text' : 'password'"
                placeholder="此值会写入下方 env 块"
                autocomplete="off"
              />
              <button class="icon-btn sm" @click="revealKey = !revealKey">
                <component :is="revealKey ? EyeOff : Eye" :size="15" :stroke-width="1.8" />
              </button>
            </div>
            <div class="field-toggle">
              <span class="env-key-lab">env 字段名</span>
              <input v-model="form.envKeyField" class="env-key-input" placeholder="MINIMAX_API_KEY" />
            </div>
          </label>

          <!-- 额外 env -->
          <div class="cfg-head">
            <span class="f-lab">额外 env 变量</span>
            <button class="fmt-btn" @click="addEnvRow">
              <Plus :size="12" :stroke-width="2" /> 新增一行
            </button>
          </div>
          <div v-if="form.extraEnv.length === 0" class="env-empty">
            一般只填 API Key 就够,有特殊需求(代理/超时/区域)再加
          </div>
          <div v-for="(row, i) in form.extraEnv" :key="i" class="env-row">
            <input v-model="row.key" class="env-k" placeholder="KEY" />
            <input v-model="row.value" class="env-v" placeholder="value" />
            <button class="icon-btn sm" @click="removeEnvRow(i)">
              <Trash2 :size="14" :stroke-width="1.8" />
            </button>
          </div>

          <!-- 工具放行预览 -->
          <div class="allow-box">
            <div class="allow-head">
              <Sparkles :size="12" :stroke-width="1.8" />
              <span>对话调用时需放行的工具前缀</span>
            </div>
            <code class="allow-code">{{ allowToken }}__*</code>
            <p class="allow-hint">
              接入 chat.rs 时,把 <code>{{ allowToken }}__*</code> 加进 allowed_tools,否则 MCP 工具会被收窄拒掉。
            </p>
          </div>

          <!-- JSON 预览 -->
          <div class="cfg-head">
            <span class="f-lab">--mcp-config 预览(只读)</span>
          </div>
          <textarea class="json-editor" :value="previewJson" readonly spellcheck="false" rows="8" />

          <div v-if="localErr" class="err">{{ localErr }}</div>
        </div>

        <footer class="m-foot">
          <label class="enable-toggle">
            <input type="checkbox" v-model="form.enabled" />
            <span>保存后立即启用</span>
          </label>
          <div class="foot-actions">
            <button v-if="!props.inline" class="btn-cancel" @click="close">取消</button>
            <button class="btn-add" :disabled="saving" @click="submit">
              <Check :size="14" :stroke-width="2.4" /> {{ saving ? "保存中…" : "保存" }}
            </button>
          </div>
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

/* inline 模式：嵌入视图 / EnvDoctor */
.mcp-panel {
  flex: 1;
  overflow: auto;
  display: flex;
  flex-direction: column;
}
.mcp-inner {
  width: 100%;
  max-width: 740px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
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
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-family: var(--serif);
  font-size: 17px;
  font-weight: 600;
  color: var(--ink);
  letter-spacing: 1.5px;
}
.m-sub {
  font-family: var(--sans, inherit);
  font-size: 11px;
  font-weight: 400;
  letter-spacing: 0;
  color: var(--muted);
  margin-left: 6px;
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
  display: inline-flex;
  align-items: center;
  gap: 5px;
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
  max-height: 144px;
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
.chip-tag {
  font-family: var(--mono);
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--panel);
  color: var(--muted);
  border: 1px solid var(--border-soft);
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
  align-items: center;
  gap: 8px;
  margin-top: 6px;
}
.env-key-lab {
  font-size: 11px;
  color: var(--muted);
  white-space: nowrap;
}
.env-key-input {
  font-family: var(--mono);
  font-size: 11.5px !important;
  padding: 5px 9px !important;
  width: 220px;
}

.cfg-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 6px 0 8px;
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

.env-empty {
  font-size: 11.5px;
  color: var(--muted);
  background: var(--bg-soft);
  border: 1px dashed var(--border);
  border-radius: 8px;
  padding: 8px 11px;
  margin-bottom: 10px;
}
.env-row {
  display: flex;
  gap: 6px;
  margin-bottom: 6px;
}
.env-k, .env-v {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  font-size: 12px;
  font-family: var(--mono);
  background: var(--bg-soft);
  color: var(--text);
}
.env-k { width: 200px; }
.env-v { flex: 1; }
.env-k:focus, .env-v:focus {
  outline: none;
  border-color: var(--primary);
  background: var(--panel);
}

.allow-box {
  margin: 14px 0 10px;
  padding: 11px 13px;
  background: var(--primary-soft);
  border: 1px solid var(--primary);
  border-radius: 10px;
}
.allow-head {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  font-weight: 600;
  color: var(--primary-deep);
  margin-bottom: 6px;
}
.allow-code {
  display: inline-block;
  font-family: var(--mono);
  font-size: 13px;
  color: var(--ink);
  background: var(--panel);
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
}
.allow-hint {
  margin: 7px 0 0;
  font-size: 11px;
  color: var(--text-2);
  line-height: 1.65;
}
.allow-hint code {
  font-family: var(--mono);
  font-size: 11px;
  color: var(--primary-deep);
}

.json-editor {
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.6;
  resize: vertical;
  min-height: 140px;
  cursor: text;
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
  align-items: center;
  justify-content: space-between;
  padding: 12px 18px;
  border-top: 1px solid var(--border-soft);
  background: var(--bg-soft);
}
.enable-toggle {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-2);
  cursor: pointer;
}
.enable-toggle input { accent-color: var(--primary); width: 14px; height: 14px; }
.foot-actions {
  display: flex;
  gap: 9px;
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
