import { defineStore } from "pinia";
import { ref } from "vue";

/**
 * 工作流包（Workflow Pack）
 * ──────────────────────────────────────────────────────────────
 * 一个「结构化提示词」：由若干有序「环节(step)」编排而成。
 * 每个环节是一段带标题的提示词正文，调整环节顺序 / 增删即可重新编排，
 * 适合那些需要不断改变编排方式的任务。点「使用」会把所有环节拼装成
 * 一段完整提示词，填入对话输入框。本地持久化（localStorage）。
 */
export interface WorkflowStep {
  id: string;
  label: string; // 环节标题，如「角色」「任务」「约束」「输出格式」
  content: string; // 该环节的提示词正文
}

export interface WorkflowPack {
  id: string;
  name: string;
  description: string;
  color: string; // 强调色（取自 PACK_COLORS）
  steps: WorkflowStep[];
  createdAt: number;
  updatedAt: number;
}

/** 工作流编辑器提交的数据（id 为空 = 新建） */
export interface WorkflowDraft {
  id?: string;
  name: string;
  description: string;
  color: string;
  steps: WorkflowStep[];
}

const STORAGE_KEY = "polaris:workflow-packs:v1";
const SEED_KEY = "polaris:workflow-packs-seeded:v1";

/** 墨蓝水墨主题取色 —— 每个包一抹强调色 */
export const PACK_COLORS = [
  "#2c4661", // 墨蓝
  "#a78c4f", // 金
  "#c0392b", // 朱
  "#3f6b5a", // 松绿
  "#6b4f7a", // 紫
  "#9c6b3f", // 赭石
];

function uid(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

export function newStep(label = "", content = ""): WorkflowStep {
  return { id: uid(), label, content };
}

/** 把一个工作流包的环节拼装成最终提示词 */
export function assemblePack(p: { steps: WorkflowStep[] }): string {
  return p.steps
    .map((s) => {
      const body = s.content.trim();
      if (!body) return "";
      const label = s.label.trim();
      return label ? `【${label}】\n${body}` : body;
    })
    .filter(Boolean)
    .join("\n\n");
}

/** 首启种入的示例包（只种一次） */
function seedPacks(): WorkflowPack[] {
  const now = Date.now();
  return [
    {
      id: uid(),
      name: "深度调研报告",
      description: "围绕一个主题做多源检索，交叉验证后产出结构化报告",
      color: PACK_COLORS[0],
      createdAt: now,
      updatedAt: now,
      steps: [
        newStep("角色", "你是一名严谨的研究分析师，擅长多源检索与交叉验证。"),
        newStep(
          "任务",
          "围绕主题「__________」展开调研：检索权威来源，提炼关键事实，标注分歧与不确定处。"
        ),
        newStep(
          "方法",
          "1. 列出 3–5 个核心子问题\n2. 逐个检索并记录来源\n3. 对冲突信息交叉验证\n4. 汇总成结论"
        ),
        newStep(
          "输出格式",
          "Markdown 报告：摘要 → 关键发现（带来源）→ 风险/分歧 → 结论与建议。"
        ),
      ],
    },
    {
      id: uid(),
      name: "代码审查",
      description: "对一段代码做高标准审查，聚焦正确性、安全与可维护性",
      color: PACK_COLORS[3],
      createdAt: now,
      updatedAt: now,
      steps: [
        newStep("角色", "你是一位资深工程师，做代码审查时直接、具体、不客套。"),
        newStep(
          "审查重点",
          "正确性与边界条件 · 安全隐患 · 性能 · 命名与可读性 · 是否与现有风格一致。"
        ),
        newStep(
          "输出",
          "按「严重 / 建议 / 小提示」分级，每条给出文件:行号与可操作的修改方案。"
        ),
      ],
    },
  ];
}

export const useWorkflowsStore = defineStore("workflows", () => {
  const packs = ref<WorkflowPack[]>([]);

  // 编辑器（新建 / 修改共用一个模态）
  const editorOpen = ref(false);
  const editorTarget = ref<WorkflowPack | null>(null); // null = 新建

  // 「使用」→ 把拼装文本送进对话输入框；带 nonce 以便重复使用同一包也能触发
  const insertRequest = ref<{ text: string; n: number } | null>(null);
  let insertSeq = 0;

  function load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        packs.value = JSON.parse(raw) as WorkflowPack[];
        return;
      }
    } catch {
      /* ignore，落到种入分支 */
    }
    // 首启：种入示例包一次（用户删光后不再回种）
    if (!localStorage.getItem(SEED_KEY)) {
      packs.value = seedPacks();
      localStorage.setItem(SEED_KEY, "1");
      persist();
    }
  }

  function persist() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(packs.value));
  }

  function openCreate() {
    editorTarget.value = null;
    editorOpen.value = true;
  }
  function openEdit(p: WorkflowPack) {
    editorTarget.value = p;
    editorOpen.value = true;
  }
  function closeEditor() {
    editorOpen.value = false;
    editorTarget.value = null;
  }

  /** 新建或更新一个包 */
  function savePack(draft: WorkflowDraft) {
    const now = Date.now();
    const steps = draft.steps.filter(
      (s) => s.label.trim() || s.content.trim()
    );
    if (draft.id) {
      const i = packs.value.findIndex((p) => p.id === draft.id);
      if (i >= 0) {
        packs.value[i] = {
          ...packs.value[i],
          name: draft.name.trim(),
          description: draft.description.trim(),
          color: draft.color,
          steps,
          updatedAt: now,
        };
        packs.value = [...packs.value];
      }
    } else {
      packs.value = [
        {
          id: uid(),
          name: draft.name.trim(),
          description: draft.description.trim(),
          color: draft.color,
          steps,
          createdAt: now,
          updatedAt: now,
        },
        ...packs.value,
      ];
    }
    persist();
  }

  function removePack(id: string) {
    packs.value = packs.value.filter((p) => p.id !== id);
    persist();
  }

  /** 点「使用」：拼装并请求填入对话框 */
  function usePack(p: WorkflowPack) {
    insertRequest.value = { text: assemblePack(p), n: ++insertSeq };
  }

  function clearInsert() {
    insertRequest.value = null;
  }

  load();

  return {
    packs,
    editorOpen,
    editorTarget,
    insertRequest,
    openCreate,
    openEdit,
    closeEditor,
    savePack,
    removePack,
    usePack,
    clearInsert,
  };
});
