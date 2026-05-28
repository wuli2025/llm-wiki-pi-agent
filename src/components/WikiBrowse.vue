<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { marked } from "marked";
import { Upload, LoaderCircle, CheckCircle2, XCircle, X, Trash2 } from "@lucide/vue";
import {
  kb,
  type KbHit,
  artifacts as artifactsApi,
  type ArtifactSearchHit,
} from "../tauri";
import { useAppStore } from "../stores/app";
import { useArtifactsStore } from "../stores/artifacts";
import { useFileDrop } from "../composables/useFileDrop";

const app = useAppStore();
const artifactsStore = useArtifactsStore();

type Tab = "overview" | "browse" | "manage";
const tab = ref<Tab>("browse");
const files = ref<string[]>([]);
const selected = ref<string | null>(null);
const markdown = ref("");
const rendered = computed(() => (markdown.value ? marked.parse(markdown.value) : ""));
const query = ref("");
const hits = ref<KbHit[]>([]);
// 历史对话产物命中（搜索记忆把过往输出文件也算入）
const artHits = ref<ArtifactSearchHit[]>([]);
const rootPath = ref("");
const scanned = ref<number | null>(null);
const ingestPath = ref("");
const ingestMsg = ref("");

onMounted(async () => {
  rootPath.value = await kb.root();
  await refreshList();
});

async function refreshList() {
  try {
    files.value = await kb.list(null);
  } catch (e: any) {
    files.value = [];
  }
}

async function openFile(p: string) {
  selected.value = p;
  try {
    markdown.value = await kb.read(p);
  } catch (e: any) {
    markdown.value = `_(读取失败:${e?.message ?? e})_`;
  }
}

async function doScan() {
  scanned.value = await kb.scan();
  await refreshList();
}

async function doSearch() {
  const q = query.value.trim();
  if (!q) {
    hits.value = [];
    artHits.value = [];
    return;
  }
  [hits.value, artHits.value] = await Promise.all([
    kb.search(q),
    artifactsApi.search(q),
  ]);
}

// 点开历史产物 → 右侧抽屉预览
function openArtifact(path: string) {
  artifactsStore.open(path);
}

async function doIngest() {
  if (!ingestPath.value.trim()) return;
  try {
    const r = await kb.ingest(ingestPath.value.trim());
    ingestMsg.value = `已 ingest → ${r}`;
    await refreshList();
  } catch (e: any) {
    ingestMsg.value = `失败:${e?.message ?? e}`;
  }
}

// 删除单份资料（浏览页每行右侧 ×）
async function doDelete(rel: string) {
  if (!confirm(`删除这份资料？\n${rel}`)) return;
  try {
    await kb.delete(rel);
    if (selected.value === rel) {
      selected.value = null;
      markdown.value = "";
    }
    await refreshList();
  } catch (e: any) {
    alert(`删除失败:${e?.message ?? e}`);
  }
}

// 清空整个资料库（管理页）
const clearMsg = ref("");
async function doClear() {
  if (
    !confirm(
      "确定清空整个资料库吗?\n这会删除包括毛主席资料库在内的全部资料,且不可撤销;清空后重启也不会再自动恢复默认资料。"
    )
  )
    return;
  try {
    const n = await kb.clear();
    clearMsg.value = `已清空,剩余 ${n} 个文件`;
    selected.value = null;
    markdown.value = "";
    await refreshList();
  } catch (e: any) {
    clearMsg.value = `失败:${e?.message ?? e}`;
  }
}

// ─────────── 拖拽上传到知识库 ───────────
interface UploadItem {
  name: string;
  status: "loading" | "ok" | "err";
  detail: string;
}
const uploading = ref<UploadItem[]>([]);

async function onDropFiles(paths: string[]) {
  // 乐观占位（大文件转换 / 复制需要时间，逐个显示进度）
  uploading.value = paths.map((p) => ({
    name: p.split(/[\\/]/).pop() || p,
    status: "loading",
    detail: "",
  }));
  try {
    const res = await kb.uploadFiles(paths);
    uploading.value = res.map((r) => ({
      name: r.name,
      status: r.ok ? "ok" : "err",
      detail: r.ok ? r.relPath : r.message,
    }));
    await refreshList();
  } catch (e: any) {
    uploading.value = uploading.value.map((u) => ({
      ...u,
      status: "err",
      detail: e?.message ?? String(e),
    }));
  }
  // 成功项几秒后淡出，失败项保留以便查看
  window.setTimeout(() => {
    uploading.value = uploading.value.filter((u) => u.status === "err");
  }, 5000);
}

const { isOver: dropOver } = useFileDrop({
  active: () => app.view === "wiki",
  onDrop: onDropFiles,
});
</script>

<template>
  <div class="wiki" :class="{ 'drag-active': dropOver }">
    <!-- 拖拽上传覆盖层 -->
    <div v-if="dropOver" class="kb-drop-overlay">
      <div class="kb-drop-card">
        <Upload :size="34" :stroke-width="1.4" />
        <div class="kb-drop-title">松开以加入知识库</div>
        <div class="kb-drop-sub">
          自动转 Markdown 入库并索引 · 支持 PDF / Word / Excel / PPT / 文本 / 代码
        </div>
      </div>
    </div>

    <!-- 上传进度（逐文件） -->
    <div v-if="uploading.length" class="upload-panel">
      <div class="upload-head">上传到知识库</div>
      <div
        v-for="(u, i) in uploading"
        :key="i"
        class="upload-row"
        :class="u.status"
      >
        <LoaderCircle v-if="u.status === 'loading'" :size="15" class="spin" />
        <CheckCircle2 v-else-if="u.status === 'ok'" :size="15" />
        <XCircle v-else :size="15" />
        <span class="up-name" :title="u.name">{{ u.name }}</span>
        <span class="up-detail" :title="u.detail">{{ u.detail }}</span>
      </div>
    </div>

    <div class="head">
      <div class="title">维基知识库</div>
      <div class="tabs">
        <button
          v-for="t in [
            { k: 'overview', l: '概览' },
            { k: 'browse', l: '浏览' },
            { k: 'manage', l: '管理' },
          ]"
          :key="t.k"
          class="tab"
          :class="{ active: tab === t.k }"
          @click="tab = t.k as Tab"
        >
          {{ t.l }}
        </button>
      </div>
      <div class="root">
        <span class="root-label">KB 根:</span>
        <code>{{ rootPath }}</code>
      </div>
    </div>

    <div v-if="tab === 'overview'" class="body overview">
      <div class="cards">
        <div class="card">
          <div class="card-title">三层目录铁律</div>
          <div class="card-body">
            <code>raw/</code> 只读原始 · <code>output/</code> 撰文 + Lint ·
            <code>wiki/</code> 知识层
          </div>
        </div>
        <div class="card">
          <div class="card-title">KB-first 召回</div>
          <div class="card-body">
            每次发消息前自动 <code>kb_search</code>,关键词加权评分,Top-N
            注入 system prompt
          </div>
        </div>
        <div class="card">
          <div class="card-title">6 模式</div>
          <div class="card-body">
            查询(严/普)· 拆解课件 · Ingest · 撰文 · Lint;
            v0.1 仅启用「普通查询 + ingest」
          </div>
        </div>
      </div>
      <button class="primary-btn" @click="doScan()">扫描索引</button>
      <span v-if="scanned !== null" class="muted">扫描完成,共 {{ scanned }} 个文件</span>
    </div>

    <div v-if="tab === 'browse'" class="body browse">
      <div class="left">
        <div class="search-row">
          <input
            v-model="query"
            placeholder="搜索 KB(标题/正文)"
            @keydown.enter="doSearch"
          />
          <button class="btn" @click="doSearch">搜</button>
        </div>
        <div v-if="hits.length" class="hit-list">
          <div class="section-title">搜索结果</div>
          <div
            v-for="h in hits"
            :key="h.path"
            class="hit"
            @click="openFile(h.path)"
          >
            <div class="hit-title">{{ h.title }}</div>
            <div class="hit-snip">{{ h.snippet }}</div>
            <div class="hit-meta">score {{ h.score.toFixed(1) }} · {{ h.path }}</div>
          </div>
        </div>
        <div v-if="artHits.length" class="hit-list">
          <div class="section-title">历史对话产物</div>
          <div
            v-for="a in artHits"
            :key="a.path"
            class="hit"
            @click="openArtifact(a.path)"
          >
            <div class="hit-title">{{ a.name }}</div>
            <div v-if="a.snippet" class="hit-snip">{{ a.snippet }}</div>
            <div class="hit-meta">产物 · {{ a.kind }} · 点开右栏预览</div>
          </div>
        </div>
        <div class="section-title">所有文件</div>
        <div
          v-for="f in files"
          :key="f"
          class="file"
          :class="{ active: selected === f }"
          @click="openFile(f)"
        >
          <span class="file-name">{{ f }}</span>
          <button
            class="file-del"
            title="删除这份资料"
            @click.stop="doDelete(f)"
          >
            <X :size="13" :stroke-width="2" />
          </button>
        </div>
        <div v-if="files.length === 0" class="muted empty">
          KB 为空 —— 把文件直接拖到本页面即可入库,或在「管理」tab 手动 ingest
        </div>
      </div>
      <div class="right">
        <div v-if="!selected" class="placeholder">
          <div class="ph-glyph">▥</div>
          <div>选择左侧文件浏览</div>
        </div>
        <div v-else class="md" v-html="rendered"></div>
      </div>
    </div>

    <div v-if="tab === 'manage'" class="body manage">
      <div class="card">
        <div class="card-title">Ingest 文件 → KB</div>
        <div class="card-body">
          直接把文件<strong>拖到本页面</strong>即可入库;也可填本机绝对路径手动 ingest。
          自动转 Markdown 入 <code>raw/</code> 并索引 —— 支持 PDF / Word(docx) /
          Excel(xlsx) / PPT(pptx) / 文本 / 代码;图片等不可转的原样保存。
        </div>
        <div class="ingest-row">
          <input v-model="ingestPath" placeholder="例:D:\案例文件夹\01_xxx.pdf" />
          <button class="primary-btn" @click="doIngest">Ingest</button>
        </div>
        <div v-if="ingestMsg" class="ingest-msg">{{ ingestMsg }}</div>
      </div>
      <div class="card">
        <div class="card-title">索引重建</div>
        <div class="card-body">
          扫描 KB 根下所有 .md 文件,构建内存索引(MVP 不持久化,启动后自动扫描)
        </div>
        <button class="primary-btn" @click="doScan">立即扫描</button>
      </div>
      <div class="card danger-card">
        <div class="card-title">清空资料库</div>
        <div class="card-body">
          删除 <code>raw/</code> 下的<strong>全部资料</strong>(含默认的毛主席资料库),
          保留目录结构。此操作<strong>不可撤销</strong>,且清空后重启不会再自动恢复默认资料。
          也可在「浏览」里逐条点 × 删除单份资料。
        </div>
        <button class="danger-btn" @click="doClear">
          <Trash2 :size="14" :stroke-width="1.8" />
          <span>清空资料库</span>
        </button>
        <span v-if="clearMsg" class="muted clear-msg">{{ clearMsg }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wiki {
  display: flex;
  flex-direction: column;
  height: 100vh;
  position: relative;
}
.head {
  padding: 18px 28px 0;
  border-bottom: 1px solid var(--hairline);
}
.title {
  font-family: var(--serif);
  font-size: 18px;
  letter-spacing: 2px;
  color: var(--ink);
}
.tabs {
  margin-top: 14px;
  display: flex;
  gap: 18px;
}
.tab {
  background: transparent;
  border: none;
  padding: 8px 0;
  color: var(--muted);
  font-size: 13px;
}
.tab.active {
  color: var(--text);
  font-weight: 600;
  border-bottom: 2px solid var(--ink);
}
.root {
  margin-top: 8px;
  font-size: 11px;
  color: var(--muted);
  padding-bottom: 8px;
}
.root-label {
  margin-right: 6px;
}
.root code {
  background: var(--code-bg);
  padding: 1px 6px;
  border-radius: 2px;
  font-family: var(--mono);
}

.body {
  flex: 1;
  overflow: hidden;
  padding: 18px 28px;
}
.body.overview {
  display: flex;
  flex-direction: column;
  gap: 18px;
}
.body.browse {
  display: grid;
  grid-template-columns: 320px 1fr;
  gap: 16px;
  height: calc(100vh - 130px);
}
.body.manage {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.cards {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 14px;
}
.card {
  background: var(--panel);
  border: 1px solid var(--hairline);
  border-radius: 4px;
  padding: 16px 18px;
}
.card-title {
  font-family: var(--serif);
  font-weight: 600;
  font-size: 13.5px;
  color: var(--text);
  margin-bottom: 6px;
}
.card-body {
  font-size: 12.5px;
  color: var(--text-2);
  line-height: 1.7;
}

.primary-btn {
  align-self: flex-start;
  padding: 7px 16px;
  background: var(--ink);
  color: #fafaf7;
  border: none;
  border-radius: 4px;
  font-size: 12.5px;
}
.primary-btn:hover {
  background: var(--primary);
}
.muted {
  color: var(--muted);
  font-size: 12px;
}

.left {
  border: 1px solid var(--hairline);
  border-radius: 4px;
  padding: 10px;
  overflow-y: auto;
  background: var(--panel);
}
.right {
  border: 1px solid var(--hairline);
  border-radius: 4px;
  padding: 22px 28px;
  overflow-y: auto;
  background: var(--panel);
}
.search-row {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}
.search-row input {
  flex: 1;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-size: 12.5px;
  background: var(--bg);
}
.search-row input:focus {
  outline: none;
  border-color: var(--primary);
}
.btn {
  padding: 6px 12px;
  border: 1px solid var(--border);
  background: var(--panel);
  border-radius: 3px;
  font-size: 12.5px;
}
.btn:hover {
  border-color: var(--primary);
}

.section-title {
  font-family: var(--serif);
  font-size: 11px;
  letter-spacing: 1.5px;
  color: var(--dim);
  padding: 8px 4px 4px;
}
.hit-list {
  margin-bottom: 10px;
}
.hit {
  padding: 8px 10px;
  border-radius: 3px;
  cursor: pointer;
  margin-bottom: 2px;
}
.hit:hover {
  background: var(--selection-bg);
}
.hit-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.hit-snip {
  font-size: 11.5px;
  color: var(--muted);
  margin-top: 2px;
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.hit-meta {
  font-size: 10.5px;
  color: var(--dim);
  margin-top: 2px;
  font-family: var(--mono);
}

.file {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 6px 5px 10px;
  font-size: 12.5px;
  color: var(--text-2);
  border-radius: 3px;
  cursor: pointer;
  font-family: var(--mono);
}
.file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.file:hover {
  background: var(--selection-bg);
  color: var(--text);
}
.file.active {
  background: var(--selection-bg);
  color: var(--ink);
  font-weight: 500;
}
.file-del {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  border: none;
  background: transparent;
  color: var(--dim);
  border-radius: 4px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.12s, background 0.12s, color 0.12s;
}
.file:hover .file-del {
  opacity: 1;
}
.file-del:hover {
  background: var(--vermilion-soft);
  color: var(--vermilion);
}

/* 清空资料库 —— 危险操作卡片 */
.danger-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  border-color: rgba(192, 57, 43, 0.25);
}
.danger-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  align-self: flex-start;
  margin-top: 12px;
  padding: 7px 14px;
  background: var(--vermilion);
  color: #fff;
  border: none;
  border-radius: 4px;
  font-size: 12.5px;
  cursor: pointer;
}
.danger-btn:hover {
  opacity: 0.9;
}
.clear-msg {
  margin-top: 8px;
}

.placeholder {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--dim);
  font-family: var(--serif);
  letter-spacing: 1px;
}
.ph-glyph {
  font-size: 40px;
  margin-bottom: 12px;
  color: var(--border-strong);
}

.md {
  font-size: 13.5px;
  line-height: 1.85;
  color: var(--text);
}
.md :deep(h1),
.md :deep(h2),
.md :deep(h3) {
  font-family: var(--serif);
  letter-spacing: 1px;
}
.md :deep(h1) {
  font-size: 22px;
  margin-top: 0;
}
.md :deep(h2) {
  font-size: 17px;
  border-bottom: 1px solid var(--hairline);
  padding-bottom: 6px;
}
.md :deep(code) {
  background: var(--code-bg);
  padding: 1.5px 6px;
  border-radius: 2px;
  font-family: var(--mono);
  font-size: 12px;
}
.md :deep(pre) {
  background: var(--bg-soft);
  border: 1px solid var(--hairline);
  padding: 14px 16px;
  border-radius: 3px;
  overflow-x: auto;
}
.md :deep(blockquote) {
  border-left: 2px solid var(--ink);
  padding-left: 14px;
  color: var(--text-2);
  margin-left: 0;
}
.md :deep(a) {
  color: var(--primary);
}

.ingest-row {
  display: flex;
  gap: 6px;
  margin-top: 12px;
}
.ingest-row input {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-size: 12.5px;
  background: var(--bg);
  font-family: var(--mono);
}
.ingest-msg {
  margin-top: 8px;
  font-size: 12px;
  color: var(--muted);
}
.empty {
  padding: 20px 8px;
  font-style: italic;
}

/* ─────────── 拖拽上传覆盖层 ─────────── */
.kb-drop-overlay {
  position: absolute;
  inset: 10px;
  z-index: 50;
  background: rgba(44, 70, 97, 0.07);
  border: 2px dashed var(--primary);
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(1px);
  pointer-events: none;
}
.kb-drop-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  color: var(--primary);
  text-align: center;
  padding: 0 24px;
}
.kb-drop-title {
  font-family: var(--serif);
  font-size: 18px;
  font-weight: 600;
  letter-spacing: 1px;
}
.kb-drop-sub {
  font-size: 12.5px;
  color: var(--muted);
}

/* ─────────── 上传进度面板 ─────────── */
.upload-panel {
  position: absolute;
  right: 18px;
  bottom: 18px;
  z-index: 40;
  width: 320px;
  max-height: 50vh;
  overflow-y: auto;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  padding: 10px 12px;
}
.upload-head {
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 8px;
}
.upload-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 12px;
}
.upload-row.loading {
  color: var(--muted);
}
.upload-row.ok {
  color: #2f9e44;
}
.upload-row.err {
  color: var(--vermilion);
}
.up-name {
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 130px;
}
.up-detail {
  color: var(--dim);
  font-size: 11px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}
.spin {
  animation: spin 0.9s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
