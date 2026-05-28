<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch, nextTick } from "vue";
import {
  Zap,
  ChevronUp,
  X,
  Plus,
  Check,
  RefreshCw,
  Pencil,
  Trash2,
  ExternalLink,
  Search,
  LogIn,
  ShieldCheck,
  BarChart3,
} from "@lucide/vue";
import { useProvidersStore } from "../stores/providers";
import type { ProviderView, TokenBucket } from "../tauri";

const props = defineProps<{ collapsed?: boolean }>();
const store = useProvidersStore();

const open = ref(false);
const filter = ref("");

// Codex 授权
const codexOpen = ref(false);
let codexTimer: number | null = null;

// 用量周期
type Period = "today" | "week" | "month" | "year";
const period = ref<Period>("today");
const periods: { key: Period; label: string }[] = [
  { key: "today", label: "今日" },
  { key: "week", label: "近 7 天" },
  { key: "month", label: "近 30 天" },
  { key: "year", label: "近 1 年" },
];

onMounted(() => {
  store.refresh();
  store.refreshUsage();
});

watch(open, (v) => {
  if (v) {
    store.refresh();
    store.refreshUsage();
    nextTick(() => window.addEventListener("keydown", onEsc));
  } else {
    codexOpen.value = false;
    stopCodexPoll();
    window.removeEventListener("keydown", onEsc);
  }
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onEsc);
  stopCodexPoll();
});
function onEsc(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (codexOpen.value) codexOpen.value = false;
  else open.value = false;
}

function fmt(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(2) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(2) + "M";
  if (n >= 1e4) return (n / 1e3).toFixed(0) + "K";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "K";
  return String(n);
}
function fmtCost(n: number): string {
  return "$" + n.toFixed(4);
}
function hostOf(url: string): string {
  if (!url) return "本地 / 订阅";
  try {
    return new URL(url).host;
  } catch {
    return url.replace(/^https?:\/\//, "");
  }
}

const current = computed(() => store.current);
const todayTotal = computed(() => store.usage?.today.total ?? 0);

const filtered = computed(() => {
  const q = filter.value.trim().toLowerCase();
  if (!q) return store.providers;
  return store.providers.filter(
    (p) =>
      p.name.toLowerCase().includes(q) ||
      p.baseUrl.toLowerCase().includes(q) ||
      p.id.toLowerCase().includes(q)
  );
});

function bucketOf(p: Period): TokenBucket | null {
  return store.usage ? store.usage[p] : null;
}
const activeBucket = computed(() => bucketOf(period.value));

async function onRowClick(p: ProviderView) {
  if (p.kind === "codex") {
    codexOpen.value = true;
    store.refreshCodex();
    return;
  }
  if (p.kind === "copilot") {
    store.openAdd(p);
    open.value = false;
    return;
  }
  if (p.id === store.currentId) return;
  if (!p.hasKey) {
    store.openAdd(p);
    open.value = false;
    return;
  }
  await store.switchTo(p.id);
}

function editProvider(p: ProviderView) {
  store.openAdd(p);
  open.value = false;
}
function addCustom() {
  store.openAdd(null);
  open.value = false;
}
function openBoard() {
  store.openUsage();
  open.value = false;
}

async function removeProvider(p: ProviderView) {
  const verb = p.isPreset ? "清除配置" : "删除";
  if (!confirm(`${verb}「${p.name}」?`)) return;
  await store.remove(p.id);
}

function openSite(url: string) {
  if (url) window.open(url, "_blank");
}

// Codex 授权
async function doCodexLogin() {
  const ok = await store.codexLogin();
  if (ok) startCodexPoll();
}
function startCodexPoll() {
  stopCodexPoll();
  let n = 0;
  codexTimer = window.setInterval(async () => {
    n++;
    await store.refreshCodex();
    if (store.codex?.loggedIn || n > 40) stopCodexPoll();
  }, 2000);
}
function stopCodexPoll() {
  if (codexTimer !== null) {
    clearInterval(codexTimer);
    codexTimer = null;
  }
}
</script>

<template>
  <div class="dock-root">
    <!-- resting 药丸 -->
    <button
      class="pill"
      :class="{ rail: props.collapsed, active: open }"
      :title="current ? `当前: ${current.name}` : 'API 供应商'"
      @click="open = !open"
    >
      <span
        class="dot"
        :style="{
          background: current?.color || 'var(--primary)',
          boxShadow: `0 0 0 3px ${(current?.color || '#2c4661')}1f`,
        }"
      />
      <template v-if="!props.collapsed">
        <span class="pill-main">
          <span class="pill-name">{{ current?.name || "选择供应商" }}</span>
          <span class="pill-sub">
            <Zap :size="9" :stroke-width="2.4" />
            {{ fmt(todayTotal) }} · 今日
          </span>
        </span>
        <ChevronUp class="chev" :class="{ flip: open }" :size="14" :stroke-width="2" />
      </template>
    </button>

    <Teleport to="body">
      <Transition name="dock-fade">
        <div v-if="open" class="dock-overlay" @click="open = false">
          <div class="panel" @click.stop>
            <div class="panel-accent" />

            <header class="panel-head">
              <div class="head-titles">
                <div class="title">API 供应商</div>
                <div class="subtitle">点选即切换 · 写入 ~/.pi/agent/models.json</div>
              </div>
              <div class="head-actions">
                <button class="icon-btn" title="添加供应商" @click="addCustom">
                  <Plus :size="16" :stroke-width="2" />
                </button>
                <button class="icon-btn" title="关闭" @click="open = false">
                  <X :size="15" :stroke-width="1.8" />
                </button>
              </div>
            </header>

            <div class="panel-body">
              <div class="search-row">
                <Search :size="13" :stroke-width="1.8" class="s-ic" />
                <input v-model="filter" class="search-input" placeholder="搜索供应商…" />
                <button v-if="filter" class="icon-btn sm" @click="filter = ''">
                  <X :size="13" :stroke-width="1.8" />
                </button>
              </div>

              <div class="prov-list">
                <div
                  v-for="p in filtered"
                  :key="p.id"
                  class="prov-row"
                  :class="{ on: p.id === store.currentId, pending: store.switching === p.id }"
                  @click="onRowClick(p)"
                >
                  <span class="row-bar" v-if="p.id === store.currentId" />
                  <span class="prov-dot" :style="{ background: p.color }" />
                  <span class="prov-info">
                    <span class="prov-name">{{ p.name }}</span>
                    <span class="prov-host">{{
                      p.kind === "codex"
                        ? "ChatGPT · 需授权"
                        : p.kind === "copilot"
                        ? "需 OAuth · 代理"
                        : hostOf(p.baseUrl)
                    }}</span>
                  </span>

                  <span class="prov-tail">
                    <span v-if="store.switching === p.id" class="spinner" />
                    <span v-else-if="p.id === store.currentId" class="badge-on">
                      <Check :size="11" :stroke-width="2.6" /> 使用中
                    </span>
                    <span v-else-if="p.kind === 'codex' || p.kind === 'copilot'" class="badge-oauth">授权</span>
                    <span v-else-if="!p.hasKey" class="badge-need">配置</span>

                    <span class="row-actions">
                      <button v-if="p.websiteUrl" class="mini-act" title="官网" @click.stop="openSite(p.websiteUrl)">
                        <ExternalLink :size="12" :stroke-width="1.8" />
                      </button>
                      <button
                        v-if="p.kind !== 'codex' && p.kind !== 'copilot'"
                        class="mini-act"
                        :title="p.isPreset ? '配置' : '编辑'"
                        @click.stop="editProvider(p)"
                      >
                        <Pencil :size="12" :stroke-width="1.8" />
                      </button>
                      <button
                        v-if="(p.isPreset && p.hasKey && p.kind === 'key') || p.kind === 'custom'"
                        class="mini-act danger"
                        :title="p.isPreset ? '清除配置' : '删除'"
                        @click.stop="removeProvider(p)"
                      >
                        <Trash2 :size="12" :stroke-width="1.8" />
                      </button>
                    </span>
                  </span>
                </div>

                <div v-if="filtered.length === 0" class="list-empty">无匹配供应商</div>

                <button class="add-row" @click="addCustom">
                  <Plus :size="13" :stroke-width="2.2" /> 添加供应商
                </button>
              </div>

              <!-- Codex 授权面板 -->
              <Transition name="ed-fade">
                <div v-if="codexOpen" class="editor codex">
                  <div class="ed-title">Codex (ChatGPT) 授权</div>
                  <template v-if="!store.codex || store.codex.installed === false">
                    <p class="codex-note">未检测到 <code>codex</code> CLI。安装后即可用 ChatGPT 账号授权:</p>
                    <code class="codex-cmd">npm i -g @openai/codex</code>
                    <div class="ed-actions">
                      <button class="ed-cancel" @click="codexOpen = false">关闭</button>
                      <button class="ed-save" @click="store.refreshCodex()">重新检测</button>
                    </div>
                  </template>
                  <template v-else-if="store.codex.loggedIn">
                    <p class="codex-ok"><ShieldCheck :size="14" :stroke-width="2" /> 已授权 ChatGPT</p>
                    <p class="codex-note">可在终端用 <code>codex</code> 直接对话。让 pi 直接路由到 Codex 需 Anthropic↔OpenAI 翻译代理(轻量版未内置)。</p>
                    <div class="ed-actions">
                      <button class="ed-cancel" @click="codexOpen = false">关闭</button>
                    </div>
                  </template>
                  <template v-else>
                    <p class="codex-note">已检测到 codex CLI,但尚未授权。点击下方按钮将打开浏览器完成 ChatGPT 登录。</p>
                    <div class="ed-actions">
                      <button class="ed-cancel" @click="codexOpen = false">关闭</button>
                      <button class="ed-save login" @click="doCodexLogin"><LogIn :size="13" :stroke-width="2" /> 授权登录</button>
                    </div>
                  </template>
                </div>
              </Transition>

              <div v-if="store.error" class="err-line">{{ store.error }}</div>

              <!-- 用量(紧凑) -->
              <section class="usage">
                <div class="usage-head">
                  <span class="u-title">Token 用量</span>
                  <div class="u-actions">
                    <button class="ghost" title="完整统计" @click="openBoard"><BarChart3 :size="12" :stroke-width="1.8" /> 详细</button>
                    <button class="icon-btn sm" title="刷新" @click="store.refreshUsage()"><RefreshCw :size="12" :stroke-width="1.8" /></button>
                  </div>
                </div>

                <template v-if="store.usage?.available">
                  <div class="period-chips">
                    <button
                      v-for="pd in periods"
                      :key="pd.key"
                      class="chip"
                      :class="{ on: period === pd.key }"
                      @click="period = pd.key"
                    >
                      <span class="chip-lab">{{ pd.label }}</span>
                      <span class="chip-num">{{ fmt(bucketOf(pd.key)?.total || 0) }}</span>
                    </button>
                  </div>
                  <div v-if="activeBucket" class="mini-foot">
                    <span>成本估算 <b>{{ fmtCost(activeBucket.cost) }}</b></span>
                    <span>输入 {{ fmt(activeBucket.input) }} · 输出 {{ fmt(activeBucket.output) }}</span>
                    <span>{{ activeBucket.requests }} 次</span>
                  </div>
                </template>
                <div v-else class="usage-empty">
                  暂无用量数据<br /><span>(尚未通过 pi 产生会话)</span>
                </div>
              </section>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.dock-root { width: 100%; }

.pill {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 7px 9px;
  background: linear-gradient(180deg, var(--panel) 0%, var(--bg-soft) 100%);
  border: 1px solid var(--border-soft);
  border-radius: 9px;
  text-align: left;
  transition: border-color 140ms ease, box-shadow 140ms ease;
  box-shadow: var(--shadow-sm);
}
.pill:hover { border-color: var(--border-strong); box-shadow: var(--shadow); }
.pill.active { border-color: var(--primary); box-shadow: 0 0 0 2px var(--primary-soft); }
.pill.rail { justify-content: center; padding: 8px 0; }
.dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; transition: box-shadow 200ms ease; }
.pill-main { flex: 1; display: flex; flex-direction: column; min-width: 0; gap: 1px; }
.pill-name { font-size: 12.5px; color: var(--text); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.pill-sub { font-size: 10px; color: var(--muted); font-family: var(--mono); display: inline-flex; align-items: center; gap: 3px; }
.chev { color: var(--muted); transition: transform 200ms ease; }
.chev.flip { transform: rotate(180deg); }

.dock-overlay { position: fixed; inset: 0; z-index: 200; }
.panel {
  position: fixed;
  left: 12px;
  bottom: 54px;
  width: 384px;
  max-height: min(80vh, 720px);
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  box-shadow: var(--shadow-lg), 0 0 0 1px var(--hairline);
  overflow: hidden;
}
.panel-accent { height: 2px; background: linear-gradient(90deg, var(--primary) 0%, var(--gold) 55%, var(--vermilion) 100%); opacity: 0.85; }
.panel-head { display: flex; align-items: flex-start; justify-content: space-between; padding: 13px 12px 10px 14px; border-bottom: 1px solid var(--border-soft); }
.head-titles { display: flex; flex-direction: column; gap: 2px; }
.head-actions { display: flex; gap: 2px; }
.title { font-family: var(--serif); font-size: 14.5px; font-weight: 600; color: var(--ink); letter-spacing: 1.5px; }
.subtitle { font-size: 10px; color: var(--dim); font-family: var(--mono); }
.icon-btn { border: none; background: transparent; color: var(--muted); border-radius: 5px; width: 26px; height: 26px; display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; }
.icon-btn:hover { background: var(--selection-bg); color: var(--text); }
.icon-btn.sm { width: 22px; height: 22px; }
.panel-body { flex: 1; min-height: 0; overflow-y: auto; }

.search-row { display: flex; align-items: center; gap: 6px; margin: 9px 10px 2px; padding: 5px 9px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-soft); }
.search-row:focus-within { border-color: var(--primary); }
.s-ic { color: var(--muted); flex-shrink: 0; }
.search-input { flex: 1; border: none; background: transparent; font-size: 12px; color: var(--text); }
.search-input:focus { outline: none; }

.prov-list { padding: 6px; }
.prov-row { position: relative; display: flex; align-items: center; gap: 9px; padding: 8px 9px; border-radius: 8px; cursor: pointer; transition: background 120ms ease; }
.prov-row:hover { background: var(--selection-bg); }
.prov-row.on { background: var(--primary-soft); }
.prov-row.pending { opacity: 0.6; }
.row-bar { position: absolute; left: 0; top: 7px; bottom: 7px; width: 2.5px; border-radius: 2px; background: var(--primary); }
.prov-dot { width: 9px; height: 9px; border-radius: 50%; flex-shrink: 0; }
.prov-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
.prov-name { font-size: 12.5px; color: var(--text); font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.prov-host { font-size: 10px; color: var(--muted); font-family: var(--mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.prov-tail { display: flex; align-items: center; gap: 4px; flex-shrink: 0; }
.badge-on { display: inline-flex; align-items: center; gap: 3px; font-size: 10px; color: var(--primary-deep); font-weight: 600; }
.badge-need { font-size: 9.5px; color: var(--gold); border: 1px solid var(--gold); border-radius: 4px; padding: 1px 5px; opacity: 0.85; }
.badge-oauth { font-size: 9.5px; color: #10a37f; border: 1px solid #10a37f; border-radius: 4px; padding: 1px 5px; }
.row-actions { display: none; align-items: center; gap: 2px; }
.prov-row:hover .row-actions { display: inline-flex; }
.mini-act { border: none; background: transparent; color: var(--muted); width: 22px; height: 22px; border-radius: 5px; display: inline-flex; align-items: center; justify-content: center; }
.mini-act:hover { background: var(--border); color: var(--text); }
.mini-act.danger:hover { background: var(--vermilion-soft); color: var(--vermilion); }
.spinner { width: 12px; height: 12px; border: 2px solid var(--border); border-top-color: var(--primary); border-radius: 50%; animation: spin 0.7s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.list-empty { text-align: center; font-size: 11.5px; color: var(--dim); padding: 12px 0; }
.add-row { width: 100%; display: flex; align-items: center; justify-content: center; gap: 5px; padding: 8px; margin-top: 2px; border: 1px dashed var(--border-strong); border-radius: 8px; background: transparent; color: var(--muted); font-size: 12px; }
.add-row:hover { border-color: var(--primary); color: var(--primary); background: var(--primary-soft); }

.editor { margin: 0 10px 9px; padding: 11px; border: 1px solid var(--border); border-radius: 9px; background: var(--bg-soft); display: flex; flex-direction: column; gap: 7px; }
.editor.codex { border-color: #10a37f55; background: #10a37f0c; }
.ed-title { font-size: 11.5px; font-weight: 600; color: var(--text-2); font-family: var(--serif); letter-spacing: 0.5px; }
.ed-actions { display: flex; gap: 6px; justify-content: flex-end; margin-top: 1px; }
.ed-cancel, .ed-save { display: inline-flex; align-items: center; gap: 4px; border: 1px solid var(--border); background: var(--panel); color: var(--text-2); font-size: 11.5px; padding: 5px 12px; border-radius: 6px; }
.ed-cancel:hover { background: var(--selection-bg); }
.ed-save { background: var(--ink); color: #fff; border-color: var(--ink); }
.ed-save:hover { background: var(--primary); border-color: var(--primary); }
.ed-save.login { background: #10a37f; border-color: #10a37f; }
.ed-save.login:hover { background: #0d8a6c; }
.codex-note { margin: 0; font-size: 11px; color: var(--text-2); line-height: 1.6; }
.codex-note code, .codex-cmd { font-family: var(--mono); font-size: 10.5px; background: var(--code-bg); color: var(--code-text); padding: 1px 5px; border-radius: 4px; }
.codex-cmd { display: block; padding: 6px 8px; user-select: all; }
.codex-ok { margin: 0; display: inline-flex; align-items: center; gap: 5px; font-size: 12px; font-weight: 600; color: #10a37f; }
.err-line { margin: 0 14px 9px; font-size: 11px; color: var(--vermilion); background: var(--vermilion-soft); border-radius: 6px; padding: 6px 9px; }

.usage { border-top: 1px solid var(--border-soft); padding: 12px 14px 15px; }
.usage-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 10px; }
.u-title { font-family: var(--serif); font-size: 11px; letter-spacing: 1.5px; color: var(--dim); }
.u-actions { display: flex; align-items: center; gap: 4px; }
.ghost { display: inline-flex; align-items: center; gap: 4px; border: 1px solid var(--border); background: var(--panel); color: var(--text-2); font-size: 10.5px; padding: 3px 8px; border-radius: 6px; }
.ghost:hover { border-color: var(--primary); color: var(--primary); }
.period-chips { display: grid; grid-template-columns: repeat(4, 1fr); gap: 7px; margin-bottom: 10px; }
.chip { display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 8px 4px 7px; border: 1px solid var(--border-soft); border-radius: 9px; background: var(--bg-soft); transition: border-color 120ms ease, background 120ms ease; }
.chip:hover { border-color: var(--border-strong); }
.chip.on { border-color: var(--primary); background: var(--primary-soft); }
.chip-lab { font-size: 10px; color: var(--text-2); }
.chip-num { font-family: var(--mono); font-size: 13.5px; font-weight: 600; color: var(--primary-deep); letter-spacing: -0.3px; }
.chip.on .chip-lab { color: var(--primary-deep); }
.mini-foot { display: flex; flex-wrap: wrap; gap: 4px 12px; font-size: 10.5px; color: var(--muted); padding-top: 4px; }
.mini-foot b { color: var(--primary-deep); font-family: var(--mono); }
.usage-empty { text-align: center; font-size: 11.5px; color: var(--muted); padding: 16px 0; line-height: 1.7; }
.usage-empty span { font-size: 10px; color: var(--dim); }

.dock-fade-enter-active, .dock-fade-leave-active { transition: opacity 180ms ease; }
.dock-fade-enter-active .panel, .dock-fade-leave-active .panel { transition: transform 220ms cubic-bezier(0.16, 1, 0.3, 1), opacity 180ms ease; transform-origin: bottom left; }
.dock-fade-enter-from, .dock-fade-leave-to { opacity: 0; }
.dock-fade-enter-from .panel, .dock-fade-leave-to .panel { opacity: 0; transform: translateY(10px) scale(0.97); }
.ed-fade-enter-active, .ed-fade-leave-active { transition: opacity 160ms ease, transform 160ms ease; }
.ed-fade-enter-from, .ed-fade-leave-to { opacity: 0; transform: translateY(-4px); }
</style>
