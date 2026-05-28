<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import {
  X,
  Check,
  Plus,
  Trash2,
  ChevronUp,
  ChevronDown,
  WandSparkles,
  Workflow,
} from "@lucide/vue";
import {
  useWorkflowsStore,
  newStep,
  assemblePack,
  PACK_COLORS,
  type WorkflowStep,
} from "../stores/workflows";

const store = useWorkflowsStore();

const form = reactive({
  id: undefined as string | undefined,
  name: "",
  description: "",
  color: PACK_COLORS[0],
  steps: [] as WorkflowStep[],
});

const localErr = ref<string | null>(null);
const showPreview = ref(false);

const isEdit = computed(() => !!form.id);

function initForm() {
  const t = store.editorTarget;
  localErr.value = null;
  showPreview.value = false;
  if (t) {
    form.id = t.id;
    form.name = t.name;
    form.description = t.description;
    form.color = t.color;
    // 深拷贝环节，编辑期间不动 store
    form.steps = t.steps.map((s) => ({ ...s }));
  } else {
    form.id = undefined;
    form.name = "";
    form.description = "";
    form.color = PACK_COLORS[0];
    form.steps = [
      newStep("角色", ""),
      newStep("任务", ""),
      newStep("输出格式", ""),
    ];
  }
}

onMounted(() => {
  initForm();
  window.addEventListener("keydown", onEsc);
});
onBeforeUnmount(() => window.removeEventListener("keydown", onEsc));
watch(
  () => store.editorTarget,
  () => initForm()
);

function onEsc(e: KeyboardEvent) {
  if (e.key === "Escape") store.closeEditor();
}

function addStep() {
  form.steps.push(newStep("", ""));
}
function removeStep(i: number) {
  form.steps.splice(i, 1);
}
function moveStep(i: number, dir: -1 | 1) {
  const j = i + dir;
  if (j < 0 || j >= form.steps.length) return;
  const [s] = form.steps.splice(i, 1);
  form.steps.splice(j, 0, s);
}

const preview = computed(() => assemblePack({ steps: form.steps }));

function submit() {
  localErr.value = null;
  if (!form.name.trim()) {
    localErr.value = "请填写工作流包名称";
    return;
  }
  if (!form.steps.some((s) => s.content.trim())) {
    localErr.value = "至少填写一个环节的内容";
    return;
  }
  store.savePack({
    id: form.id,
    name: form.name,
    description: form.description,
    color: form.color,
    steps: form.steps,
  });
  store.closeEditor();
}
</script>

<template>
  <Teleport to="body">
    <div class="modal-overlay" @click="store.closeEditor()">
      <div class="modal" @click.stop>
        <div class="m-accent" />
        <header class="m-head">
          <div class="m-title">
            <Workflow :size="17" :stroke-width="1.7" class="m-title-ic" />
            {{ isEdit ? "编辑工作流包" : "新建工作流包" }}
          </div>
          <button class="icon-btn" @click="store.closeEditor()">
            <X :size="17" :stroke-width="1.8" />
          </button>
        </header>

        <div class="m-body">
          <!-- 基本信息 -->
          <div class="f-grid">
            <label class="field">
              <span class="f-lab">名称</span>
              <input v-model="form.name" placeholder="例如：深度调研报告" />
            </label>
            <label class="field">
              <span class="f-lab">强调色</span>
              <div class="swatches">
                <button
                  v-for="c in PACK_COLORS"
                  :key="c"
                  class="swatch"
                  :class="{ on: form.color === c }"
                  :style="{ background: c }"
                  @click="form.color = c"
                >
                  <Check
                    v-if="form.color === c"
                    :size="12"
                    :stroke-width="3"
                  />
                </button>
              </div>
            </label>
          </div>

          <label class="field">
            <span class="f-lab">描述</span>
            <input
              v-model="form.description"
              placeholder="一句话说明这个工作流包的用途（可选）"
            />
          </label>

          <!-- 环节编排 -->
          <div class="steps-head">
            <span class="f-lab">环节编排</span>
            <span class="steps-hint">上下拖动顺序即重新编排 · 点「使用」时按序拼装</span>
          </div>

          <div class="steps">
            <div v-for="(s, i) in form.steps" :key="s.id" class="step">
              <div class="step-bar">
                <span class="step-no">{{ i + 1 }}</span>
                <input
                  v-model="s.label"
                  class="step-label"
                  placeholder="环节标题，如「约束」"
                />
                <div class="step-ops">
                  <button
                    class="op"
                    title="上移"
                    :disabled="i === 0"
                    @click="moveStep(i, -1)"
                  >
                    <ChevronUp :size="14" :stroke-width="2" />
                  </button>
                  <button
                    class="op"
                    title="下移"
                    :disabled="i === form.steps.length - 1"
                    @click="moveStep(i, 1)"
                  >
                    <ChevronDown :size="14" :stroke-width="2" />
                  </button>
                  <button class="op danger" title="删除环节" @click="removeStep(i)">
                    <Trash2 :size="13" :stroke-width="1.9" />
                  </button>
                </div>
              </div>
              <textarea
                v-model="s.content"
                class="step-content"
                rows="3"
                placeholder="该环节的提示词正文…"
              />
            </div>
          </div>

          <button class="add-step" @click="addStep">
            <Plus :size="14" :stroke-width="2.2" /> 添加环节
          </button>

          <!-- 拼装预览 -->
          <div class="preview-head" @click="showPreview = !showPreview">
            <WandSparkles :size="13" :stroke-width="1.8" />
            <span>拼装预览</span>
            <component
              :is="showPreview ? ChevronUp : ChevronDown"
              :size="14"
              :stroke-width="2"
              class="pv-caret"
            />
          </div>
          <pre v-if="showPreview" class="preview">{{
            preview || "（环节都为空，暂无可拼装内容）"
          }}</pre>

          <div v-if="localErr" class="err">{{ localErr }}</div>
        </div>

        <footer class="m-foot">
          <button class="btn-cancel" @click="store.closeEditor()">取消</button>
          <button class="btn-add" @click="submit">
            <Check :size="14" :stroke-width="2.4" />
            {{ isEdit ? "保存修改" : "创建" }}
          </button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 400;
  background: rgba(20, 20, 25, 0.28);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  animation: ov 160ms ease;
}
@keyframes ov {
  from {
    opacity: 0;
  }
}
.modal {
  width: min(680px, 96vw);
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 16px;
  box-shadow: var(--shadow-lg), 0 0 0 1px var(--hairline);
  overflow: hidden;
  animation: pop 200ms cubic-bezier(0.16, 1, 0.3, 1);
}
@keyframes pop {
  from {
    opacity: 0;
    transform: translateY(12px) scale(0.98);
  }
}
.m-accent {
  height: 3px;
  background: linear-gradient(
    90deg,
    var(--primary) 0%,
    var(--gold) 55%,
    var(--vermilion) 100%
  );
}
.m-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 15px 18px 12px;
}
.m-title {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-family: var(--serif);
  font-size: 17px;
  font-weight: 600;
  color: var(--ink);
  letter-spacing: 1.5px;
}
.m-title-ic {
  color: var(--primary);
}
.icon-btn {
  border: none;
  background: transparent;
  color: var(--muted);
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.icon-btn:hover {
  background: var(--selection-bg);
  color: var(--text);
}

.m-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 14px 18px 6px;
}

.f-grid {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 14px;
  align-items: start;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}
.f-lab {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
}
.field input {
  width: 100%;
  padding: 9px 11px;
  border: 1px solid var(--border);
  border-radius: 9px;
  font-size: 13px;
  background: var(--bg-soft);
  color: var(--text);
}
.field input:focus {
  outline: none;
  border-color: var(--primary);
  background: var(--panel);
}

.swatches {
  display: flex;
  gap: 7px;
  padding-top: 1px;
}
.swatch {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  border: 2px solid transparent;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  box-shadow: var(--shadow-sm);
  transition: transform 120ms ease, box-shadow 120ms ease;
}
.swatch:hover {
  transform: translateY(-1px);
}
.swatch.on {
  border-color: var(--panel);
  box-shadow: 0 0 0 2px currentColor;
}

.steps-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  margin: 6px 0 8px;
}
.steps-hint {
  font-size: 11px;
  color: var(--dim);
}

.steps {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.step {
  border: 1px solid var(--border-soft);
  border-radius: 11px;
  background: var(--bg-soft);
  padding: 9px 10px 10px;
  transition: border-color 120ms ease;
}
.step:focus-within {
  border-color: var(--primary);
}
.step-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 7px;
}
.step-no {
  flex-shrink: 0;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--primary-soft);
  color: var(--primary-deep);
  font-size: 11px;
  font-weight: 700;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.step-label {
  flex: 1;
  border: none;
  background: transparent;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text);
  padding: 3px 2px;
}
.step-label:focus {
  outline: none;
}
.step-label::placeholder {
  color: var(--dim);
  font-weight: 400;
}
.step-ops {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
.op {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--muted);
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.op:hover:not(:disabled) {
  background: var(--selection-bg);
  color: var(--text);
}
.op:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}
.op.danger:hover {
  background: var(--vermilion-soft);
  color: var(--vermilion);
}
.step-content {
  width: 100%;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel);
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--text);
  padding: 8px 10px;
  resize: vertical;
  min-height: 56px;
}
.step-content:focus {
  outline: none;
  border-color: var(--primary);
}

.add-step {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-top: 10px;
  padding: 7px 13px;
  border: 1px dashed var(--border-strong);
  background: transparent;
  color: var(--text-2);
  font-size: 12.5px;
  border-radius: 9px;
}
.add-step:hover {
  border-color: var(--primary);
  color: var(--primary);
  border-style: solid;
}

.preview-head {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 16px 0 0;
  color: var(--text-2);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  user-select: none;
}
.preview-head:hover {
  color: var(--primary);
}
.pv-caret {
  color: var(--muted);
}
.preview {
  margin: 8px 0 0;
  padding: 12px 14px;
  background: var(--code-bg);
  border: 1px solid var(--border-soft);
  border-radius: 9px;
  font-family: var(--mono);
  font-size: 12px;
  line-height: 1.65;
  color: var(--code-text);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 220px;
  overflow-y: auto;
}

.err {
  margin: 12px 0 4px;
  font-size: 12px;
  color: var(--vermilion);
  background: var(--vermilion-soft);
  border-radius: 8px;
  padding: 8px 11px;
}

.m-foot {
  display: flex;
  justify-content: flex-end;
  gap: 9px;
  padding: 12px 18px;
  border-top: 1px solid var(--border-soft);
  background: var(--bg-soft);
}
.btn-cancel,
.btn-add {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--border);
  background: var(--panel);
  color: var(--text-2);
  font-size: 13px;
  padding: 8px 18px;
  border-radius: 9px;
}
.btn-cancel:hover {
  background: var(--selection-bg);
}
.btn-add {
  background: var(--ink);
  color: #fff;
  border-color: var(--ink);
}
.btn-add:hover {
  background: var(--primary);
  border-color: var(--primary);
}
</style>
