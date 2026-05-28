<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  claudeMd,
  type ClaudeMdArea,
  type KbClaudeMd,
  type ProjectClaudeMd,
} from "../tauri";

type Selected =
  | { kind: "kb" }
  | { kind: "project"; projectId: string; projectName: string };

const projects = ref<ProjectClaudeMd[]>([]);
const kbInfo = ref<KbClaudeMd | null>(null);

const selected = ref<Selected | null>(null);
const content = ref("");
const originalContent = ref("");
const loading = ref(false);
const saving = ref(false);
const message = ref<{ kind: "ok" | "err"; text: string } | null>(null);

const dirty = computed(() => content.value !== originalContent.value);

const selectedMeta = computed(() => {
  const s = selected.value;
  if (!s) return null;
  if (s.kind === "kb") {
    return {
      label: "知识库",
      sub: kbInfo.value?.absPath ?? "",
      exists: kbInfo.value?.exists ?? false,
      active: kbInfo.value?.active ?? false,
    };
  }
  const p = projects.value.find((x) => x.projectId === s.projectId);
  return {
    label: s.projectName,
    sub: p?.absPath ?? "",
    exists: p?.exists ?? false,
    active: p?.active ?? false,
  };
});

async function refresh() {
  loading.value = true;
  try {
    const [ps, kb] = await Promise.all([
      claudeMd.listProjects(),
      claudeMd.kbInfo(),
    ]);
    projects.value = ps;
    kbInfo.value = kb;
  } finally {
    loading.value = false;
  }
}

async function selectKb() {
  if (dirty.value && !confirm("当前文件有未保存的修改, 切换会丢失。继续?")) return;
  selected.value = { kind: "kb" };
  await loadContent("kb");
}

async function selectProject(p: ProjectClaudeMd) {
  if (dirty.value && !confirm("当前文件有未保存的修改, 切换会丢失。继续?")) return;
  selected.value = {
    kind: "project",
    projectId: p.projectId,
    projectName: p.projectName,
  };
  await loadContent("project", p.projectId);
}

async function loadContent(area: ClaudeMdArea, projectId?: string) {
  message.value = null;
  try {
    const text = await claudeMd.read(area, projectId);
    content.value = text;
    originalContent.value = text;
  } catch (err: any) {
    message.value = { kind: "err", text: `读取失败: ${err}` };
    content.value = "";
    originalContent.value = "";
  }
}

async function save() {
  if (!selected.value || !dirty.value) return;
  saving.value = true;
  message.value = null;
  try {
    if (selected.value.kind === "kb") {
      await claudeMd.write("kb", undefined, content.value);
    } else {
      await claudeMd.write("project", selected.value.projectId, content.value);
    }
    originalContent.value = content.value;
    message.value = { kind: "ok", text: "已保存" };
    await refresh();
  } catch (err: any) {
    message.value = { kind: "err", text: `保存失败: ${err}` };
  } finally {
    saving.value = false;
  }
}

function revert() {
  content.value = originalContent.value;
  message.value = null;
}

function stripMarker() {
  const lines = content.value.split(/\r?\n/);
  while (lines.length && /polaris:placeholder/.test(lines[0])) lines.shift();
  while (lines.length && lines[0].trim() === "") lines.shift();
  content.value = lines.join("\n");
}

function statusBadge(active: boolean, exists: boolean): string {
  if (!exists) return "未创建";
  return active ? "已启用" : "占位";
}

onMounted(refresh);
</script>

<template>
  <div class="cmd-root">
    <div class="cmd-head">
      <div>
        <div class="title">CLAUDE.md · 主上下文</div>
        <div class="sub">
          每个项目一份, 知识库共享一份。每次给项目下的对话发消息前会自动注入,
          但只取「已启用」的(无 <code>polaris:placeholder</code> 标记行)。
        </div>
      </div>
      <button class="btn ghost" @click="refresh" :disabled="loading">
        {{ loading ? "刷新中…" : "重新扫描" }}
      </button>
    </div>

    <div class="cmd-body">
      <!-- Left: list -->
      <aside class="list">
        <div class="grp-head">知识库 · 全局共享</div>
        <button
          class="item"
          :class="{
            active: selected?.kind === 'kb',
            on: kbInfo?.active,
          }"
          @click="selectKb"
          :title="kbInfo?.absPath"
        >
          <span class="dot" :class="{ on: kbInfo?.active }" />
          <span class="rel">PolarisKB</span>
          <span
            class="badge"
            :class="{ on: kbInfo?.active, miss: !kbInfo?.exists }"
          >
            {{ statusBadge(kbInfo?.active ?? false, kbInfo?.exists ?? false) }}
          </span>
        </button>

        <div class="grp-head">项目 · {{ projects.length }}</div>
        <button
          v-for="p in projects"
          :key="p.projectId"
          class="item"
          :class="{
            active:
              selected?.kind === 'project' &&
              selected.projectId === p.projectId,
            on: p.active,
          }"
          @click="selectProject(p)"
          :title="p.absPath"
        >
          <span class="dot" :class="{ on: p.active }" />
          <span class="rel">{{ p.projectName }}</span>
          <span
            class="badge"
            :class="{ on: p.active, miss: !p.exists }"
          >
            {{ statusBadge(p.active, p.exists) }}
          </span>
        </button>

        <div v-if="projects.length === 0 && !loading" class="empty">
          没有项目。请先到左边栏新建项目。
        </div>
      </aside>

      <!-- Right: editor -->
      <section class="editor">
        <div v-if="!selected || !selectedMeta" class="placeholder">
          ← 从左边挑一个
        </div>
        <template v-else>
          <div class="ed-head">
            <div class="ed-path">
              <span class="ed-area">
                {{ selected.kind === "kb" ? "知识库" : "项目" }}
              </span>
              <span class="ed-rel">{{ selectedMeta.label }}</span>
              <span
                v-if="!selectedMeta.exists"
                class="badge miss"
                style="margin-left: 8px"
              >未创建(保存即新建)</span>
            </div>
            <div class="ed-actions">
              <button
                class="btn ghost"
                @click="stripMarker"
                :disabled="!/polaris:placeholder/.test(content)"
                title="一键删掉顶部 polaris:placeholder 行 → 启用"
              >
                启用 (删占位行)
              </button>
              <button class="btn ghost" @click="revert" :disabled="!dirty">
                还原
              </button>
              <button
                class="btn primary"
                @click="save"
                :disabled="!dirty || saving"
              >
                {{ saving ? "保存中…" : dirty ? "保存" : "已保存" }}
              </button>
            </div>
          </div>
          <div class="ed-fullpath" :title="selectedMeta.sub">
            {{ selectedMeta.sub }}
          </div>
          <div v-if="message" class="msg" :class="message.kind">
            {{ message.text }}
          </div>
          <textarea
            v-model="content"
            class="ed-area-input"
            spellcheck="false"
            placeholder="编辑 CLAUDE.md…"
          ></textarea>
        </template>
      </section>
    </div>
  </div>
</template>

<style scoped>
.cmd-root {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: var(--bg);
}

.cmd-head {
  padding: 14px 18px 10px;
  border-bottom: 1px solid var(--border-soft);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.cmd-head .title {
  font-family: var(--serif);
  font-size: 16px;
  letter-spacing: 2px;
  color: var(--ink);
}
.cmd-head .sub {
  font-size: 12px;
  color: var(--muted);
  margin-top: 4px;
}
.cmd-head .sub code {
  font-size: 11.5px;
  background: var(--selection-bg);
  padding: 1px 5px;
  border-radius: 3px;
}

.cmd-body {
  flex: 1;
  display: grid;
  grid-template-columns: 280px 1fr;
  min-height: 0;
}

.list {
  border-right: 1px solid var(--border-soft);
  overflow-y: auto;
  padding: 6px 4px;
}
.grp-head {
  font-family: var(--serif);
  font-size: 11px;
  letter-spacing: 1.5px;
  color: var(--dim);
  padding: 12px 10px 4px;
}
.item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 10px;
  border: none;
  border-radius: 3px;
  background: transparent;
  color: var(--text-2);
  font-size: 13px;
  text-align: left;
}
.item:hover {
  background: var(--selection-bg);
}
.item.active {
  background: var(--selection-bg);
  color: var(--text);
  font-weight: 500;
  border-left: 2px solid var(--ink);
  padding-left: 8px;
}
.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--border);
  flex-shrink: 0;
}
.dot.on {
  background: var(--primary);
}
.rel {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 2px;
  background: var(--border);
  color: var(--muted);
  font-family: var(--serif);
  letter-spacing: 1px;
}
.badge.on {
  background: var(--ink);
  color: #fff;
}
.badge.miss {
  background: transparent;
  border: 1px dashed var(--border);
  color: var(--dim);
}

.empty {
  font-size: 12px;
  color: var(--dim);
  padding: 12px;
  font-style: italic;
}

.editor {
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  font-family: var(--serif);
  letter-spacing: 2px;
}
.ed-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 18px 6px;
}
.ed-path {
  display: flex;
  gap: 10px;
  align-items: baseline;
  flex: 1;
  min-width: 0;
}
.ed-area {
  font-family: var(--serif);
  font-size: 11px;
  letter-spacing: 1.5px;
  color: var(--dim);
}
.ed-rel {
  font-size: 14px;
  color: var(--text);
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ed-fullpath {
  padding: 0 18px 10px;
  font-size: 11px;
  color: var(--dim);
  font-family: ui-monospace, Consolas, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  border-bottom: 1px solid var(--border-soft);
}
.ed-actions {
  display: flex;
  gap: 6px;
}

.btn {
  padding: 5px 12px;
  border-radius: 3px;
  font-size: 12px;
  border: 1px solid var(--border);
  background: var(--panel);
  color: var(--text);
  cursor: pointer;
}
.btn:hover {
  background: var(--selection-bg);
}
.btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.btn.ghost {
  background: transparent;
}
.btn.primary {
  background: var(--ink);
  border-color: var(--ink);
  color: #fff;
}
.btn.primary:hover {
  background: var(--primary);
  border-color: var(--primary);
}

.msg {
  padding: 6px 18px;
  font-size: 12px;
  border-bottom: 1px solid var(--border-soft);
}
.msg.ok {
  color: var(--primary);
  background: var(--selection-bg);
}
.msg.err {
  color: var(--vermilion);
  background: var(--selection-bg);
}

.ed-area-input {
  flex: 1;
  border: none;
  outline: none;
  resize: none;
  padding: 14px 18px;
  font-family: ui-monospace, "JetBrains Mono", Consolas, monospace;
  font-size: 13px;
  line-height: 1.65;
  background: var(--panel);
  color: var(--text);
  tab-size: 2;
}
</style>
