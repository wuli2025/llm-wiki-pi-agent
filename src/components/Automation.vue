<script setup lang="ts">
import { computed, onMounted } from "vue";
import {
  Clock,
  Plus,
  Play,
  SquarePen,
  Trash2,
  Newspaper,
  BookMarked,
  Tv,
  Sparkles,
  Folder,
  Telescope,
  Repeat,
  LoaderCircle,
  X,
  CircleStop,
} from "@lucide/vue";
import { useAutomationStore, type AutomationFlow } from "../stores/automation";
import { useAppStore } from "../stores/app";
import { useChatStore } from "../stores/chat";

const auto = useAutomationStore();
const app = useAppStore();
const chat = useChatStore();

onMounted(() => {
  if (!app.projects.length) app.refreshProjects();
  auto.startScheduler();
});

const ICONS: Record<string, any> = {
  newspaper: Newspaper,
  "book-marked": BookMarked,
  tv: Tv,
  sparkles: Sparkles,
};
function iconOf(f: AutomationFlow) {
  return ICONS[f.icon] || Sparkles;
}

function scheduleLabel(f: AutomationFlow): string {
  const s = f.schedule;
  if (s.kind === "daily") return `每天 ${s.time}`;
  if (s.kind === "interval") return `每 ${s.everyHours} 小时`;
  return "手动触发";
}
function projectLabel(f: AutomationFlow): string {
  if (!f.projectId) return "当前项目";
  return app.projects.find((p) => p.id === f.projectId)?.name || "未知项目";
}
function running(f: AutomationFlow): boolean {
  return !!f.lastConvId && chat.isSending(f.lastConvId);
}

async function run(f: AutomationFlow) {
  await auto.runFlow(f);
}
function edit(f: AutomationFlow) {
  auto.openEdit(f);
}
function remove(f: AutomationFlow) {
  if (confirm(`删除自动化「${f.name}」？`)) auto.removeFlow(f.id);
}

// ── 缩小版对话框（运行进度）──
const activeBubbles = computed(() => chat.bubblesFor(auto.activeConvId));
const activeRunning = computed(
  () => !!auto.activeConvId && chat.isSending(auto.activeConvId)
);
function closePanel() {
  auto.activeConvId = null;
}
function stopRun() {
  if (auto.activeConvId) chat.cancel(auto.activeConvId);
}
</script>

<template>
  <div class="auto-wrap">
    <div class="auto-main" :class="{ 'with-panel': auto.activeConvId }">
      <!-- 头部 -->
      <header class="head">
        <div class="title-row">
          <Clock :size="20" :stroke-width="1.7" class="t-icon" />
          <h1>自动化</h1>
        </div>
        <p class="lead">
          把一段编排好的任务交给本机 pi 定时/循环跑：选方向 → 深度搜索 →
          仿知识库风格成稿 → 多维评审 → 落到草稿箱（不自动发布，由你过目后再发）。
        </p>
      </header>

      <!-- 流程卡片 -->
      <div class="grid">
        <button class="card new" @click="auto.openCreate()">
          <span class="new-plus"><Plus :size="22" :stroke-width="1.8" /></span>
          <span class="new-text">新建自动化</span>
        </button>

        <div v-for="f in auto.flows" :key="f.id" class="card flow">
          <div class="c-head">
            <span class="c-icon" :style="{ background: f.color }">
              <component :is="iconOf(f)" :size="15" :stroke-width="1.8" color="#fff" />
            </span>
            <span class="c-name" :title="f.name">{{ f.name }}</span>
            <span v-if="running(f)" class="run-tag">
              <LoaderCircle :size="12" :stroke-width="2" class="spin" /> 运行中
            </span>
          </div>
          <p class="c-desc">{{ f.description || "（无描述）" }}</p>
          <div class="c-meta">
            <span class="meta"><Folder :size="12" :stroke-width="1.6" /> {{ projectLabel(f) }}</span>
            <span class="meta"><Clock :size="12" :stroke-width="1.6" /> {{ scheduleLabel(f) }}</span>
            <span v-if="f.deepResearch" class="meta"><Telescope :size="12" :stroke-width="1.6" /> 深度</span>
            <span v-if="f.loopCount > 1" class="meta"><Repeat :size="12" :stroke-width="1.6" /> ×{{ f.loopCount }}</span>
          </div>
          <div class="c-act">
            <button class="run-btn" :disabled="running(f)" @click="run(f)">
              <Play :size="13" :stroke-width="2" /> {{ running(f) ? "运行中…" : "运行" }}
            </button>
            <button class="mini-btn" title="编辑" @click="edit(f)">
              <SquarePen :size="14" :stroke-width="1.7" />
            </button>
            <button class="mini-btn danger" title="删除" @click="remove(f)">
              <Trash2 :size="14" :stroke-width="1.7" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 缩小版对话框：运行进度 -->
    <aside v-if="auto.activeConvId" class="run-panel">
      <div class="rp-head">
        <span class="rp-title">
          <component :is="activeRunning ? LoaderCircle : Sparkles" :size="14" :stroke-width="1.9" :class="{ spin: activeRunning }" />
          运行进度
        </span>
        <div class="rp-act">
          <button v-if="activeRunning" class="rp-stop" title="停止" @click="stopRun">
            <CircleStop :size="15" :stroke-width="1.8" />
          </button>
          <button class="rp-close" title="收起" @click="closePanel">
            <X :size="16" :stroke-width="1.7" />
          </button>
        </div>
      </div>
      <div class="rp-body">
        <div
          v-for="(b, i) in activeBubbles"
          :key="i"
          class="bubble"
          :class="b.role"
        >
          <template v-if="b.role === 'tool'">
            <span class="tool-pill">{{ b.text }}</span>
          </template>
          <template v-else>
            <div class="b-text">{{ b.text }}</div>
            <div v-if="b.artifacts && b.artifacts.length" class="b-arts">
              <span v-for="(a, j) in b.artifacts" :key="j" class="art-pill">
                📄 {{ a.split('/').pop() }}
              </span>
            </div>
          </template>
        </div>
        <div v-if="activeRunning" class="typing"><span></span><span></span><span></span></div>
        <div v-if="!activeBubbles.length && !activeRunning" class="rp-empty">
          运行后这里会实时显示进度。
        </div>
      </div>
    </aside>
  </div>
</template>

<style scoped>
.auto-wrap {
  flex: 1;
  display: flex;
  min-height: 0;
  background: var(--bg);
}
.auto-main {
  flex: 1;
  overflow-y: auto;
  padding: 34px 44px 60px;
  min-width: 0;
}

.head { margin-bottom: 24px; }
.title-row { display: flex; align-items: center; gap: 10px; }
.t-icon { color: var(--ink); }
.head h1 {
  font-family: var(--serif);
  font-size: 22px;
  font-weight: 600;
  letter-spacing: 3px;
  color: var(--ink);
  margin: 0;
}
.lead {
  margin: 10px 0 0;
  font-size: 13px;
  line-height: 1.9;
  color: var(--text-2);
  max-width: 720px;
  letter-spacing: 0.3px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 14px;
}
.card {
  border: 1px solid var(--border-soft);
  border-radius: 12px;
  background: var(--panel);
  padding: 16px;
  text-align: left;
  transition: border-color 0.15s, box-shadow 0.15s, transform 0.15s;
}
.card.flow:hover {
  border-color: var(--border);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.06);
  transform: translateY(-1px);
}

.card.new {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 160px;
  border-style: dashed;
  border-color: var(--border);
  color: var(--muted);
  cursor: pointer;
}
.card.new:hover { border-color: var(--ink); color: var(--ink); background: var(--selection-bg); }
.new-plus {
  width: 40px; height: 40px;
  border-radius: 50%;
  display: inline-flex; align-items: center; justify-content: center;
  background: var(--selection-bg);
}
.card.new:hover .new-plus { background: var(--ink); color: #fff; }
.new-text { font-size: 13px; letter-spacing: 1px; }

.c-head { display: flex; align-items: center; gap: 9px; }
.c-icon {
  width: 26px; height: 26px;
  border-radius: 8px;
  display: inline-flex; align-items: center; justify-content: center;
  flex-shrink: 0;
}
.c-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--ink);
  flex: 1;
  min-width: 0;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.run-tag {
  display: inline-flex; align-items: center; gap: 4px;
  font-size: 10.5px; color: var(--primary);
  background: var(--primary-soft);
  padding: 2px 7px; border-radius: 20px;
  flex-shrink: 0;
}
.c-desc {
  margin: 10px 0 12px;
  font-size: 12px;
  line-height: 1.7;
  color: var(--text-2);
  min-height: 40px;
}
.c-meta {
  display: flex; flex-wrap: wrap; gap: 6px;
  margin-bottom: 14px;
}
.meta {
  display: inline-flex; align-items: center; gap: 4px;
  font-size: 11px; color: var(--muted);
  background: var(--bg-soft, var(--selection-bg));
  padding: 3px 8px; border-radius: 6px;
}
.meta svg { color: var(--dim); }

.c-act { display: flex; align-items: center; gap: 8px; }
.run-btn {
  flex: 1;
  display: inline-flex; align-items: center; justify-content: center; gap: 5px;
  border: none;
  background: var(--ink); color: #fff;
  font-size: 12.5px; letter-spacing: 1px;
  padding: 7px 12px; border-radius: 8px;
  cursor: pointer;
}
.run-btn:hover:not(:disabled) { background: var(--primary); }
.run-btn:disabled { opacity: 0.55; cursor: not-allowed; }
.mini-btn {
  border: 1px solid var(--border);
  background: transparent; color: var(--muted);
  width: 30px; height: 30px;
  border-radius: 8px;
  display: inline-flex; align-items: center; justify-content: center;
  cursor: pointer;
}
.mini-btn:hover { border-color: var(--ink); color: var(--ink); }
.mini-btn.danger:hover { border-color: var(--vermilion); color: var(--vermilion); }

/* ── 缩小版对话框 ── */
.run-panel {
  width: 360px;
  flex-shrink: 0;
  border-left: 1px solid var(--border-soft);
  background: var(--bg-soft, var(--panel));
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.rp-head {
  display: flex; align-items: center; justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border-soft);
}
.rp-title {
  display: inline-flex; align-items: center; gap: 7px;
  font-size: 13px; font-weight: 600; color: var(--ink);
  font-family: var(--serif); letter-spacing: 1px;
}
.rp-act { display: flex; gap: 4px; }
.rp-stop, .rp-close {
  border: none; background: transparent; color: var(--muted);
  display: inline-flex; padding: 4px; border-radius: 6px; cursor: pointer;
}
.rp-stop:hover { color: var(--vermilion); background: var(--selection-bg); }
.rp-close:hover { color: var(--text); background: var(--selection-bg); }
.rp-body {
  flex: 1;
  overflow-y: auto;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.bubble { font-size: 12.5px; line-height: 1.7; }
.bubble.user .b-text {
  background: var(--ink); color: #fff;
  padding: 8px 11px; border-radius: 10px 10px 2px 10px;
  align-self: flex-end;
  white-space: pre-wrap; word-break: break-word;
}
.bubble.user { display: flex; justify-content: flex-end; }
.bubble.assistant .b-text {
  color: var(--text);
  white-space: pre-wrap; word-break: break-word;
}
.bubble.tool { }
.tool-pill {
  display: inline-block;
  font-size: 11px; color: var(--muted);
  background: var(--selection-bg);
  padding: 2px 8px; border-radius: 20px;
  font-family: var(--mono);
}
.b-arts { margin-top: 6px; display: flex; flex-wrap: wrap; gap: 5px; }
.art-pill {
  font-size: 11px; color: var(--primary-deep, var(--primary));
  background: var(--primary-soft);
  padding: 3px 8px; border-radius: 6px;
}
.rp-empty { font-size: 12px; color: var(--dim); font-style: italic; text-align: center; padding: 30px 0; }

.spin { animation: spin 0.9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.typing { display: flex; gap: 4px; padding: 4px 0; }
.typing span {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--muted);
  animation: blink 1.2s infinite both;
}
.typing span:nth-child(2) { animation-delay: 0.2s; }
.typing span:nth-child(3) { animation-delay: 0.4s; }
@keyframes blink { 0%, 80%, 100% { opacity: 0.25; } 40% { opacity: 1; } }
</style>
