import { defineStore } from "pinia";
import { ref, computed } from "vue";
import {
  convApi,
  type Conversation,
  type Project,
} from "../tauri";

export type ViewKey =
  | "chat"
  | "wiki"
  | "graph"
  | "sandbox"
  | "claude_md"
  | "skill_center"
  | "env_doctor"
  | "settings";

export const useAppStore = defineStore("app", () => {
  const view = ref<ViewKey>("chat");
  const sidebarCollapsed = ref(false);
  const drawerCollapsed = ref(false);

  // 置顶对话：仅前端持久化（localStorage），侧栏排序时置顶优先
  const PINNED_KEY = "polaris.pinnedConvs.v1";
  function loadPinned(): Set<string> {
    try {
      const raw = localStorage.getItem(PINNED_KEY);
      if (raw) return new Set(JSON.parse(raw) as string[]);
    } catch {
      /* ignore corrupt storage */
    }
    return new Set();
  }
  const pinnedConvs = ref<Set<string>>(loadPinned());
  function persistPinned() {
    try {
      localStorage.setItem(PINNED_KEY, JSON.stringify([...pinnedConvs.value]));
    } catch {
      /* storage may be unavailable */
    }
  }
  function isPinned(convId: string | null | undefined): boolean {
    return !!convId && pinnedConvs.value.has(convId);
  }
  function togglePin(convId: string) {
    if (!convId) return;
    const s = new Set(pinnedConvs.value);
    if (s.has(convId)) s.delete(convId);
    else s.add(convId);
    pinnedConvs.value = s;
    persistPinned();
  }

  // 任务完成但用户未查看的会话集合 → 侧栏显示墨蓝色未读点
  const unreadConvs = ref<Set<string>>(new Set());
  function markUnread(convId: string) {
    if (!convId) return;
    // 正在查看的对话不标记
    if (convId === currentConvId.value) return;
    unreadConvs.value = new Set(unreadConvs.value).add(convId);
  }
  function clearUnread(convId: string) {
    if (!unreadConvs.value.has(convId)) return;
    const s = new Set(unreadConvs.value);
    s.delete(convId);
    unreadConvs.value = s;
  }

  // 项目 + 对话
  const projects = ref<Project[]>([]);
  const expandedProjects = ref<Set<string>>(new Set());
  const conversationsByProject = ref<Record<string, Conversation[]>>({});
  const currentConvId = ref<string | null>(null);
  const currentProjectId = ref<string | null>(null);

  function setView(v: ViewKey) {
    view.value = v;
  }
  function toggleSidebar() {
    sidebarCollapsed.value = !sidebarCollapsed.value;
  }
  function toggleDrawer() {
    drawerCollapsed.value = !drawerCollapsed.value;
  }

  const sidebarWidth = computed(() => (sidebarCollapsed.value ? 48 : 260));
  const drawerWidth = computed(() => (drawerCollapsed.value ? 48 : 300));

  async function refreshProjects() {
    projects.value = await convApi.listProjects();
    if (!currentProjectId.value && projects.value.length) {
      currentProjectId.value = projects.value[0].id;
      expandedProjects.value.add(currentProjectId.value);
      await refreshConversations(currentProjectId.value);
    }
  }

  async function refreshConversations(projectId: string) {
    conversationsByProject.value[projectId] =
      await convApi.listConversations(projectId);
    // Vue 3 reactive: 替换 ref 触发更新
    conversationsByProject.value = { ...conversationsByProject.value };
  }

  async function toggleProject(projectId: string) {
    if (expandedProjects.value.has(projectId)) {
      expandedProjects.value.delete(projectId);
    } else {
      expandedProjects.value.add(projectId);
      if (!conversationsByProject.value[projectId]) {
        await refreshConversations(projectId);
      }
    }
    expandedProjects.value = new Set(expandedProjects.value);
  }

  async function createProject(name: string) {
    const p = await convApi.createProject(name);
    projects.value = [...projects.value, p];
    expandedProjects.value = new Set([...expandedProjects.value, p.id]);
    currentProjectId.value = p.id;
    conversationsByProject.value = { ...conversationsByProject.value, [p.id]: [] };
    return p;
  }

  async function createConversation(projectId: string) {
    const c = await convApi.createConversation(projectId);
    const cur = conversationsByProject.value[projectId] ?? [];
    conversationsByProject.value = {
      ...conversationsByProject.value,
      [projectId]: [c, ...cur],
    };
    expandedProjects.value = new Set([...expandedProjects.value, projectId]);
    currentConvId.value = c.id;
    currentProjectId.value = projectId;
    setView("chat");
    return c;
  }

  async function deleteConversation(conv: Conversation) {
    await convApi.deleteConversation(conv.id);
    const cur = conversationsByProject.value[conv.projectId] ?? [];
    conversationsByProject.value = {
      ...conversationsByProject.value,
      [conv.projectId]: cur.filter((c) => c.id !== conv.id),
    };
    if (currentConvId.value === conv.id) {
      currentConvId.value = null;
    }
    // 删除后顺手清掉置顶标记，避免遗留垃圾
    if (pinnedConvs.value.has(conv.id)) togglePin(conv.id);
  }

  async function renameConversation(conv: Conversation, title: string) {
    const t = title.trim();
    if (!t || t === conv.title) return;
    await convApi.renameConversation(conv.id, t);
    const cur = conversationsByProject.value[conv.projectId] ?? [];
    conversationsByProject.value = {
      ...conversationsByProject.value,
      [conv.projectId]: cur.map((c) => (c.id === conv.id ? { ...c, title: t } : c)),
    };
  }

  function selectConversation(conv: Conversation) {
    currentConvId.value = conv.id;
    currentProjectId.value = conv.projectId;
    clearUnread(conv.id);
    setView("chat");
  }

  return {
    // ui
    view,
    sidebarCollapsed,
    drawerCollapsed,
    sidebarWidth,
    drawerWidth,
    setView,
    toggleSidebar,
    toggleDrawer,
    unreadConvs,
    markUnread,
    clearUnread,
    // pin
    pinnedConvs,
    isPinned,
    togglePin,
    // conv
    projects,
    expandedProjects,
    conversationsByProject,
    currentConvId,
    currentProjectId,
    refreshProjects,
    refreshConversations,
    toggleProject,
    createProject,
    createConversation,
    deleteConversation,
    renameConversation,
    selectConversation,
  };
});
