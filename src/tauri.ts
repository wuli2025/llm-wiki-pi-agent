/**
 * Typed wrappers around Tauri commands.
 *
 * Designed so the renderer can still mount in a plain browser (npm run dev) by
 * detecting absence of __TAURI_INTERNALS__ and returning empty / stub data.
 */
import { invoke as rawInvoke } from "@tauri-apps/api/core";
import { listen as rawListen, type UnlistenFn } from "@tauri-apps/api/event";

export const isTauri =
  typeof window !== "undefined" &&
  // @ts-ignore tauri injects this
  typeof (window as any).__TAURI_INTERNALS__ !== "undefined";

export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    // Browser-only stubs so the UI is still navigable during pure-web dev.
    return browserStub(cmd, args) as T;
  }
  return rawInvoke<T>(cmd, args);
}

export async function listen<T>(
  event: string,
  cb: (payload: T) => void
): Promise<UnlistenFn> {
  if (!isTauri) return () => {};
  return rawListen<T>(event, (e) => cb(e.payload));
}

// ──────────────────────────────────────────────────────────────
// KB module
// ──────────────────────────────────────────────────────────────
export interface KbHit {
  path: string;
  title: string;
  snippet: string;
  score: number;
}
export interface KbNode {
  id: string;
  title: string;
  category: string;
  /** "doc" 文档 | "folder" 目录中枢 | "root" 知识库根 */
  kind: "doc" | "folder" | "root";
}
export interface KbEdge {
  source: string;
  target: string;
}
export interface KbGraph {
  nodes: KbNode[];
  edges: KbEdge[];
}
/** 知识库拖拽上传的逐文件结果 */
export interface KbUploadResult {
  name: string;
  relPath: string;
  ok: boolean;
  message: string;
}

export const kb = {
  scan: () => invoke<number>("kb_scan"),
  search: (q: string, topK = 8) =>
    invoke<KbHit[]>("kb_search", { query: q, topK }),
  list: (subdir: string | null = null) =>
    invoke<string[]>("kb_list", { subdir }),
  read: (relPath: string) => invoke<string>("kb_read", { relPath }),
  /** 删除一份资料(浏览页 ×)，返回剩余文件数 */
  delete: (relPath: string) => invoke<number>("kb_delete", { relPath }),
  /** 清空资料库(管理页)，返回剩余文件数 */
  clear: () => invoke<number>("kb_clear"),
  ingest: (sourcePath: string) =>
    invoke<string>("kb_ingest", { sourcePath }),
  /** 拖拽上传：任意格式 → 转 markdown 入 raw/，返回逐文件结果 */
  uploadFiles: (paths: string[]) =>
    invoke<KbUploadResult[]>("kb_upload_files", { paths }),
  graph: () => invoke<KbGraph>("kb_graph"),
  root: () => invoke<string>("kb_root"),
  defaultRoot: () => invoke<string>("kb_default_root"),
  setRoot: (newPath: string) =>
    invoke<number>("kb_set_root", { newPath }),
};

// ──────────────────────────────────────────────────────────────
// Sandbox module → 已迁出至 features/sandbox/api.ts (架构重构 Phase 1)
// 浏览器降级 stub 仍保留在本文件下方的 browserStub() 中。
// ──────────────────────────────────────────────────────────────

// ──────────────────────────────────────────────────────────────
// Chat module
// ──────────────────────────────────────────────────────────────
export type PermissionMode =
  | "manual"
  | "auto_current"
  | "auto_all"
  | "deny";

export interface ChatSendArgs {
  prompt: string;
  permissionMode: PermissionMode;
  useSandbox?: boolean;
  skillIds?: string[];
  conversationId?: string;
  /** 目标模式：完成条件。设置后 Claude 会持续推进直到达成，不中途收尾。 */
  goal?: string;
  /** 「请教毛主席」：注入毛选式客观分析指令，调用毛主席资料库，生成标注来源的 HTML。 */
  consultMao?: boolean;
}

export interface ChatStreamEvent {
  reqId: string;
  kind: "delta" | "tool" | "error" | "done" | "artifact";
  text?: string;
  tool?: string;
  conversationId?: string;
}

/** 对话拖拽上传的附件（复制进会话 uploads 目录） */
export interface AttachedFile {
  name: string;
  /** uploads 目录里的绝对路径（正斜杠） */
  path: string;
  /** text | image | pdf | office | binary */
  kind: "text" | "image" | "pdf" | "office" | "binary";
  size: number;
  ok: boolean;
  error?: string;
}

export const chat = {
  send: (args: ChatSendArgs) =>
    invoke<string>("chat_send", { args: args as unknown as Record<string, unknown> }),
  cancel: (reqId: string) => invoke<void>("chat_cancel", { reqId }),
  /** 拖拽上传：把文件复制进当前会话，返回附件清单 */
  attachFiles: (conversationId: string | undefined, paths: string[]) =>
    invoke<AttachedFile[]>("chat_attach_files", {
      conversationId: conversationId ?? null,
      paths,
    }),
};

// ──────────────────────────────────────────────────────────────
// Artifacts module — 对话生成的成品文件，右侧抽屉预览
// ──────────────────────────────────────────────────────────────
export type ArtifactKind =
  | "html"
  | "svg"
  | "image"
  | "markdown"
  | "text"
  | "binary";

export interface ArtifactPayload {
  path: string;
  name: string;
  ext: string;
  kind: ArtifactKind;
  /** 文本类(html/svg/markdown/text)内容 */
  text?: string;
  /** 图片类的 data URL */
  dataUrl?: string;
  size: number;
}

/** 「参考资料」文件夹视图的一条文件记录 */
export interface ArtifactEntry {
  path: string;
  name: string;
  ext: string;
  kind: ArtifactKind;
  size: number;
  /** 修改时间 Unix 秒 */
  modified: number;
}

export const artifacts = {
  read: (path: string) => invoke<ArtifactPayload>("artifact_read", { path }),
  openExternal: (path: string) =>
    invoke<void>("artifact_open_external", { path }),
  /** 在系统文件管理器中定位并选中该文件（资源管理器 / 访达） */
  reveal: (path: string) => invoke<void>("artifact_reveal", { path }),
  /** 列出某会话产物文件，按修改时间倒序 */
  list: (conversationId?: string) =>
    invoke<ArtifactEntry[]>("artifact_list", {
      conversationId: conversationId ?? null,
    }),
  /** 跨所有对话检索历史产物文件（文件名 + 正文） */
  search: (query: string) =>
    invoke<ArtifactSearchHit[]>("artifact_search", { query }),
};

/** 跨对话产物搜索命中 */
export interface ArtifactSearchHit {
  path: string;
  name: string;
  kind: ArtifactKind;
  conversationId: string;
  snippet: string;
  modified: number;
  score: number;
}

// ──────────────────────────────────────────────────────────────
// Skills module
// ──────────────────────────────────────────────────────────────
export interface Skill {
  id: string;
  name: string;
  description: string;
  source: string;
  /** 是否已拥有可用（预装 / 已安装 / 用户自建） */
  installed?: boolean;
  /** 是否可删除（物理存在于用户目录，可卸载） */
  removable?: boolean;
}

export const skills = {
  list: () => invoke<Skill[]>("list_skills"),
  get: (id: string) => invoke<Skill>("get_skill", { id }),
  create: (id: string, name: string, description: string, systemPrompt: string) =>
    invoke<void>("create_skill", { id, name, description, systemPrompt }),
  install: (id: string) => invoke<void>("install_skill", { id }),
  /** 从外部来源导入：本地 .md/.zip/目录 · 远程 .md/.zip · git 仓库 URL（返回导入的 id 列表） */
  import: (source: string) => invoke<string[]>("import_skill", { source }),
  delete: (id: string) => invoke<void>("delete_skill", { id }),
};

// ──────────────────────────────────────────────────────────────
// CLAUDE.md 主上下文 module
// 每个 conv 项目一份 + KB 共享一份
// ──────────────────────────────────────────────────────────────
export interface ProjectClaudeMd {
  projectId: string;
  projectName: string;
  absPath: string;
  exists: boolean;
  active: boolean;
  size: number;
}

export interface KbClaudeMd {
  absPath: string;
  exists: boolean;
  active: boolean;
  size: number;
}

export type ClaudeMdArea = "kb" | "project";

export const claudeMd = {
  listProjects: () => invoke<ProjectClaudeMd[]>("claude_md_list_projects"),
  kbInfo: () => invoke<KbClaudeMd>("claude_md_kb_info"),
  read: (area: ClaudeMdArea, projectId?: string) =>
    invoke<string>("claude_md_read", { area, projectId: projectId ?? null }),
  write: (area: ClaudeMdArea, projectId: string | undefined, content: string) =>
    invoke<void>("claude_md_write", {
      area,
      projectId: projectId ?? null,
      content,
    }),
};

// ──────────────────────────────────────────────────────────────
// Conv module (项目 + 对话历史)
// ──────────────────────────────────────────────────────────────
export interface Project {
  id: string;
  name: string;
  createdAt: number;
  archived: boolean;
}

export interface Conversation {
  id: string;
  projectId: string;
  title: string;
  createdAt: number;
  updatedAt: number;
}

export interface Message {
  id: string;
  conversationId: string;
  role: "user" | "assistant" | "tool";
  content: string;
  createdAt: number;
}

// Rust 端用 snake_case, serde 默认行为, 这里手动映射回 camelCase
type RawProject = {
  id: string;
  name: string;
  created_at: number;
  archived: boolean;
};
type RawConv = {
  id: string;
  project_id: string;
  title: string;
  created_at: number;
  updated_at: number;
};
type RawMsg = {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  created_at: number;
};

const p = (r: RawProject): Project => ({
  id: r.id,
  name: r.name,
  createdAt: r.created_at,
  archived: r.archived,
});
const c = (r: RawConv): Conversation => ({
  id: r.id,
  projectId: r.project_id,
  title: r.title,
  createdAt: r.created_at,
  updatedAt: r.updated_at,
});
const m = (r: RawMsg): Message => ({
  id: r.id,
  conversationId: r.conversation_id,
  role: r.role as Message["role"],
  content: r.content,
  createdAt: r.created_at,
});

export const convApi = {
  listProjects: async () => (await invoke<RawProject[]>("conv_list_projects")).map(p),
  createProject: async (name: string) =>
    p(await invoke<RawProject>("conv_create_project", { name })),
  archiveProject: (projectId: string) =>
    invoke<void>("conv_archive_project", { projectId }),
  listConversations: async (projectId: string) =>
    (await invoke<RawConv[]>("conv_list_conversations", { projectId })).map(c),
  createConversation: async (projectId: string) =>
    c(await invoke<RawConv>("conv_create_conversation", { projectId })),
  deleteConversation: (conversationId: string) =>
    invoke<void>("conv_delete_conversation", { conversationId }),
  renameConversation: (conversationId: string, title: string) =>
    invoke<void>("conv_rename_conversation", { conversationId, title }),
  getMessages: async (conversationId: string) =>
    (await invoke<RawMsg[]>("conv_get_messages", { conversationId })).map(m),
};

// ──────────────────────────────────────────────────────────────
// API 供应商坞 + 用量看板 module
// ──────────────────────────────────────────────────────────────
export interface ProviderView {
  id: string;
  name: string;
  note: string;
  baseUrl: string;
  tokenField: string;
  category: string; // official | cn_official | aggregator | third_party | cloud_provider | custom
  websiteUrl: string;
  color: string;
  kind: string; // official | key | codex | copilot | custom
  isPreset: boolean;
  hasKey: boolean;
  authToken: string;
  /** pi `--model` 用的模型 id */
  model: string;
  /** pi 协议 (anthropic-messages) */
  api: string;
  /** 完整 settings_config（env：base_url + token） */
  settingsConfig: any;
}
export interface ProviderListResult {
  providers: ProviderView[];
  currentId: string;
}
export interface ProviderSaveInput {
  id?: string;
  name: string;
  note?: string;
  websiteUrl?: string;
  tokenField?: string;
  /** pi `--model` 用的模型 id */
  model?: string;
  /** 完整 settings_config（env 含 base_url + token） */
  settingsConfig: any;
}
export interface TokenBucket {
  input: number;
  output: number;
  cacheRead: number;
  cacheCreation: number;
  total: number;
  requests: number;
  cost: number;
}
export interface DailyUsage {
  date: string;
  label: string;
  total: number;
  cost: number;
}
export interface UsageSummary {
  available: boolean;
  today: TokenBucket;
  week: TokenBucket;
  month: TokenBucket;
  year: TokenBucket;
  daily: DailyUsage[];
}
export interface CodexStatus {
  installed: boolean;
  loggedIn: boolean;
  authPath: string;
}

export const provider = {
  list: () => invoke<ProviderListResult>("provider_list"),
  switch: (id: string) => invoke<string>("provider_switch", { id }),
  save: (input: ProviderSaveInput) =>
    invoke<string>("provider_save", { input }),
  delete: (id: string) => invoke<void>("provider_delete", { id }),
  usage: () => invoke<UsageSummary>("usage_summary"),
  codexStatus: () => invoke<CodexStatus>("codex_status"),
  codexLogin: () => invoke<void>("codex_login"),
};

// ──────────────────────────────────────────────────────────────
// 环境医生 module — 新用户「环境监测 + 配置安装」(claude / pwsh / PATH)
// ──────────────────────────────────────────────────────────────
export interface ToolStatus {
  key: "pi" | "pwsh" | "node" | "npm";
  name: string;
  found: boolean;
  version: string | null;
  path: string | null;
  onPath: boolean;
  required: boolean;
  hint: string;
}
export interface EnvReport {
  os: string;
  pi: ToolStatus;
  pwsh: ToolStatus;
  node: ToolStatus;
  npm: ToolStatus;
  piDir: string | null;
  piDirOnUserPath: boolean;
  ready: boolean;
}
export interface PathFixResult {
  ok: boolean;
  dir: string | null;
  status: string;
  message: string;
}
export interface EnvStreamEvent {
  reqId: string;
  kind: "log" | "error" | "done";
  line?: string;
  ok?: boolean;
  message?: string;
}
/** pi 更新检测结果 */
export interface PiUpdateInfo {
  installed: boolean;
  current: string | null;
  latest: string | null;
  updateAvailable: boolean;
  checked: boolean;
  message: string;
}

export const envDoctor = {
  check: () => invoke<EnvReport>("env_check"),
  fixPath: () => invoke<PathFixResult>("env_fix_path"),
  /** 安装 pi 内核 (npm install -g @earendil-works/pi-coding-agent) */
  installPi: (method: "native" | "npm" = "npm") =>
    invoke<string>("env_install_pi", { method }),
  /** 安装 Node.js LTS (winget) —— pi(npm 包) 的前置依赖 */
  installNode: () => invoke<string>("env_install_node"),
  installPwsh: () => invoke<string>("env_install_pwsh"),
  /** 检测 pi 是否有新版本 (当前版本 vs npm latest) */
  checkPiUpdate: () => invoke<PiUpdateInfo>("env_pi_update_check"),
  /** 更新 pi 到最新版，流式日志同安装 */
  updatePi: () => invoke<string>("env_update_pi"),
  cancel: (reqId: string) => invoke<void>("env_cancel", { reqId }),
};

// ──────────────────────────────────────────────────────────────
// Browser stubs (when running in plain `npm run dev` without Tauri)
// ──────────────────────────────────────────────────────────────
function browserStub(cmd: string, _args?: Record<string, unknown>): unknown {
  switch (cmd) {
    case "kb_scan":
      return 0;
    case "kb_search":
      return [];
    case "kb_list":
      return [];
    case "kb_read":
      return "_(browser stub)_  本文件需要 Tauri 后端读取。";
    case "kb_delete":
      return 0;
    case "kb_clear":
      return 0;
    case "kb_ingest":
      return "browser-stub";
    case "kb_upload_files": {
      const paths = (_args?.paths as string[]) ?? [];
      return paths.map((p) => ({
        name: p.split(/[\\/]/).pop() || p,
        relPath: `raw/${p.split(/[\\/]/).pop() || p}`,
        ok: true,
        message: "(browser stub)",
      }));
    }
    case "chat_attach_files": {
      const paths = (_args?.paths as string[]) ?? [];
      return paths.map((p) => ({
        name: p.split(/[\\/]/).pop() || p,
        path: p,
        kind: "binary",
        size: 0,
        ok: true,
      }));
    }
    case "kb_graph":
      return { nodes: [], edges: [] };
    case "kb_root":
      return "(browser-only, no fs access)";
    case "kb_default_root":
      return "(browser-only)";
    case "kb_set_root":
      return 0;
    case "sandbox_status":
      return {
        docker_installed: false,
        docker_running: false,
        image_built: false,
        image_name: "polaris-sandbox:alpine",
        container_running: false,
        container_name: "polaris-sandbox",
        notes: ["浏览器模式 - 仅 UI 预览,无 Docker 能力"],
      };
    case "sandbox_build_image":
    case "sandbox_start":
    case "sandbox_stop":
    case "sandbox_exec":
      return "(browser stub)";
    case "cube_config_get":
      return { backend: "docker", endpoint: "", apiKey: "" };
    case "cube_config_set":
      return (_args?.config as unknown) ?? { backend: "docker", endpoint: "", apiKey: "" };
    case "cube_status":
      return {
        backend: "docker",
        endpoint: "",
        configured: false,
        reachable: false,
        note: "浏览器模式 - 无后端探测",
      };
    case "chat_send":
      return "stub-req-id";
    case "artifact_read": {
      const path = String(_args?.path ?? "demo.html");
      return {
        path,
        name: path.split("/").pop() || path,
        ext: "html",
        kind: "html",
        text:
          "<!doctype html><html><body style='font-family:sans-serif;padding:40px;text-align:center'><h1>预览占位</h1><p>浏览器模式无后端，无法读取真实文件。</p></body></html>",
        size: 0,
      };
    }
    case "artifact_open_external":
      return undefined;
    case "artifact_list":
      return [];
    case "artifact_search":
      return [];
    case "list_skills":
      return [
        { id: "deep-research", name: "深度搜索", description: "使用 LLM 大规模联网搜索相关内容，自动检索、汇总、交叉验证多来源信息", source: "third-party", installed: true, removable: false },
        { id: "skill-creator", name: "Skill 创建向导", description: "引导用户创建自定义 Skill，自动生成模板和配置文件", source: "official", installed: true, removable: false },
        { id: "pdf", name: "PDF 文档处理", description: "提取 / 生成 / 编辑 PDF：抽取文本表格、合并拆分、Markdown 转 PDF、表单与 OCR", source: "official", installed: false, removable: false },
        { id: "xlsx", name: "Excel 表格", description: "读取分析与生成 Excel：透视统计、公式、图表、多 sheet 报表", source: "official", installed: false, removable: false },
        { id: "pptx", name: "PPT 演示文稿", description: "把 PDF / 文档 / 数据转成有高级感的 PPT：母版配色、版式层级、图表，python-pptx 生成", source: "official", installed: false, removable: false },
        { id: "edge-tts", name: "语音合成 Edge-TTS", description: "把文本转成自然语音音频，多语言多音色，免费无需 key", source: "third-party", installed: false, removable: false },
        { id: "hyperframes", name: "视频动画 Hyperframes", description: "用逐帧 / 分镜方式生成短视频与动画，ffmpeg 合成，可配 Edge-TTS 旁白", source: "third-party", installed: false, removable: false },
        { id: "web-search", name: "联网搜索", description: "实时联网检索，基于 Tavily / Brave 等真实来源回答并交叉验证", source: "third-party", installed: false, removable: false },
        { id: "image-gen", name: "AI 生图 gpt-image-2", description: "用 OpenAI gpt-image-2 模型按描述生成图片，自动扩写提示词，支持多候选与改图", source: "third-party", installed: false, removable: false },
        { id: "cloak-browser", name: "CloakBrowser 浏览器", description: "Agent 默认浏览器：源码级隐身 Chromium，drop-in 替换 Playwright，过 Cloudflare / 反爬。可随时关闭移除", source: "third-party", installed: true, removable: false },
      ];
    case "get_skill":
      return { id: "deep-research", name: "深度搜索", description: "使用 LLM 大规模联网搜索相关内容", source: "third-party", installed: true, removable: false };
    case "import_skill":
      return ["browser-stub-skill"];
    case "create_skill":
    case "install_skill":
    case "delete_skill":
      return undefined;
    case "conv_list_projects":
      return [
        {
          id: "p-stub",
          name: "(浏览器) 示例项目",
          created_at: 0,
          archived: false,
        },
      ];
    case "conv_create_project":
      return {
        id: "p-stub-new",
        name: (_args?.name as string) || "新项目",
        created_at: 0,
        archived: false,
      };
    case "conv_list_conversations":
      return [];
    case "conv_create_conversation":
      return {
        id: "c-stub-new",
        project_id: _args?.projectId as string,
        title: "新对话",
        created_at: 0,
        updated_at: 0,
      };
    case "conv_get_messages":
      return [];
    case "conv_archive_project":
    case "conv_delete_conversation":
    case "conv_rename_conversation":
      return undefined;
    case "claude_md_list_projects":
      return [];
    case "claude_md_kb_info":
      return {
        absPath: "(browser-only)",
        exists: false,
        active: false,
        size: 0,
      };
    case "claude_md_read":
      return "_(browser stub)_  本文件需要 Tauri 后端读取。";
    case "claude_md_write":
      return undefined;
    case "provider_list": {
      const mk = (id: string, name: string, baseUrl: string, category: string, color: string, kind: string, hasKey: boolean, model = "claude-sonnet-4-5", authToken = "") => ({
        id, name, note: "", baseUrl, tokenField: "ANTHROPIC_AUTH_TOKEN", category, websiteUrl: baseUrl, color, kind, isPreset: true, hasKey, authToken,
        model, api: "anthropic-messages",
        settingsConfig: { env: baseUrl ? { ANTHROPIC_BASE_URL: baseUrl, ...(authToken ? { ANTHROPIC_AUTH_TOKEN: authToken } : {}) } : {} },
      });
      return {
        providers: [
          mk("claude-official", "Claude 官方", "", "official", "#D97757", "official", true, "claude-opus-4-7"),
          mk("zhipu-glm", "智谱 GLM", "https://open.bigmodel.cn/api/anthropic", "cn_official", "#1f4e79", "key", false, "glm-4.6"),
          mk("kimi", "Kimi 月之暗面", "https://api.moonshot.cn/anthropic", "cn_official", "#1f4e79", "key", true, "kimi-k2-0905-preview", "sk-demo"),
          mk("deepseek", "DeepSeek 深度求索", "https://api.deepseek.com/anthropic", "cn_official", "#1f4e79", "key", false, "deepseek-chat"),
          mk("openrouter", "OpenRouter", "https://openrouter.ai/api", "aggregator", "#3a6ea5", "key", false, "anthropic/claude-sonnet-4.5"),
          mk("aihubmix", "AiHubMix", "https://aihubmix.com", "aggregator", "#3a6ea5", "key", false),
          mk("packycode", "PackyCode", "https://www.packyapi.com", "third_party", "#5a7a9a", "key", false),
          mk("github-copilot", "GitHub Copilot", "https://api.githubcopilot.com", "third_party", "#5a7a9a", "copilot", false, ""),
          mk("codex", "Codex (ChatGPT)", "https://chatgpt.com/backend-api/codex", "third_party", "#5a7a9a", "codex", false, ""),
        ],
        currentId: "kimi",
      };
    }
    case "provider_switch":
      return String(_args?.id ?? "claude-official");
    case "provider_save":
      return "custom-stub";
    case "provider_delete":
      return undefined;
    case "codex_status":
      return { installed: false, loggedIn: false, authPath: "(browser-only)" };
    case "codex_login":
      return undefined;
    case "env_check": {
      const tool = (key: string, name: string, found: boolean, required = false): ToolStatus => ({
        key: key as ToolStatus["key"],
        name,
        found,
        version: found ? "(browser stub) v0.0.0" : null,
        path: found ? `/usr/local/bin/${key}` : null,
        onPath: found,
        required,
        hint: found ? "(browser stub) 已安装" : "未安装 —— 浏览器预览无法真实检测",
      });
      return {
        os: "browser",
        pi: tool("pi", "pi (内核)", false, true),
        pwsh: tool("pwsh", "PowerShell 7", false),
        node: tool("node", "Node.js", true),
        npm: tool("npm", "npm", true),
        piDir: null,
        piDirOnUserPath: true,
        ready: false,
      };
    }
    case "env_fix_path":
      return {
        ok: false,
        dir: null,
        status: "skipped",
        message: "浏览器预览模式无法修改环境变量。",
      };
    case "env_install_pi":
    case "env_install_node":
    case "env_install_pwsh":
    case "env_update_pi":
      return "env-stub-req";
    case "env_pi_update_check":
      return {
        installed: true,
        current: "1.0.0",
        latest: "1.0.1",
        updateAvailable: true,
        checked: true,
        message: "(browser stub) 发现新版本 1.0.1 (当前 1.0.0)。",
      };
    case "env_cancel":
      return undefined;
    case "usage_summary": {
      const daily = Array.from({ length: 14 }, (_, i) => {
        const d = new Date(Date.now() - (13 - i) * 86400000);
        const label = `${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
        return { date: label, label, total: Math.round(300000 + Math.random() * 1600000), cost: +(Math.random() * 6).toFixed(4) };
      });
      return {
        available: true,
        today: { input: 75600, output: 644800, cacheRead: 45506800, cacheCreation: 1637200, total: 720483 + 47144001, requests: 411, cost: 49.107 },
        week: { input: 280000, output: 64000, cacheRead: 6100000, cacheCreation: 410000, total: 6854000, requests: 248, cost: 112.4 },
        month: { input: 980000, output: 240000, cacheRead: 22000000, cacheCreation: 1400000, total: 24620000, requests: 940, cost: 421.8 },
        year: { input: 1900000, output: 520000, cacheRead: 44000000, cacheCreation: 2800000, total: 49220000, requests: 1894, cost: 980.5 },
        daily,
      };
    }
    default:
      return null;
  }
}
