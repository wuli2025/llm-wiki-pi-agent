<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import {
  Puzzle,
  Plus,
  Search,
  Sparkles,
  Globe,
  Wrench,
  Trash2,
  Download,
  FileText,
  Table,
  AudioLines,
  Clapperboard,
  Image as ImageIcon,
  Ghost,
  FolderOpen,
  X,
} from "@lucide/vue";
import { skills as skillsApi, isTauri, type Skill } from "../tauri";
import { useSkillsStore } from "../stores/skills";

const skillsStore = useSkillsStore();
const activeTab = ref<"market" | "mine">("market");
const skillList = ref<Skill[]>([]);
const searchQuery = ref("");
const loading = ref(false);
// 正在安装的 skill id（用于按钮 loading 态）
const installing = ref<Set<string>>(new Set());

// 创建弹窗
const showCreateModal = ref(false);
const createForm = ref({
  id: "",
  name: "",
  description: "",
  systemPrompt: "",
});
const createError = ref("");

// 导入/下载弹窗
const showImportModal = ref(false);
const importSource = ref("");
const importing = ref(false);
const importError = ref("");

onMounted(loadSkills);

async function loadSkills() {
  loading.value = true;
  try {
    skillList.value = await skillsApi.list();
  } catch {
    skillList.value = [
      { id: "deep-research", name: "深度搜索", description: "...", source: "third-party" },
      { id: "skill-creator", name: "Skill 创建向导", description: "...", source: "official" },
    ];
  } finally {
    loading.value = false;
  }
}

const marketSkills = computed(() =>
  skillList.value.filter((s) => s.source !== "user")
);

// 「我的技能」= 已安装 / 已创建（物理存在于用户目录，removable）。预装内置只在市场展示。
const mySkills = computed(() =>
  skillList.value.filter((s) => s.removable)
);

const currentSkills = computed(() => {
  const list = activeTab.value === "market" ? marketSkills.value : mySkills.value;
  if (!searchQuery.value.trim()) return list;
  const q = searchQuery.value.toLowerCase();
  return list.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.description.toLowerCase().includes(q)
  );
});

function iconForSkill(skill: Skill) {
  const map: Record<string, any> = {
    "deep-research": Globe,
    "skill-creator": Wrench,
    pdf: FileText,
    xlsx: Table,
    "edge-tts": AudioLines,
    hyperframes: Clapperboard,
    "web-search": Search,
    "image-gen": ImageIcon,
    "cloak-browser": Ghost,
  };
  return map[skill.id] ?? Sparkles;
}

function sourceLabel(source: string) {
  if (source === "official") return "官方";
  if (source === "third-party") return "第三方";
  return "我的";
}

// 从市场安装 → 复制到用户目录 → 重新加载 → 自动激活（立即可用，无需手动确认）
async function onInstall(skill: Skill) {
  if (installing.value.has(skill.id) || skill.installed) return;
  installing.value = new Set(installing.value).add(skill.id);
  try {
    await skillsApi.install(skill.id);
    await loadSkills();
    skillsStore.enable(skill.id); // 安装即激活
  } catch (e: any) {
    alert(`安装失败: ${e?.message ?? e}`);
  } finally {
    const next = new Set(installing.value);
    next.delete(skill.id);
    installing.value = next;
  }
}

// ── 导入 / 下载（外部来源） ──
function openImportModal() {
  importSource.value = "";
  importError.value = "";
  showImportModal.value = true;
}
function closeImportModal() {
  showImportModal.value = false;
}
async function browseFile() {
  if (!isTauri) return;
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const sel = await open({
      multiple: false,
      filters: [{ name: "Skill", extensions: ["md", "zip"] }],
    });
    if (typeof sel === "string") importSource.value = sel;
  } catch {
    /* 用户取消或无 dialog */
  }
}
async function submitImport() {
  const src = importSource.value.trim();
  if (!src) {
    importError.value = "请填写来源：URL / git 仓库 / 本地文件路径";
    return;
  }
  importing.value = true;
  importError.value = "";
  try {
    const ids = await skillsApi.import(src);
    await loadSkills();
    ids.forEach((id) => skillsStore.enable(id)); // 导入即激活
    closeImportModal();
    activeTab.value = "mine";
  } catch (e: any) {
    importError.value = e?.message ?? String(e);
  } finally {
    importing.value = false;
  }
}

async function onDelete(skill: Skill) {
  if (!confirm(`确定移除技能「${skill.name}」?`)) return;
  try {
    await skillsApi.delete(skill.id);
    skillsStore.remove(skill.id);
    await loadSkills();
  } catch (e: any) {
    alert(`移除失败: ${e?.message ?? e}`);
  }
}

function openCreateModal() {
  createForm.value = { id: "", name: "", description: "", systemPrompt: "" };
  createError.value = "";
  showCreateModal.value = true;
}

function closeCreateModal() {
  showCreateModal.value = false;
}

function sanitizeId(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

function onNameInput() {
  if (!createForm.value.id) {
    createForm.value.id = sanitizeId(createForm.value.name);
  }
}

async function submitCreate() {
  createError.value = "";
  const { id, name, description, systemPrompt } = createForm.value;
  if (!id.trim() || !name.trim() || !systemPrompt.trim()) {
    createError.value = "ID、名称和 System Prompt 为必填项";
    return;
  }
  try {
    await skillsApi.create(id.trim(), name.trim(), description.trim(), systemPrompt.trim());
    await loadSkills();
    skillsStore.enable(id.trim()); // 创建即激活
    closeCreateModal();
    activeTab.value = "mine";
  } catch (e: any) {
    createError.value = e?.message ?? String(e);
  }
}
</script>

<template>
  <div class="skill-center">
    <!-- Header -->
    <div class="sc-header">
      <div class="sc-title">
        <Puzzle :size="20" :stroke-width="1.8" class="sc-title-icon" />
        <span>技能中心</span>
      </div>
      <div class="sc-header-actions">
        <button class="sc-import-btn" @click="openImportModal" title="从外部 URL / git 仓库 / 本地文件导入">
          <Download :size="14" :stroke-width="2" />
          <span>导入/下载</span>
        </button>
        <button class="sc-new-btn" @click="openCreateModal">
          <Plus :size="14" :stroke-width="2" />
          <span>新技能</span>
        </button>
      </div>
    </div>

    <!-- Search + Tabs -->
    <div class="sc-toolbar">
      <div class="sc-tabs">
        <button
          class="sc-tab"
          :class="{ active: activeTab === 'market' }"
          @click="activeTab = 'market'"
        >
          市场精选
        </button>
        <button
          class="sc-tab"
          :class="{ active: activeTab === 'mine' }"
          @click="activeTab = 'mine'"
        >
          我的技能
          <span v-if="mySkills.length > 0" class="sc-tab-badge">{{ mySkills.length }}</span>
        </button>
      </div>
      <div class="sc-search">
        <Search :size="14" :stroke-width="1.8" class="sc-search-icon" />
        <input v-model="searchQuery" placeholder="搜索技能..." type="text" />
      </div>
    </div>

    <!-- Skill Grid -->
    <div v-if="!loading" class="sc-grid">
      <div v-for="skill in currentSkills" :key="skill.id" class="sc-card">
        <div class="sc-card-head">
          <div class="sc-card-icon">
            <component :is="iconForSkill(skill)" :size="22" :stroke-width="1.6" />
          </div>
          <div class="sc-card-meta">
            <div class="sc-card-name">{{ skill.name }}</div>
            <div class="sc-card-source">{{ sourceLabel(skill.source) }}</div>
          </div>
        </div>
        <div class="sc-card-desc">{{ skill.description }}</div>
        <div class="sc-card-foot">
          <!-- 未安装 → 安装按钮 -->
          <button
            v-if="!skill.installed"
            class="sc-card-install"
            :disabled="installing.has(skill.id)"
            @click="onInstall(skill)"
          >
            <Download :size="13" :stroke-width="1.9" />
            <span>{{ installing.has(skill.id) ? "安装中…" : "安装" }}</span>
          </button>
          <!-- 已安装 → 状态 + 卸载 + 开关 -->
          <template v-else>
            <span class="sc-installed-badge">已安装</span>
            <button
              v-if="skill.removable"
              class="sc-card-delete"
              @click="onDelete(skill)"
              title="卸载 / 删除"
            >
              <Trash2 :size="13" :stroke-width="1.8" />
            </button>
            <label class="switch" title="开启/关闭">
              <input
                type="checkbox"
                :checked="skillsStore.has(skill.id)"
                @change="skillsStore.toggle(skill.id)"
              />
              <span class="slider round" />
            </label>
          </template>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="currentSkills.length === 0 && !loading" class="sc-empty">
      <template v-if="activeTab === 'mine'">
        <div>还没有安装或创建技能 — 去「市场精选」安装，或自己创建一个</div>
        <button class="sc-empty-btn" @click="openCreateModal">+ 创建第一个技能</button>
      </template>
      <template v-else>
        暂无技能
      </template>
    </div>

    <!-- 导入/下载弹窗 -->
    <div v-if="showImportModal" class="modal-overlay" @click.self="closeImportModal">
      <div class="modal">
        <div class="modal-head">
          <span class="modal-title">导入 / 下载技能</span>
          <button class="modal-close" @click="closeImportModal">
            <X :size="16" :stroke-width="2" />
          </button>
        </div>
        <div class="modal-body">
          <div class="form-row">
            <label>来源（不限来源，鼓励从外面拿）</label>
            <input
              v-model="importSource"
              placeholder="git 仓库 / 远程 .md / .zip / 本地路径"
              @keydown.enter="submitImport"
            />
          </div>
          <div class="import-hint">
            <div>支持任意来源：</div>
            <ul>
              <li><strong>git 仓库</strong>：如 <code>https://github.com/obra/superpowers</code>（整套合集自动逐个装）</li>
              <li><strong>远程文件</strong>：<code>https://…/skill.md</code> 或 <code>https://…/pack.zip</code></li>
              <li><strong>本地</strong>：<code>.md</code> 文件 / <code>.zip</code> 压缩包 / 技能目录</li>
            </ul>
          </div>
          <button v-if="isTauri" class="import-browse" @click="browseFile">
            <FolderOpen :size="14" :stroke-width="1.8" />
            <span>选择本地 .md / .zip 文件…</span>
          </button>
          <div v-if="importError" class="form-error">{{ importError }}</div>
        </div>
        <div class="modal-foot">
          <button class="modal-btn secondary" @click="closeImportModal">取消</button>
          <button class="modal-btn primary" :disabled="importing" @click="submitImport">
            {{ importing ? "导入中…" : "导入" }}
          </button>
        </div>
      </div>
    </div>

    <!-- 创建弹窗 -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="closeCreateModal">
      <div class="modal">
        <div class="modal-head">
          <span class="modal-title">创建新技能</span>
          <button class="modal-close" @click="closeCreateModal">
            <X :size="16" :stroke-width="2" />
          </button>
        </div>
        <div class="modal-body">
          <div class="form-row">
            <label>名称</label>
            <input v-model="createForm.name" placeholder="例如: 高老师风格写作" @input="onNameInput" />
          </div>
          <div class="form-row">
            <label>ID（唯一标识，只能用小写字母、数字、-）</label>
            <input v-model="createForm.id" placeholder="gao-style-writer" />
          </div>
          <div class="form-row">
            <label>描述</label>
            <input v-model="createForm.description" placeholder="一句话描述这个技能的作用..." />
          </div>
          <div class="form-row">
            <label>System Prompt（核心指令）</label>
            <textarea
              v-model="createForm.systemPrompt"
              placeholder="# 角色定义&#10;&#10;你是...&#10;&#10;## 工作方式&#10;1. ..."
              rows="8"
            ></textarea>
          </div>
          <div v-if="createError" class="form-error">{{ createError }}</div>
        </div>
        <div class="modal-foot">
          <button class="modal-btn secondary" @click="closeCreateModal">取消</button>
          <button class="modal-btn primary" @click="submitCreate">创建</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.skill-center {
  height: 100vh;
  overflow-y: auto;
  padding: 24px 32px;
  background: var(--bg);
}
.sc-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}
.sc-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-family: var(--serif);
  font-size: 18px;
  font-weight: 600;
  color: var(--ink);
  letter-spacing: 1px;
}
.sc-title-icon {
  color: var(--primary);
}
.sc-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.sc-new-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  background: var(--ink);
  color: #fafaf7;
  border: none;
  border-radius: 6px;
  font-size: 12.5px;
  cursor: pointer;
}
.sc-new-btn:hover {
  background: var(--primary);
}
.sc-import-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  background: transparent;
  color: var(--text-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 12.5px;
  cursor: pointer;
}
.sc-import-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}

/* 导入弹窗内的提示 */
.import-hint {
  font-size: 12px;
  color: var(--muted);
  line-height: 1.7;
  background: var(--bg-soft);
  border-radius: 8px;
  padding: 10px 12px;
  margin-bottom: 12px;
}
.import-hint ul {
  margin: 6px 0 0;
  padding-left: 18px;
}
.import-hint code {
  font-family: var(--mono);
  font-size: 11px;
  background: var(--panel);
  padding: 1px 4px;
  border-radius: 3px;
}
.import-browse {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  background: transparent;
  color: var(--primary);
  border: 1px dashed var(--border);
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
}
.import-browse:hover {
  border-color: var(--primary);
  background: var(--primary-soft);
}

.sc-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 20px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-soft);
}
.sc-tabs {
  display: flex;
  gap: 4px;
}
.sc-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 13px;
  border-radius: 4px;
  cursor: pointer;
}
.sc-tab:hover {
  color: var(--text);
  background: var(--bg-soft);
}
.sc-tab.active {
  color: var(--ink);
  background: var(--panel);
  font-weight: 600;
  box-shadow: var(--shadow-sm);
}
.sc-tab-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  background: var(--primary-soft);
  color: var(--primary);
  border-radius: 9px;
  font-size: 10.5px;
  font-weight: 600;
}
.sc-search {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 5px 10px;
  width: 240px;
}
.sc-search-icon {
  color: var(--muted);
  flex-shrink: 0;
}
.sc-search input {
  border: none;
  outline: none;
  background: transparent;
  font-size: 12.5px;
  color: var(--text);
  width: 100%;
}
.sc-search input::placeholder {
  color: var(--dim);
}

.sc-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}
.sc-card {
  background: var(--panel);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
  padding: 16px;
  box-shadow: var(--shadow-sm);
  transition: box-shadow 0.15s, border-color 0.15s;
}
.sc-card:hover {
  box-shadow: var(--shadow);
  border-color: var(--border);
}
.sc-card-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}
.sc-card-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: var(--primary-soft);
  color: var(--primary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.sc-card-meta {
  flex: 1;
  min-width: 0;
}
.sc-card-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}
.sc-card-source {
  font-size: 11px;
  color: var(--muted);
  margin-top: 2px;
}
.sc-card-desc {
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.6;
  margin-bottom: 12px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.sc-card-foot {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: flex-end;
}
.sc-card-delete {
  padding: 5px;
  background: transparent;
  border: none;
  color: var(--muted);
  border-radius: 4px;
  cursor: pointer;
}
.sc-card-delete:hover {
  color: var(--vermilion);
  background: rgba(192, 57, 43, 0.06);
}
.sc-card-use {
  padding: 5px 14px;
  background: var(--ink);
  color: #fafaf7;
  border: none;
  border-radius: 5px;
  font-size: 12px;
  cursor: pointer;
}
.sc-card-use:hover {
  background: var(--primary);
}

/* 安装按钮 */
.sc-card-install {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 14px;
  background: var(--ink);
  color: #fafaf7;
  border: none;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
}
.sc-card-install:hover {
  background: var(--primary);
}
.sc-card-install:disabled {
  background: var(--border);
  color: var(--muted);
  cursor: default;
}
/* 已安装徽标 */
.sc-installed-badge {
  margin-right: auto;
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  background: rgba(16, 185, 129, 0.1);
  color: #0f9d6b;
  border-radius: 10px;
  font-size: 11px;
  font-weight: 600;
}

/* ─── 绿键 Toggle Switch ─── */
.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
}
.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}
.slider {
  position: absolute;
  cursor: pointer;
  inset: 0;
  background: #ccc;
  border-radius: 20px;
  transition: 0.2s;
}
.slider::before {
  content: "";
  position: absolute;
  height: 16px;
  width: 16px;
  left: 2px;
  bottom: 2px;
  background: white;
  border-radius: 50%;
  transition: 0.2s;
}
input:checked + .slider {
  background: #10b981;
}
input:checked + .slider::before {
  transform: translateX(16px);
}

.sc-empty {
  text-align: center;
  padding: 60px 0;
  color: var(--muted);
  font-size: 13px;
}
.sc-empty-btn {
  margin-top: 12px;
  padding: 6px 16px;
  background: var(--ink);
  color: #fafaf7;
  border: none;
  border-radius: 6px;
  font-size: 12.5px;
  cursor: pointer;
}
.sc-empty-btn:hover {
  background: var(--primary);
}

/* ─────────── 创建弹窗 ─────────── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(20, 20, 25, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.modal {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: 520px;
  max-width: 90vw;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-lg);
}
.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-soft);
}
.modal-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}
.modal-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--muted);
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}
.modal-close:hover {
  background: var(--bg-soft);
  color: var(--text);
}
.modal-body {
  padding: 16px 20px;
  overflow-y: auto;
}
.form-row {
  margin-bottom: 14px;
}
.form-row label {
  display: block;
  font-size: 12px;
  color: var(--text-2);
  margin-bottom: 5px;
}
.form-row input,
.form-row textarea {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 13px;
  background: var(--bg);
  color: var(--text);
  outline: none;
  resize: vertical;
}
.form-row input:focus,
.form-row textarea:focus {
  border-color: var(--primary);
}
.form-error {
  color: var(--vermilion);
  font-size: 12px;
  padding: 4px 0;
}
.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 20px 16px;
  border-top: 1px solid var(--border-soft);
}
.modal-btn {
  padding: 6px 16px;
  border-radius: 6px;
  font-size: 13px;
  border: none;
  cursor: pointer;
}
.modal-btn.secondary {
  background: var(--bg-soft);
  color: var(--text-2);
}
.modal-btn.secondary:hover {
  background: var(--border);
}
.modal-btn.primary {
  background: var(--ink);
  color: #fafaf7;
}
.modal-btn.primary:hover {
  background: var(--primary);
}
</style>
