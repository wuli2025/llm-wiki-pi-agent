<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import {
  MessagesSquare,
  Library,
  Waypoints,
  Clock,
  Puzzle,
  CloudDownload,
  FileText,
  Stethoscope,
  Server,
  Settings,
  PanelLeftClose,
  PanelLeftOpen,
  Pin,
  Folder,
  FolderOpen,
  MoreHorizontal,
  Archive,
} from "@lucide/vue";
import { useAppStore } from "../stores/app";
import ProviderDock from "./ProviderDock.vue";
import type { Conversation } from "../tauri";

const app = useAppStore();

type NavItem = { key: typeof app.view; label: string; icon: any };
// 常驻主项（仿豆包：顶层精简）。统一用 lucide 线性图标，去掉杂乱的 Unicode 字符，求一致的高级线条感。
const primaryNav: NavItem[] = [
  { key: "chat", label: "对话", icon: MessagesSquare },
  { key: "wiki", label: "知识库", icon: Library },
  { key: "graph", label: "图谱", icon: Waypoints },
  { key: "automation", label: "自动化", icon: Clock },
  // 沙箱入口已隐藏：进入沙箱视图首挂载较重、点击有卡顿，且当前非核心路径。
  // 视图与路由（App.vue / SandboxStatus）保留，未来需要时把这一项加回即可。
  { key: "skill_center", label: "技能中心", icon: Puzzle },
  { key: "update", label: "更新", icon: CloudDownload },
];
// 收纳进「更多」的次要项（目录说明 / 环境 / MCP / 设置）
const moreNav: NavItem[] = [
  { key: "claude_md", label: "目录说明", icon: FileText },
  { key: "env_doctor", label: "环境", icon: Stethoscope },
  { key: "mcp", label: "MCP", icon: Server },
  { key: "settings", label: "设置", icon: Settings },
];
const showMore = ref(false);
const moreActive = computed(() => moreNav.some((i) => i.key === app.view));
function pickNav(k: typeof app.view) {
  app.setView(k);
}

const newProjectName = ref("");
const showNewProject = ref(false);

onMounted(() => {
  app.refreshProjects();
});

async function submitNewProject() {
  const n = newProjectName.value.trim();
  if (!n) {
    showNewProject.value = false;
    return;
  }
  await app.createProject(n);
  newProjectName.value = "";
  showNewProject.value = false;
}

async function newConv(pid: string) {
  await app.createConversation(pid);
}

async function confirmDelete(c: Conversation) {
  if (confirm(`删除对话「${c.title}」?(消息也会被清空)`)) {
    await app.deleteConversation(c);
  }
}

// 项目「…」更多菜单（仿 Codex 项目操作）：在资源管理器打开 / 归档移除
const openMenuPid = ref<string | null>(null);
function toggleProjMenu(pid: string) {
  openMenuPid.value = openMenuPid.value === pid ? null : pid;
}
function closeProjMenu() {
  openMenuPid.value = null;
}
async function revealProject(pid: string) {
  closeProjMenu();
  try {
    await app.openProjectDir(pid);
  } catch (e) {
    console.error("打开项目目录失败", e);
  }
}
async function archiveProj(proj: { id: string; name: string }) {
  closeProjMenu();
  if (
    confirm(
      `归档项目「${proj.name}」?\n该项目会从列表移除（对话与文件保留，不会删除）。`
    )
  ) {
    await app.archiveProject(proj.id);
  }
}

// 对话按「几天的一个对话」分组：置顶 → 今天 → 昨天 → 7 天内 → 更早，
// 各组内按最近活跃时间倒序（最新的在最上）。仿 Codex：项目名虚化、对话实体可标注。
interface ConvGroup {
  label: string;
  items: Conversation[];
}
const DAY_MS = 86_400_000;
// updatedAt 兼容秒/毫秒：小于 1e12 视为秒，统一换算成毫秒
function toMs(t: number): number {
  return t < 1e12 ? t * 1000 : t;
}
// 该时间戳属于「今天起算的第几天前」（0=今天, 1=昨天, ...）
function daysAgo(t: number): number {
  const now = new Date();
  const startToday = new Date(
    now.getFullYear(),
    now.getMonth(),
    now.getDate()
  ).getTime();
  return Math.floor((startToday - toMs(t)) / DAY_MS);
}
function convGroups(projectId: string): ConvGroup[] {
  const list = app.conversationsByProject[projectId] || [];
  const byTimeDesc = (a: Conversation, b: Conversation) => b.updatedAt - a.updatedAt;
  const pinned = list.filter((c) => app.isPinned(c.id)).sort(byTimeDesc);
  const rest = list.filter((c) => !app.isPinned(c.id)).sort(byTimeDesc);

  const today: Conversation[] = [];
  const yest: Conversation[] = [];
  const week: Conversation[] = [];
  const older: Conversation[] = [];
  for (const c of rest) {
    const d = daysAgo(c.updatedAt);
    if (d <= 0) today.push(c);
    else if (d === 1) yest.push(c);
    else if (d <= 7) week.push(c);
    else older.push(c);
  }

  const groups: ConvGroup[] = [];
  if (pinned.length) groups.push({ label: "置顶", items: pinned });
  if (today.length) groups.push({ label: "今天", items: today });
  if (yest.length) groups.push({ label: "昨天", items: yest });
  if (week.length) groups.push({ label: "7 天内", items: week });
  if (older.length) groups.push({ label: "更早", items: older });
  return groups;
}
</script>

<template>
  <aside class="sb" :class="{ collapsed: app.sidebarCollapsed }">
    <!-- Head：顶部留白，仅保留收起按钮（品牌 logo/文字已按要求移除） -->
    <div class="sb-head">
      <template v-if="!app.sidebarCollapsed">
        <button
          class="collapse-btn push-right"
          title="收起侧栏"
          @click="app.toggleSidebar()"
        >
          <PanelLeftClose :size="17" :stroke-width="1.7" />
        </button>
      </template>
      <template v-else>
        <button
          class="collapse-btn rail"
          title="展开侧栏"
          @click="app.toggleSidebar()"
        >
          <PanelLeftOpen :size="17" :stroke-width="1.7" />
        </button>
      </template>
    </div>

    <!-- Nav -->
    <nav class="nav">
      <button
        v-for="it in primaryNav"
        :key="it.key"
        class="nav-item"
        :class="{ active: app.view === it.key }"
        :title="it.label"
        @click="pickNav(it.key)"
      >
        <span class="glyph-icon"
          ><component :is="it.icon" :size="17" :stroke-width="1.6"
        /></span>
        <span v-if="!app.sidebarCollapsed" class="label">{{ it.label }}</span>
      </button>

      <!-- 更多：把 目录说明 / 环境 / MCP / 设置 收纳进来（仿豆包，顶层更清爽） -->
      <button
        class="nav-item"
        :class="{ active: moreActive && !showMore, expanded: showMore }"
        :title="'更多'"
        @click="showMore = !showMore"
      >
        <span class="glyph-icon"
          ><MoreHorizontal :size="17" :stroke-width="1.6"
        /></span>
        <span v-if="!app.sidebarCollapsed" class="label">更多</span>
        <span v-if="!app.sidebarCollapsed" class="more-chev">{{
          showMore ? "▾" : "▸"
        }}</span>
      </button>

      <template v-if="showMore">
        <button
          v-for="it in moreNav"
          :key="it.key"
          class="nav-item sub"
          :class="{ active: app.view === it.key }"
          :title="it.label"
          @click="pickNav(it.key)"
        >
          <span class="glyph-icon"
            ><component :is="it.icon" :size="16" :stroke-width="1.6"
          /></span>
          <span v-if="!app.sidebarCollapsed" class="label">{{ it.label }}</span>
        </button>
      </template>
    </nav>

    <!-- Projects + Conversations -->
    <div v-if="!app.sidebarCollapsed" class="proj-section">
      <div class="proj-head">
        <span class="proj-title">项目</span>
        <button
          class="ic-btn plus"
          title="新建项目"
          @click="showNewProject = !showNewProject"
        >
          +
        </button>
      </div>

      <div v-if="showNewProject" class="new-proj-row">
        <input
          v-model="newProjectName"
          placeholder="项目名"
          @keydown.enter="submitNewProject"
          @keydown.esc="(showNewProject = false), (newProjectName = '')"
          autofocus
        />
        <button class="primary-mini" @click="submitNewProject">建</button>
      </div>

      <div v-for="proj in app.projects" :key="proj.id" class="proj-block">
        <div
          class="proj"
          :class="{ active: app.currentProjectId === proj.id, open: app.expandedProjects.has(proj.id) }"
          @click="app.toggleProject(proj.id)"
        >
          <component
            :is="app.expandedProjects.has(proj.id) ? FolderOpen : Folder"
            class="folder"
            :size="15"
            :stroke-width="1.7"
          />
          <span class="name">{{ proj.name }}</span>
          <button
            class="ic-btn plus mini"
            title="新建对话"
            @click.stop="newConv(proj.id)"
          >
            +
          </button>
          <button
            class="ic-btn dots mini"
            :class="{ on: openMenuPid === proj.id }"
            title="更多操作"
            @click.stop="toggleProjMenu(proj.id)"
          >
            <MoreHorizontal :size="14" :stroke-width="1.8" />
          </button>

          <!-- 项目操作菜单（仿 Codex 右侧「…」）-->
          <div v-if="openMenuPid === proj.id" class="proj-menu" @click.stop>
            <button class="pm-item" @click="revealProject(proj.id)">
              <FolderOpen :size="14" :stroke-width="1.7" />
              <span>在资源管理器中打开</span>
            </button>
            <div class="pm-sep"></div>
            <button class="pm-item danger" @click="archiveProj(proj)">
              <Archive :size="14" :stroke-width="1.7" />
              <span>归档项目（移出列表）</span>
            </button>
          </div>
        </div>

        <template v-if="app.expandedProjects.has(proj.id)">
          <template v-for="g in convGroups(proj.id)" :key="g.label">
            <div class="day-label">{{ g.label }}</div>
            <div
              v-for="c in g.items"
              :key="c.id"
              class="conv"
              :class="{ active: app.currentConvId === c.id, pinned: app.isPinned(c.id) }"
              @click="app.selectConversation(c)"
            >
              <span
                v-if="app.unreadConvs.has(c.id)"
                class="cv-dot"
                title="有已完成的任务待查看"
              ></span>
              <Pin
                v-if="app.isPinned(c.id)"
                :size="11"
                :stroke-width="1.8"
                class="cv-pin"
              />
              <span class="cv-name" :title="c.title">{{ c.title }}</span>
              <button
                class="ca delete"
                title="删除对话"
                @click.stop="confirmDelete(c)"
              >
                ×
              </button>
            </div>
          </template>
          <div
            v-if="(app.conversationsByProject[proj.id] || []).length === 0"
            class="empty-hint"
          >
            点项目右侧 + 新建对话
          </div>
        </template>
      </div>
    </div>

    <!-- 点击空白处关闭项目菜单 -->
    <div v-if="openMenuPid" class="menu-backdrop" @click="closeProjMenu()"></div>

    <div class="footer">
      <ProviderDock :collapsed="app.sidebarCollapsed" />
    </div>
  </aside>
</template>

<style scoped>
.sb {
  background: var(--bg-soft);
  border-right: 1px solid var(--border-soft);
  display: flex;
  flex-direction: column;
  padding: 8px 8px 6px;
  overflow: hidden;
}
.sb.collapsed {
  padding: 8px 4px;
}

.sb-head {
  display: flex;
  align-items: center;
  padding: 4px 4px 8px;
  gap: 6px;
}
.collapse-btn.push-right {
  margin-left: auto;
}
.collapse-btn {
  width: 26px;
  height: 26px;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--muted);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s, color 0.15s;
}
.collapse-btn:hover {
  background: var(--selection-bg);
  color: var(--text);
}
.collapse-btn.rail {
  margin: 0 auto;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border: none;
  border-radius: 3px;
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
  text-align: left;
}
.nav-item:hover {
  background: var(--selection-bg);
}
.nav-item.active {
  background: var(--selection-bg);
  color: var(--text);
  font-weight: 500;
  border-left: 2px solid var(--ink);
  padding-left: 8px;
}
.sb.collapsed .nav-item {
  justify-content: center;
  padding: 7px 0;
}
.sb.collapsed .nav-item.active {
  border-left: none;
  border-right: 2px solid var(--ink);
}
/* 「更多」展开态 + 折叠箭头 */
.more-chev {
  margin-left: auto;
  font-size: 9px;
  color: var(--dim);
}
.nav-item.expanded {
  color: var(--text);
}
/* 「更多」里的次要项：缩进 + 字号略小，作为子级 */
.nav-item.sub {
  padding-left: 26px;
  font-size: 12.5px;
  color: var(--muted);
}
.nav-item.sub .glyph,
.nav-item.sub .glyph-icon {
  width: 15px;
}
.nav-item.sub.active {
  padding-left: 24px;
}
.sb.collapsed .nav-item.sub {
  padding-left: 0;
}
.glyph {
  display: inline-block;
  width: 16px;
  text-align: center;
  color: var(--muted);
  font-family: var(--serif);
}
.glyph-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  color: var(--muted);
}
.nav-item.active .glyph,
.nav-item.active .glyph-icon {
  color: var(--ink);
}
.label {
  flex: 1;
}

.proj-section {
  margin-top: 14px;
  padding-top: 10px;
  border-top: 1px solid var(--border-soft);
  overflow-y: auto;
  flex: 1;
}
.proj-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 10px 6px;
}
.proj-title {
  font-family: var(--serif);
  font-size: 11px;
  letter-spacing: 1.5px;
  color: var(--dim);
}
.ic-btn {
  width: 18px;
  height: 18px;
  border: none;
  border-radius: 3px;
  background: transparent;
  color: var(--muted);
  font-size: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}
.ic-btn:hover {
  background: var(--border);
  color: var(--text);
}
.ic-btn.plus {
  background: var(--ink);
  color: #fff;
  font-size: 11px;
}
.ic-btn.plus:hover {
  background: var(--primary);
}
.ic-btn.mini {
  opacity: 0;
}
/* 项目「…」更多操作按钮：幽灵态，hover 行才显形；菜单打开时常驻 */
.ic-btn.dots {
  color: var(--dim);
}
.ic-btn.dots:hover {
  background: var(--border);
  color: var(--text);
}
.ic-btn.dots.on {
  opacity: 1;
  background: var(--border);
  color: var(--text);
}

.new-proj-row {
  display: flex;
  gap: 4px;
  padding: 4px 10px 6px;
}
.new-proj-row input {
  flex: 1;
  padding: 4px 6px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-size: 12px;
  background: var(--panel);
}
.new-proj-row input:focus {
  outline: none;
  border-color: var(--primary);
}
.primary-mini {
  padding: 2px 10px;
  background: var(--ink);
  color: #fff;
  border: none;
  border-radius: 3px;
  font-size: 11px;
}
.primary-mini:hover {
  background: var(--primary);
}

.proj-block {
  margin-bottom: 4px;
  position: relative;
}
/* 项目 = 文件夹（仿 Codex）：名称虚化、低调，弱化为「分组容器」 */
.proj {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 10px;
  font-size: 12.5px;
  border-radius: 7px;
  cursor: pointer;
}
.proj:hover {
  background: var(--selection-bg);
}
.proj:hover .ic-btn.mini {
  opacity: 1;
}
.proj.active,
.proj.open {
  background: transparent;
}
.proj .folder {
  color: var(--dim);
  flex-shrink: 0;
}
.proj.open .folder,
.proj:hover .folder {
  color: var(--muted);
}
.proj .name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  /* 虚化：低对比、字距拉开，作为分组标题而非主角 */
  color: var(--muted);
  font-weight: 500;
  letter-spacing: 0.5px;
}
.proj:hover .name {
  color: var(--text-2);
}

/* 项目操作下拉菜单 —— 软阴影 + 圆角，求精致高级感 */
.proj-menu {
  position: absolute;
  z-index: 50;
  top: 30px;
  right: 6px;
  min-width: 184px;
  padding: 5px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.16), 0 2px 8px rgba(0, 0, 0, 0.08);
  display: flex;
  flex-direction: column;
  gap: 1px;
  animation: pmIn 0.13s ease;
}
@keyframes pmIn {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.97);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
.pm-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 7px 9px;
  border: none;
  background: transparent;
  color: var(--text-2);
  font-size: 12.5px;
  border-radius: 6px;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.pm-item svg {
  color: var(--muted);
  flex-shrink: 0;
}
.pm-item:hover {
  background: var(--selection-bg);
  color: var(--text);
}
.pm-item:hover svg {
  color: var(--text);
}
.pm-item.danger:hover {
  color: var(--vermilion);
}
.pm-item.danger:hover svg {
  color: var(--vermilion);
}
.pm-sep {
  height: 1px;
  margin: 3px 6px;
  background: var(--border-soft);
}
.menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 45;
}

.day-label {
  font-size: 10px;
  color: var(--dim);
  padding: 7px 10px 3px 30px;
  font-family: var(--serif);
  letter-spacing: 1.5px;
}
/* 对话 = 实体（仿 Codex）：更醒目、可点的主条目，颜色加深、字号略大 */
.conv {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 6px 10px 6px 30px;
  font-size: 13px;
  color: var(--text-2);
  border-radius: 7px;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.conv:hover {
  background: var(--selection-bg);
  color: var(--text);
}
.conv:hover .ca {
  opacity: 1;
}
.conv.active {
  background: var(--selection-bg-hover);
  color: var(--text);
  font-weight: 600;
}
.cv-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--primary);
  box-shadow: 0 0 0 2px var(--primary-soft);
  flex-shrink: 0;
  animation: cvDotIn 0.3s ease;
}
@keyframes cvDotIn {
  from { transform: scale(0); }
  to { transform: scale(1); }
}
.cv-pin {
  flex-shrink: 0;
  color: var(--gold);
  transform: rotate(35deg);
}
.cv-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ca {
  width: 16px;
  height: 16px;
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 13px;
  border-radius: 2px;
  opacity: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}
.ca:hover {
  background: var(--border);
  color: var(--text);
}
.ca.delete:hover {
  color: var(--vermilion);
}

.empty-hint {
  font-size: 11px;
  color: var(--dim);
  padding: 4px 10px 4px 26px;
  font-style: italic;
}

.footer {
  margin-top: auto;
  padding-top: 6px;
  border-top: 1px solid var(--border-soft);
}
.footer-text {
  font-size: 10.5px;
  color: var(--dim);
  text-align: center;
  font-family: var(--serif);
  letter-spacing: 1.5px;
  padding: 4px 0;
}
</style>
