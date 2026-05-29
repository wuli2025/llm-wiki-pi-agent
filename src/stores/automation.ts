import { defineStore } from "pinia";
import { ref } from "vue";
import { convApi, type PermissionMode } from "../tauri";
import { useAppStore } from "./app";
import { useChatStore } from "./chat";

/**
 * 自动化（Automation）板块 —— 板块⑨
 * ──────────────────────────────────────────────────────────────
 * 一个「自动化流程」= 一段编排好的提示词 + 运行配置（在哪个项目跑、什么时候跑、
 * 循环几次、是否深度检测）。运行时在所选项目下新建一个对话，把提示词作为一轮
 * 发给本机 Claude Code（复用 chat 管线，本地轻量化执行），流式结果就是这条对话。
 *
 * 设计参考 WorkBuddy / Codex 的「routine / scheduled task」与阿里悟空的「自动工作流」：
 * 都把「可复用的指令模板 + 触发时机 + 执行上下文」三者解耦。这里走最轻的本地实现：
 * 不引入独立编排引擎，靠一段强 agentic 提示词驱动 Claude 自己完成「搜索→撰写→评价→存草稿」。
 * 不直接对外发布（公众号/小红书），只把成品落到项目草稿箱，由用户自行发送。
 */

export type ScheduleKind = "manual" | "daily" | "interval";
export interface Schedule {
  kind: ScheduleKind;
  /** daily: "HH:MM" */
  time?: string;
  /** interval: 每多少小时跑一次 */
  everyHours?: number;
}

export type ExecEnv = "local" | "sandbox";

export interface AutomationFlow {
  id: string;
  name: string;
  /** lucide 图标名（Automation.vue 里映射成组件） */
  icon: string;
  color: string;
  description: string;
  /** 编排好的提示词正文（创建/编辑对话框里的大文本框） */
  prompt: string;
  /** 在哪个项目里运行；null = 运行时取当前项目 */
  projectId: string | null;
  execEnv: ExecEnv;
  schedule: Schedule;
  /** 循环几次（≥1）；>1 时让流程自我迭代改进 */
  loopCount: number;
  /** 是否深度检测（联网深度搜索 deep-research） */
  deepResearch: boolean;
  /** 内置流程（不可删，可改一份副本） */
  builtin?: boolean;
  createdAt: number;
  updatedAt: number;
  lastRunAt?: number;
  /** 上次运行生成的对话 id（用于「缩小版对话框」回看进度） */
  lastConvId?: string;
}

export interface FlowDraft {
  id?: string;
  name: string;
  icon: string;
  color: string;
  description: string;
  prompt: string;
  projectId: string | null;
  execEnv: ExecEnv;
  schedule: Schedule;
  loopCount: number;
  deepResearch: boolean;
}

const STORAGE_KEY = "polaris:automation-flows:v1";
const SEED_KEY = "polaris:automation-flows-seeded:v1";

export const FLOW_COLORS = [
  "#2c4661", // 墨蓝
  "#c0392b", // 朱（小红书）
  "#3f6b5a", // 松绿（公众号）
  "#a78c4f", // 金
  "#6b4f7a", // 紫（B站）
];

function uid(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

// ───────────────────────── 内置流程的提示词 ─────────────────────────
// 共同约定：用 deep-research 联网搜最近资讯 → 用本项目知识库(PolarisKB，沿双链用
// Read/Glob/Grep 自取) 模仿既有风格撰写 → 多维评审打分并据此修订 → 成品落到「草稿箱」
// 文件(.md + 可预览 .html)，**绝不**自动对外发布，留给用户自己发。

const DRAFT_CONVENTION = `\n\n## 产出约定（务必遵守）
- 全程**不要**真的对外发布到任何平台；你没有、也不需要发布权限。
- 把最终成品保存为文件放进本会话的产物目录（草稿箱）：一份 Markdown 正文 + 一份自包含、可直接预览的 HTML。文件名带「草稿」二字与日期。
- 在回答末尾用一句话点明生成了哪些草稿文件，并提示「请在草稿箱预览后自行决定是否发布」。`;

const wechatPrompt = `你是一名资深微信公众号主编，要为下面这个方向产出一篇**可直接进草稿箱**的公众号长文（不发布）。

【主题方向】__________（在这里填你想写的方向/选题，可留一句话即可）

请按以下步骤完成，并把过程与结论写清楚：
1. **选题调研（联网）**：围绕该方向用深度搜索检索「最近一段时间」的新闻、动态与高赞讨论，提炼 3-5 个有传播潜力的切入角度，标注信息来源与时间。挑出最优角度，给出本文核心论点/独特价值。
2. **取材与风格对齐**：阅读本项目的知识库（PolarisKB，沿双链用 Read/Glob/Grep 自取相关条目），让选材、观点与**既有写作风格**保持一致，避免与知识库矛盾。
3. **成文**：按「标题 → 开头钩子 → 框架 → 正文(1500-3000字) → 结尾行动号召」写成稿，小标题清晰、论点有论据(数据/案例/引用)、节奏好读，并在合适处标注【配图建议】。给 5 个备选标题并推荐一个。
4. **多维评审**：以主编视角对成稿做多维度打分(满分10)并逐条点评：①标题/开头打开率 ②逻辑与论据 ③信息增量与可信度 ④阅读节奏与排版 ⑤结尾记忆点。对每条给「问题→具体改法」，据此产出**修订后的终稿**与一句话推送摘要。${DRAFT_CONVENTION}`;

const xhsPrompt = `你是一名深耕小红书的资深内容操盘手，要为下面这个方向产出一篇**可直接进草稿箱**的小红书爆款笔记（不发布）。

【主题方向】__________（在这里填你想写的方向/选题）

请按以下步骤完成：
1. **选题调研（联网）**：用深度搜索检索该方向「最近」的热点、爆款选题与用户痛点，提炼 3 个候选角度(痛点型/教程型/反差种草型)，标注来源与时间，挑出最优角度与 1 个核心卖点。
2. **风格对齐**：阅读本项目知识库（PolarisKB，沿双链用 Read/Glob/Grep 自取），让语气与选材贴合**既有风格**——真实、有温度、像朋友分享。
3. **成稿**：产出 5 个 20 字内备选标题(覆盖数字/痛点/反差/身份代入多类钩子，含 1-2 个 emoji)并推荐一个；正文 300-600 字，开头两行制造停留，主体分点(小标题+emoji+短句多换行)，结尾引导互动；末尾给 8-12 个话题标签(#)，大词+精准长尾+热点词。
4. **爆款自检评审**：以平台流量视角打分(满分10)并逐项点评(标题点击欲/前三行留人/价值点/互动引导/标签精准度)，每条给「问题→改法」，产出**修订后的最终版**。${DRAFT_CONVENTION}`;

const bilibiliPrompt = `你是一名严谨的知识库维护者，本流程的目标是**用 B 站同领域内容反哺、补全本项目的知识库**（不产出对外稿件）。

【关注领域】__________（在这里填你要补全知识库的领域/主题）

请按以下步骤完成：
1. **同领域调研（联网）**：用深度搜索以 B 站为主、辅以其他来源，检索该领域「最近」较优质、较高互动的视频/专栏所覆盖的知识点与观点，整理成一份「外部知识点清单」，每条标注来源与时间。
2. **对照本知识库找缺口**：阅读本项目知识库（PolarisKB，沿双链用 Read/Glob/Grep 通读相关条目），把「外部知识点清单」与现有条目逐项对照，产出三类结论：
   - **可补全**：知识库缺失/陈旧、且有可靠来源可补的点；
   - **需存疑**：来源不足或有争议、不宜直接写入的点（说明原因）；
   - **已覆盖**：知识库已有的点（避免重复）。
3. **补全知识库**：对「可补全」的点，按知识库既有的条目格式与双链风格，新增/更新相应 Markdown 条目（放回知识库对应位置，建立双链），每条注明来源与补充日期；不要把「需存疑」的内容写进库。
4. **小结**：列出本次新增/更新了哪些条目、跳过了哪些及原因，便于人工复核。${DRAFT_CONVENTION}`;

function seedFlows(): AutomationFlow[] {
  const now = Date.now();
  const base = {
    projectId: null,
    execEnv: "local" as ExecEnv,
    deepResearch: true,
    loopCount: 1,
    builtin: true,
    createdAt: now,
    updatedAt: now,
  };
  return [
    {
      ...base,
      id: uid(),
      name: "微信公众号 · 自动撰稿（草稿）",
      icon: "newspaper",
      color: FLOW_COLORS[2],
      description:
        "选方向 → 深度搜最近新闻 → 仿知识库风格成文 → 多维评审修订 → 存草稿箱（不发布）",
      prompt: wechatPrompt,
      schedule: { kind: "manual" },
    },
    {
      ...base,
      id: uid(),
      name: "小红书 · 自动撰稿（草稿）",
      icon: "book-marked",
      color: FLOW_COLORS[1],
      description:
        "选方向 → 深度搜热点 → 仿风格写爆款笔记 → 爆款自检评审 → 存草稿箱（不发布）",
      prompt: xhsPrompt,
      schedule: { kind: "manual" },
    },
    {
      ...base,
      id: uid(),
      name: "B站调研 · 补全知识库",
      icon: "tv",
      color: FLOW_COLORS[4],
      description:
        "深度搜 B 站同领域内容 → 对照知识库找缺口（可补/存疑/已覆盖）→ 补全知识库条目",
      prompt: bilibiliPrompt,
      schedule: { kind: "manual" },
    },
  ];
}

export const useAutomationStore = defineStore("automation", () => {
  const flows = ref<AutomationFlow[]>([]);

  // 创建 / 编辑对话框
  const editorOpen = ref(false);
  const editorTarget = ref<AutomationFlow | null>(null); // null = 新建

  // 「缩小版对话框」：当前在面板里查看运行进度的对话 id
  const activeConvId = ref<string | null>(null);

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) flows.value = JSON.parse(raw) as AutomationFlow[];
    } catch {
      /* ignore，落种入分支 */
    }
    if (flows.value.length === 0 && !localStorage.getItem(SEED_KEY)) {
      flows.value = seedFlows();
      localStorage.setItem(SEED_KEY, "1");
      persist();
    }
  }

  function persist() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(flows.value));
  }

  function openCreate() {
    editorTarget.value = null;
    editorOpen.value = true;
  }
  function openEdit(f: AutomationFlow) {
    editorTarget.value = f;
    editorOpen.value = true;
  }
  function closeEditor() {
    editorOpen.value = false;
    editorTarget.value = null;
  }

  function saveFlow(draft: FlowDraft): AutomationFlow {
    const now = Date.now();
    if (draft.id) {
      const i = flows.value.findIndex((f) => f.id === draft.id);
      if (i >= 0) {
        flows.value[i] = {
          ...flows.value[i],
          name: draft.name.trim() || flows.value[i].name,
          icon: draft.icon,
          color: draft.color,
          description: draft.description.trim(),
          prompt: draft.prompt,
          projectId: draft.projectId,
          execEnv: draft.execEnv,
          schedule: draft.schedule,
          loopCount: Math.max(1, draft.loopCount || 1),
          deepResearch: draft.deepResearch,
          updatedAt: now,
        };
        flows.value = [...flows.value];
        persist();
        return flows.value[i];
      }
    }
    const f: AutomationFlow = {
      id: uid(),
      name: draft.name.trim() || "未命名自动化",
      icon: draft.icon || "sparkles",
      color: draft.color,
      description: draft.description.trim(),
      prompt: draft.prompt,
      projectId: draft.projectId,
      execEnv: draft.execEnv,
      schedule: draft.schedule,
      loopCount: Math.max(1, draft.loopCount || 1),
      deepResearch: draft.deepResearch,
      createdAt: now,
      updatedAt: now,
    };
    flows.value = [f, ...flows.value];
    persist();
    return f;
  }

  function removeFlow(id: string) {
    flows.value = flows.value.filter((f) => f.id !== id);
    persist();
  }

  /** 组装最终发给 Claude 的提示词（叠加循环 / 深度检测的框架说明） */
  function composePrompt(f: AutomationFlow): string {
    let p = f.prompt.trim();
    if (f.loopCount > 1) {
      p += `\n\n## 迭代要求\n请把上述流程独立执行并自我迭代共 ${f.loopCount} 轮：每轮在上一轮成品基础上，针对评审发现的最大问题做实质性改进，最终只把**最好的一版**留在草稿箱（其余轮次仅说明改了什么）。`;
    }
    if (f.deepResearch) {
      p += `\n\n（已开启「深度检测」：请尽量多源联网检索、交叉验证，区分事实与观点，并标注来源与时间。）`;
    }
    return p;
  }

  /** 运行一个流程：在所选项目下新建对话，把提示词作为一轮发给本机 Claude（复用 chat 管线） */
  async function runFlow(f: AutomationFlow): Promise<string | null> {
    const app = useAppStore();
    const chat = useChatStore();
    const projectId = f.projectId || app.currentProjectId;
    if (!projectId) return null;

    const conv = await convApi.createConversation(projectId);
    // 让侧栏 / 项目对话列表也能看到这条运行记录
    await app.refreshConversations(projectId).catch(() => {});

    const permissionMode: PermissionMode = "auto_current";
    const skillIds = f.deepResearch ? ["deep-research"] : [];
    const prompt = composePrompt(f);
    const display = `自动化「${f.name}」运行中…`;

    await chat.send(conv.id, prompt, display, undefined, {
      permissionMode,
      skillIds,
    });

    // 标记运行态
    const i = flows.value.findIndex((x) => x.id === f.id);
    if (i >= 0) {
      flows.value[i] = { ...flows.value[i], lastRunAt: Date.now(), lastConvId: conv.id };
      flows.value = [...flows.value];
      persist();
    }
    activeConvId.value = conv.id;
    return conv.id;
  }

  // ───────────── 轻量本地调度器：app 开着时按 schedule 触发 ─────────────
  // 每分钟检查一次；daily=到点且当天未跑过则跑；interval=距上次 ≥ everyHours 小时则跑。
  let timer: number | undefined;
  const lastDailyFire = ref<Record<string, string>>({}); // flowId -> "YYYY-MM-DD HH:MM"

  function tick() {
    const now = new Date();
    for (const f of flows.value) {
      const s = f.schedule;
      if (!s || s.kind === "manual") continue;
      const chat = useChatStore();
      // 上一轮还在跑就跳过，避免叠加
      if (f.lastConvId && chat.isSending(f.lastConvId)) continue;

      if (s.kind === "daily" && s.time) {
        const [hh, mm] = s.time.split(":").map((x) => parseInt(x, 10));
        const stamp = `${now.toDateString()} ${s.time}`;
        if (
          now.getHours() === hh &&
          now.getMinutes() === mm &&
          lastDailyFire.value[f.id] !== stamp
        ) {
          lastDailyFire.value[f.id] = stamp;
          void runFlow(f);
        }
      } else if (s.kind === "interval" && s.everyHours && s.everyHours > 0) {
        const due = (f.lastRunAt ?? 0) + s.everyHours * 3600_000;
        if (Date.now() >= due) void runFlow(f);
      }
    }
  }

  function startScheduler() {
    if (timer != null) return;
    timer = window.setInterval(tick, 60_000);
  }
  function stopScheduler() {
    if (timer != null) {
      clearInterval(timer);
      timer = undefined;
    }
  }

  load();

  return {
    flows,
    editorOpen,
    editorTarget,
    activeConvId,
    openCreate,
    openEdit,
    closeEditor,
    saveFlow,
    removeFlow,
    composePrompt,
    runFlow,
    startScheduler,
    stopScheduler,
  };
});
