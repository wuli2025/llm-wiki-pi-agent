<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { kb, isTauri } from "../tauri";

const emit = defineEmits<{ (e: "done"): void }>();

const step = ref<1 | 2>(1);
const defaultRoot = ref("");
const draft = ref("");
const busy = ref(false);
const error = ref("");

onMounted(async () => {
  try {
    defaultRoot.value = await kb.defaultRoot();
    const cur = await kb.root();
    // 预填：优先当前已解析路径，否则默认
    draft.value = cur || defaultRoot.value;
  } catch {
    /* 浏览器模式下取不到，留空 */
  }
});

async function pickFolder() {
  if (!isTauri) {
    error.value = "浏览器预览模式不支持选择目录，正式应用里可用。";
    return;
  }
  error.value = "";
  const picked = await open({
    directory: true,
    multiple: false,
    title: "选择 Polaris 工作文件夹",
  });
  if (typeof picked === "string" && picked) {
    draft.value = picked;
  }
}

function useDefault() {
  draft.value = defaultRoot.value;
}

async function finish() {
  const v = draft.value.trim();
  if (!v) {
    error.value = "请先选择一个工作文件夹。";
    return;
  }
  busy.value = true;
  error.value = "";
  try {
    await kb.setRoot(v);
    localStorage.setItem("polaris.onboarded.v1", "1");
    emit("done");
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="onboard">
    <div class="card">
      <!-- 顶部北极星徽记 -->
      <div class="badge">
        <span class="star"></span>
      </div>

      <!-- 第一步：欢迎 -->
      <template v-if="step === 1">
        <h1 class="title">欢迎来到北极星 Lite</h1>
        <p class="lead">
          Polaris 是一个本地优先的 AI 工作台。你的对话、知识库与生成的成品，
          都会安放在<strong>你自己的电脑</strong>上一个叫「工作文件夹」的地方。
        </p>
        <p class="lead dim">
          在开始之前，先帮你把这个文件夹安顿好——它是你与北极星一切协作的落脚点。
        </p>
        <div class="actions">
          <button class="btn primary" @click="step = 2">下一步 · 选择工作文件夹</button>
        </div>
      </template>

      <!-- 第二步：选工作文件夹 -->
      <template v-else>
        <h1 class="title">把工作文件夹放在哪里？</h1>
        <p class="lead">
          Polaris 会在这个目录下维护三层结构：
          <code>raw/</code> 原始素材 · <code>output/</code> 生成成品 ·
          <code>wiki/</code> 知识维基。
        </p>
        <ul class="tips">
          <li>建议选一个<strong>容量充足、你会定期备份</strong>的位置。</li>
          <li>可以是网盘 / 同步盘里的目录，方便多台设备共享。</li>
          <li>之后随时能在「设置」里更改，旧目录不会被删除。</li>
        </ul>

        <div class="field-label">
          <span>工作文件夹路径</span>
          <button class="link" @click="useDefault" :disabled="busy">
            用推荐位置
          </button>
        </div>
        <div class="field">
          <input
            v-model="draft"
            class="path"
            :placeholder="defaultRoot || 'C:\\Users\\you\\Polaris\\PolarisKB'"
            :disabled="busy"
          />
          <button class="btn ghost" @click="pickFolder" :disabled="busy">浏览…</button>
        </div>
        <p class="rec" v-if="defaultRoot">
          推荐位置：<code>{{ defaultRoot }}</code>
        </p>

        <p v-if="error" class="err">{{ error }}</p>

        <div class="actions split">
          <button class="btn text" @click="step = 1" :disabled="busy">返回</button>
          <button class="btn primary" @click="finish" :disabled="busy">
            {{ busy ? "正在创建工作文件夹…" : "进入北极星" }}
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.onboard {
  position: fixed;
  inset: 0;
  z-index: 9998;
  display: flex;
  align-items: center;
  justify-content: center;
  background:
    radial-gradient(120% 80% at 50% -10%, #eef2f7 0%, var(--bg) 55%);
  padding: 40px;
}
.card {
  width: 100%;
  max-width: 560px;
  background: var(--panel);
  border: 1px solid var(--hairline);
  border-radius: 6px;
  box-shadow: var(--shadow-lg);
  padding: 42px 46px 38px;
  animation: cardIn 0.5s cubic-bezier(0.2, 0.7, 0.2, 1);
}
@keyframes cardIn {
  from { opacity: 0; transform: translateY(14px); }
  to { opacity: 1; transform: translateY(0); }
}

.badge {
  display: flex;
  justify-content: center;
  margin-bottom: 22px;
}
.star {
  position: relative;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--primary);
  box-shadow:
    0 0 0 4px var(--primary-soft),
    0 0 18px 4px rgba(44, 70, 97, 0.25);
}
.star::before,
.star::after {
  content: "";
  position: absolute;
  left: 50%;
  top: 50%;
  background: linear-gradient(var(--g, to right), transparent, var(--primary), transparent);
}
.star::before {
  width: 46px;
  height: 1.5px;
  transform: translate(-50%, -50%);
}
.star::after {
  width: 1.5px;
  height: 46px;
  transform: translate(-50%, -50%);
}

.title {
  font-family: var(--serif);
  font-size: 23px;
  font-weight: 600;
  letter-spacing: 2px;
  color: var(--ink);
  text-align: center;
  margin: 0 0 18px;
}
.lead {
  font-size: 13.5px;
  line-height: 2;
  color: var(--text-2);
  margin: 0 0 12px;
  letter-spacing: 0.3px;
}
.lead.dim {
  color: var(--muted);
}
.lead strong {
  color: var(--ink);
  font-weight: 600;
}
.tips {
  margin: 4px 0 24px;
  padding-left: 20px;
  font-size: 12.5px;
  line-height: 1.95;
  color: var(--text-2);
}
.tips strong {
  color: var(--primary-deep);
}
code {
  background: var(--code-bg);
  color: var(--code-text);
  padding: 1px 6px;
  border-radius: 3px;
  font-family: var(--mono);
  font-size: 11.5px;
}

.field-label {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-size: 11.5px;
  letter-spacing: 1px;
  color: var(--dim);
  font-family: var(--serif);
  margin-bottom: 6px;
}
.field {
  display: flex;
  gap: 8px;
}
.path {
  flex: 1;
  padding: 9px 11px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-family: var(--mono);
  font-size: 12px;
  background: var(--panel);
  color: var(--text);
}
.path:focus {
  outline: none;
  border-color: var(--primary);
}
.rec {
  font-size: 11.5px;
  color: var(--muted);
  margin: 10px 0 0;
}
.err {
  margin: 14px 0 0;
  padding: 8px 12px;
  border-radius: 3px;
  font-size: 12.5px;
  background: var(--vermilion-soft);
  color: var(--vermilion);
  border-left: 2px solid var(--vermilion);
}

.actions {
  display: flex;
  justify-content: center;
  margin-top: 30px;
}
.actions.split {
  justify-content: space-between;
  align-items: center;
}
.btn {
  padding: 9px 18px;
  border-radius: 3px;
  font-size: 13px;
  letter-spacing: 0.5px;
  border: 1px solid transparent;
}
.btn.primary {
  background: var(--ink);
  color: #fff;
  border-color: var(--ink);
}
.btn.primary:hover:not(:disabled) {
  background: var(--primary);
  border-color: var(--primary);
}
.btn.ghost {
  background: transparent;
  border-color: var(--border);
  color: var(--text-2);
}
.btn.ghost:hover:not(:disabled) {
  border-color: var(--ink);
  color: var(--ink);
}
.btn.text {
  background: transparent;
  color: var(--muted);
}
.btn.text:hover:not(:disabled) {
  color: var(--ink);
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.link {
  background: transparent;
  border: none;
  color: var(--primary);
  font-size: 11.5px;
  cursor: pointer;
  padding: 0;
}
.link:hover:not(:disabled) {
  text-decoration: underline;
}
.link:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
