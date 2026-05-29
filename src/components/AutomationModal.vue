<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  Monitor,
  Folder,
  Clock,
  Telescope,
  Repeat,
  X,
  Info,
  Sparkles,
  ChevronDown,
  Check,
  Eraser,
  FileText,
} from "@lucide/vue";
import { useAutomationStore, FLOW_COLORS, type FlowDraft } from "../stores/automation";
import { useAppStore } from "../stores/app";
import { useWorkflowsStore, assemblePack } from "../stores/workflows";

const auto = useAutomationStore();
const app = useAppStore();
const workflows = useWorkflowsStore();

const editing = computed(() => auto.editorTarget);

// ── 表单状态 ──
const form = reactive<FlowDraft>({
  id: undefined,
  name: "",
  icon: "sparkles",
  color: FLOW_COLORS[0],
  description: "",
  prompt: "",
  projectId: null,
  execEnv: "local",
  schedule: { kind: "manual", time: "09:00", everyHours: 6 },
  loopCount: 1,
  deepResearch: true,
});

onMounted(() => {
  const t = editing.value;
  if (t) {
    form.id = t.id;
    form.name = t.name;
    form.icon = t.icon;
    form.color = t.color;
    form.description = t.description;
    form.prompt = t.prompt;
    form.projectId = t.projectId;
    form.execEnv = t.execEnv;
    form.schedule = { time: "09:00", everyHours: 6, ...t.schedule };
    form.loopCount = t.loopCount;
    form.deepResearch = t.deepResearch;
  } else if (!app.projects.length) {
    app.refreshProjects();
  }
});

// ── 下拉菜单开合（互斥）──
type Menu = "" | "env" | "project" | "schedule" | "template";
const openMenu = ref<Menu>("");
function toggle(m: Menu) {
  openMenu.value = openMenu.value === m ? "" : m;
}
function closeMenu() {
  openMenu.value = "";
}

const projectName = computed(() => {
  const p = app.projects.find((x) => x.id === form.projectId);
  return p ? p.name : "选择项目";
});

const scheduleLabel = computed(() => {
  const s = form.schedule;
  if (s.kind === "daily") return `每天 ${s.time}`;
  if (s.kind === "interval") return `每 ${s.everyHours} 小时`;
  return "手动触发";
});

function pickProject(id: string | null) {
  form.projectId = id;
  closeMenu();
}
function pickEnv(e: "local" | "sandbox") {
  form.execEnv = e;
  closeMenu();
}

// 使用模板：把工作流包拼装文本填进提示词框
function applyTemplate(packId: string) {
  const p = workflows.packs.find((x) => x.id === packId);
  if (p) {
    const text = assemblePack(p);
    form.prompt = form.prompt.trim() ? `${form.prompt.trim()}\n\n${text}` : text;
    if (!form.name.trim()) form.name = p.name;
  }
  closeMenu();
}

function clearPrompt() {
  form.prompt = "";
}

const canSubmit = computed(() => form.prompt.trim().length > 0);
const runNow = ref(true); // 创建后立即运行一次

async function submit() {
  if (!canSubmit.value) return;
  const saved = auto.saveFlow({ ...form });
  auto.closeEditor();
  if (runNow.value && saved) {
    await auto.runFlow(saved);
  }
}
</script>

<template>
  <div class="overlay" @click.self="auto.closeEditor()">
    <div class="modal" @click="closeMenu">
      <!-- 头部 -->
      <header class="m-head">
        <span class="m-badge" :style="{ background: form.color }">
          <Sparkles :size="13" :stroke-width="1.9" color="#fff" />
        </span>
        <input
          v-model="form.name"
          class="m-title"
          placeholder="给这个自动化起个名字"
        />
        <div class="m-head-act">
          <button class="ghost-btn" title="清空提示词" @click.stop="clearPrompt">
            <Eraser :size="14" :stroke-width="1.7" /> 清除
          </button>
          <span class="info-dot" title="提示词里 __________ 处填你的方向/主题；运行会在所选项目下新建一条对话，成品落到草稿箱，不会自动发布。">
            <Info :size="15" :stroke-width="1.7" />
          </span>
          <div class="menu-anchor">
            <button class="line-btn" @click.stop="toggle('template')">
              <FileText :size="13" :stroke-width="1.7" /> 使用模板
            </button>
            <div v-if="openMenu === 'template'" class="dropdown wide" @click.stop>
              <div class="dd-head">从工作流包载入</div>
              <button
                v-for="p in workflows.packs"
                :key="p.id"
                class="dd-item"
                @click="applyTemplate(p.id)"
              >
                <span class="dd-dot" :style="{ background: p.color }"></span>
                <span class="dd-name">{{ p.name }}</span>
              </button>
              <div v-if="!workflows.packs.length" class="dd-empty">暂无工作流包</div>
            </div>
          </div>
          <button class="icon-x" title="关闭" @click="auto.closeEditor()">
            <X :size="17" :stroke-width="1.7" />
          </button>
        </div>
      </header>

      <!-- 提示词正文 -->
      <textarea
        v-model="form.prompt"
        class="m-prompt"
        placeholder="在这里编排这个自动化要执行的任务（提示词）。例如：选一个方向 → 深度搜最近资讯 → 仿知识库风格成文 → 多维评审 → 存草稿箱。"
        spellcheck="false"
      ></textarea>

      <input
        v-model="form.description"
        class="m-desc"
        placeholder="一句话描述（选填，显示在卡片上）"
      />

      <!-- 底部配置条 -->
      <footer class="m-foot">
        <div class="foot-controls">
          <!-- 执行环境 -->
          <div class="menu-anchor">
            <button class="chip" :class="{ on: openMenu === 'env' }" @click.stop="toggle('env')">
              <Monitor :size="14" :stroke-width="1.7" />
              {{ form.execEnv === "local" ? "本地" : "沙箱" }}
              <ChevronDown :size="13" :stroke-width="1.7" class="chev" />
            </button>
            <div v-if="openMenu === 'env'" class="dropdown" @click.stop>
              <button class="dd-item" @click="pickEnv('local')">
                <Check :size="13" :class="{ hide: form.execEnv !== 'local' }" /> 本地执行
              </button>
              <button class="dd-item" @click="pickEnv('sandbox')">
                <Check :size="13" :class="{ hide: form.execEnv !== 'sandbox' }" /> 沙箱执行
              </button>
            </div>
          </div>

          <!-- 项目 -->
          <div class="menu-anchor">
            <button class="chip" :class="{ on: openMenu === 'project' }" @click.stop="toggle('project')">
              <Folder :size="14" :stroke-width="1.7" />
              {{ projectName }}
              <ChevronDown :size="13" :stroke-width="1.7" class="chev" />
            </button>
            <div v-if="openMenu === 'project'" class="dropdown" @click.stop>
              <div class="dd-head">项目</div>
              <button class="dd-item" @click="pickProject(null)">
                <Check :size="13" :class="{ hide: form.projectId !== null }" /> 当前项目（运行时决定）
              </button>
              <button
                v-for="p in app.projects"
                :key="p.id"
                class="dd-item"
                @click="pickProject(p.id)"
              >
                <Folder :size="13" :stroke-width="1.6" />
                <span class="dd-name">{{ p.name }}</span>
              </button>
            </div>
          </div>

          <!-- 运行时机 -->
          <div class="menu-anchor">
            <button class="chip" :class="{ on: openMenu === 'schedule' }" @click.stop="toggle('schedule')">
              <Clock :size="14" :stroke-width="1.7" />
              {{ scheduleLabel }}
              <ChevronDown :size="13" :stroke-width="1.7" class="chev" />
            </button>
            <div v-if="openMenu === 'schedule'" class="dropdown sched" @click.stop>
              <label class="rad">
                <input type="radio" value="manual" v-model="form.schedule.kind" /> 手动触发
              </label>
              <label class="rad">
                <input type="radio" value="daily" v-model="form.schedule.kind" /> 每天
                <input
                  type="time"
                  v-model="form.schedule.time"
                  class="mini-time"
                  :disabled="form.schedule.kind !== 'daily'"
                />
              </label>
              <label class="rad">
                <input type="radio" value="interval" v-model="form.schedule.kind" /> 每
                <input
                  type="number"
                  min="1"
                  max="168"
                  v-model.number="form.schedule.everyHours"
                  class="mini-num"
                  :disabled="form.schedule.kind !== 'interval'"
                /> 小时
              </label>
              <div class="dd-note">定时仅在应用开启时生效（本地轻量调度）。</div>
            </div>
          </div>

          <!-- 深度检测 -->
          <button
            class="chip toggle"
            :class="{ active: form.deepResearch }"
            title="开启后联网深度搜索、多源交叉验证"
            @click.stop="form.deepResearch = !form.deepResearch"
          >
            <Telescope :size="14" :stroke-width="1.7" /> 深度检测
          </button>

          <!-- 循环次数 -->
          <div class="chip stepper" title="流程自我迭代轮数">
            <Repeat :size="14" :stroke-width="1.7" />
            循环
            <button class="step-b" @click.stop="form.loopCount = Math.max(1, form.loopCount - 1)">−</button>
            <span class="step-n">{{ form.loopCount }}</span>
            <button class="step-b" @click.stop="form.loopCount = Math.min(5, form.loopCount + 1)">+</button>
          </div>
        </div>

        <div class="foot-submit">
          <label class="run-now">
            <input type="checkbox" v-model="runNow" /> 创建后立即运行
          </label>
          <button class="btn-cancel" @click="auto.closeEditor()">取消</button>
          <button class="btn-create" :disabled="!canSubmit" @click="submit">
            {{ editing ? "保存" : "创建" }}
          </button>
        </div>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 9990;
  background: rgba(20, 28, 40, 0.34);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
}
.modal {
  width: 100%;
  max-width: 720px;
  background: var(--panel);
  border: 1px solid var(--hairline, var(--border));
  border-radius: 14px;
  box-shadow: 0 24px 70px rgba(0, 0, 0, 0.28), 0 4px 14px rgba(0, 0, 0, 0.1);
  padding: 16px 18px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  animation: mIn 0.16s cubic-bezier(0.2, 0.7, 0.2, 1);
}
@keyframes mIn {
  from { opacity: 0; transform: translateY(10px) scale(0.985); }
  to { opacity: 1; transform: none; }
}

.m-head {
  display: flex;
  align-items: center;
  gap: 9px;
}
.m-badge {
  width: 24px;
  height: 24px;
  border-radius: 7px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.m-title {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  font-size: 16px;
  font-weight: 600;
  color: var(--ink);
  font-family: var(--serif);
  letter-spacing: 0.5px;
}
.m-title:focus { outline: none; }
.m-head-act {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.ghost-btn,
.line-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
  padding: 4px 8px;
  border-radius: 6px;
  cursor: pointer;
}
.ghost-btn:hover { color: var(--text); background: var(--selection-bg); }
.line-btn {
  border-color: var(--border);
  color: var(--text-2);
}
.line-btn:hover { border-color: var(--ink); color: var(--ink); }
.info-dot {
  display: inline-flex;
  color: var(--dim);
  cursor: help;
}
.icon-x {
  border: none;
  background: transparent;
  color: var(--muted);
  display: inline-flex;
  padding: 3px;
  border-radius: 6px;
  cursor: pointer;
}
.icon-x:hover { background: var(--selection-bg); color: var(--text); }

.m-prompt {
  width: 100%;
  min-height: 230px;
  resize: vertical;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 14px;
  font-size: 13px;
  line-height: 1.75;
  color: var(--text);
  background: var(--bg-soft, var(--panel));
  font-family: var(--mono);
}
.m-prompt:focus { outline: none; border-color: var(--primary); }
.m-desc {
  border: none;
  border-bottom: 1px dashed var(--border);
  background: transparent;
  font-size: 12px;
  color: var(--text-2);
  padding: 4px 2px;
}
.m-desc:focus { outline: none; border-bottom-color: var(--primary); }

.m-foot {
  display: flex;
  flex-direction: column;
  gap: 12px;
  border-top: 1px solid var(--border-soft);
  padding-top: 12px;
}
.foot-controls {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  align-items: center;
}
.chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-2);
  font-size: 12px;
  padding: 5px 9px;
  border-radius: 8px;
  cursor: pointer;
}
.chip:hover { border-color: var(--ink); color: var(--ink); }
.chip.on { border-color: var(--ink); color: var(--ink); background: var(--selection-bg); }
.chip .chev { color: var(--dim); margin-left: 1px; }
.chip.toggle.active {
  border-color: var(--primary);
  color: var(--primary-deep, var(--primary));
  background: var(--primary-soft);
}
.chip.stepper { gap: 4px; cursor: default; }
.chip.stepper:hover { border-color: var(--border); color: var(--text-2); }
.step-b {
  border: none;
  background: var(--selection-bg);
  color: var(--text-2);
  width: 17px;
  height: 17px;
  border-radius: 5px;
  cursor: pointer;
  line-height: 1;
  font-size: 13px;
}
.step-b:hover { background: var(--border); color: var(--ink); }
.step-n { min-width: 12px; text-align: center; font-variant-numeric: tabular-nums; }

.menu-anchor { position: relative; }
.dropdown {
  position: absolute;
  z-index: 30;
  bottom: calc(100% + 6px);
  left: 0;
  min-width: 190px;
  max-height: 320px;
  overflow-y: auto;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 12px 34px rgba(0, 0, 0, 0.2);
  padding: 5px;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.dropdown.wide { min-width: 240px; right: 0; left: auto; }
/* 头部的「使用模板」菜单往下展开 */
.m-head .dropdown { bottom: auto; top: calc(100% + 6px); }
.dropdown.sched { min-width: 230px; padding: 9px; gap: 7px; }
.dd-head {
  font-size: 10.5px;
  letter-spacing: 1px;
  color: var(--dim);
  padding: 4px 8px 2px;
  font-family: var(--serif);
}
.dd-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  border: none;
  background: transparent;
  color: var(--text-2);
  font-size: 12.5px;
  padding: 7px 8px;
  border-radius: 6px;
  text-align: left;
  cursor: pointer;
}
.dd-item:hover { background: var(--selection-bg); color: var(--text); }
.dd-item svg { color: var(--muted); flex-shrink: 0; }
.dd-item svg.hide { visibility: hidden; }
.dd-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dd-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.dd-empty { font-size: 12px; color: var(--dim); padding: 8px; font-style: italic; }
.dd-note {
  font-size: 10.5px;
  color: var(--dim);
  line-height: 1.5;
  padding: 2px 4px 0;
}
.rad {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12.5px;
  color: var(--text-2);
  cursor: pointer;
}
.mini-time, .mini-num {
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 2px 5px;
  font-size: 12px;
  background: var(--panel);
  color: var(--text);
}
.mini-num { width: 48px; }
.mini-time:disabled, .mini-num:disabled { opacity: 0.45; }

.foot-submit {
  display: flex;
  align-items: center;
  gap: 12px;
}
.run-now {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--muted);
  margin-right: auto;
  cursor: pointer;
}
.btn-cancel {
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 13px;
  padding: 7px 12px;
  border-radius: 8px;
  cursor: pointer;
}
.btn-cancel:hover { color: var(--ink); background: var(--selection-bg); }
.btn-create {
  border: none;
  background: var(--ink);
  color: #fff;
  font-size: 13px;
  padding: 7px 20px;
  border-radius: 8px;
  cursor: pointer;
  letter-spacing: 1px;
}
.btn-create:hover:not(:disabled) { background: var(--primary); }
.btn-create:disabled { opacity: 0.45; cursor: not-allowed; }
</style>
