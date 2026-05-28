<script setup lang="ts">
import { computed, ref, watch, onMounted } from "vue";
import Sidebar from "./components/Sidebar.vue";
import ViewLoader from "./components/ViewLoader.vue";
import RightDrawer from "./components/RightDrawer.vue";
import ChatPanel from "./components/ChatPanel.vue";
import WikiBrowse from "./components/WikiBrowse.vue";
import KnowledgeGraph from "./components/KnowledgeGraph.vue";
import SandboxStatus from "./features/sandbox/components/SandboxStatus.vue";
import ClaudeMdPanel from "./components/ClaudeMdPanel.vue";
import Settings from "./components/Settings.vue";
import SkillCenter from "./components/SkillCenter.vue";
import AddProviderModal from "./components/AddProviderModal.vue";
import WorkflowPackModal from "./components/WorkflowPackModal.vue";
import UsageBoard from "./components/UsageBoard.vue";
import SplashScreen from "./components/SplashScreen.vue";
import Onboarding from "./components/Onboarding.vue";
import EnvDoctor from "./components/EnvDoctor.vue";
import { useAppStore, type ViewKey } from "./stores/app";
import { useArtifactsStore } from "./stores/artifacts";
import { useProvidersStore } from "./stores/providers";
import { useChatStore } from "./stores/chat";
import { useWorkflowsStore } from "./stores/workflows";

const app = useAppStore();
const artifacts = useArtifactsStore();
const providers = useProvidersStore();
const chatStore = useChatStore();
const workflows = useWorkflowsStore();

// ─────────── 重视图切换的"点击即缓冲"加载条 ───────────
// 点击图谱/沙箱(且首次=未被 KeepAlive 暖过)时：先立刻亮加载条(此刻重组件尚未挂载，
// 能马上画出来) → 等两帧画出后再挂载重组件(建图 / 9 数字人挂载的卡顿被盖在条下) →
// 组件 ready(图谱布局稳定 / 沙箱画好)后淡出。已暖的重视图直接秒切，不再亮条。
const HEAVY: ViewKey[] = ["graph", "sandbox"];
const warmed = ref<Set<ViewKey>>(new Set());
const mountedView = ref<ViewKey>(app.view); // 真正挂载的视图（重视图冷启时滞后两帧）
const switchLoader = ref<ViewKey | null>(null); // 当前加载条覆盖的重视图
let loaderSafety: number | undefined;

watch(
  () => app.view,
  (next) => {
    if (HEAVY.includes(next) && !warmed.value.has(next)) {
      switchLoader.value = next; // 点击瞬间亮条
      clearTimeout(loaderSafety);
      loaderSafety = window.setTimeout(() => {
        if (switchLoader.value === next) switchLoader.value = null;
      }, 4500); // 兜底：ready 没来也不卡住
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          if (app.view !== next) return; // 这两帧里用户又切走了
          mountedView.value = next; // 现在才挂载重视图
          warmed.value.add(next);
        })
      );
    } else {
      mountedView.value = next;
      switchLoader.value = null;
    }
  }
);

function onViewReady(v: ViewKey) {
  if (switchLoader.value === v) switchLoader.value = null;
}

// 多开核心：app 级注册一次流式监听，任意对话的事件都按 conversationId 路由进各自缓冲，
// 这样切走/未挂载 ChatPanel 时后台任务仍持续流式推进、完成有提醒。
onMounted(() => {
  chatStore.init();
});

// 启动流程：splash(每次) → onboarding(仅首次) → env(环境检测,健康则无感放行) → ready
const ONBOARDED_KEY = "polaris.onboarded.v1";
const phase = ref<"splash" | "onboarding" | "env" | "ready">("splash");

function onSplashDone() {
  const done = localStorage.getItem(ONBOARDED_KEY);
  phase.value = done ? "env" : "onboarding";
}
function onOnboardingDone() {
  phase.value = "env";
}
function onEnvDone() {
  phase.value = "ready";
}

// 预览成品文件时把右侧抽屉拓宽；展开模式更宽，让观看更好看
const drawerTrack = computed(() => {
  if (artifacts.current) {
    return artifacts.expanded ? "min(1040px, 72vw)" : "clamp(400px, 36vw, 560px)";
  }
  return `${app.drawerWidth}px`;
});

const layoutCols = computed(
  () => `${app.sidebarWidth}px 1fr ${drawerTrack.value}`
);
</script>

<template>
  <div class="shell" :style="{ gridTemplateColumns: layoutCols }">
    <Sidebar />
    <main class="main">
      <!-- 重视图(图谱/沙箱)用 KeepAlive 缓存：第一次进算一次，之后切走再回来瞬开，
           且离开时其动画/自转随 DOM 脱离自动暂停，不在后台空耗。其余视图照常按需挂载。
           mountedView 让重视图冷启时滞后两帧挂载，先把加载条画出来再扛卡顿。 -->
      <KeepAlive :include="['KnowledgeGraph', 'SandboxStatus']">
        <ChatPanel v-if="mountedView === 'chat'" />
        <WikiBrowse v-else-if="mountedView === 'wiki'" />
        <KnowledgeGraph
          v-else-if="mountedView === 'graph'"
          @ready="onViewReady('graph')"
        />
        <SandboxStatus
          v-else-if="mountedView === 'sandbox'"
          @ready="onViewReady('sandbox')"
        />
        <ClaudeMdPanel v-else-if="mountedView === 'claude_md'" />
        <SkillCenter v-else-if="mountedView === 'skill_center'" />
        <EnvDoctor v-else-if="mountedView === 'env_doctor'" />
        <Settings v-else-if="mountedView === 'settings'" />
      </KeepAlive>

      <!-- 点击重视图即刻浮现的快速加载条（盖住挂载/建图卡顿） -->
      <Transition name="vl">
        <ViewLoader
          v-if="switchLoader"
          :dark="switchLoader === 'graph'"
          :label="switchLoader === 'graph' ? '星河生成中' : '沙箱加载中'"
        />
      </Transition>
    </main>
    <RightDrawer />

    <AddProviderModal v-if="providers.showAddModal" />
    <WorkflowPackModal v-if="workflows.editorOpen" />
    <UsageBoard v-if="providers.showUsageBoard" />

    <!-- 启动流程覆盖层：splash → onboarding -->
    <Transition name="splash-fade">
      <SplashScreen v-if="phase === 'splash'" @done="onSplashDone" />
    </Transition>
    <Transition name="onboard-fade">
      <Onboarding v-if="phase === 'onboarding'" @done="onOnboardingDone" />
    </Transition>
    <Transition name="onboard-fade">
      <EnvDoctor v-if="phase === 'env'" gate @done="onEnvDone" />
    </Transition>
  </div>
</template>

<style scoped>
.shell {
  height: 100vh;
  display: grid;
  transition: grid-template-columns 180ms ease;
}
.main {
  position: relative;
  height: 100vh;
  overflow: hidden;
  background: var(--bg);
  display: flex;
  flex-direction: column;
}
.placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--muted);
  font-family: var(--serif);
  font-size: 14px;
  letter-spacing: 2px;
}
</style>

<!-- 非 scoped：Transition 类名需作用在子组件根元素上 -->
<style>
.splash-fade-leave-active {
  transition: opacity 0.8s ease;
}
.splash-fade-leave-to {
  opacity: 0;
}
.onboard-fade-enter-active {
  transition: opacity 0.4s ease;
}
.onboard-fade-leave-active {
  transition: opacity 0.45s ease;
}
.onboard-fade-enter-from,
.onboard-fade-leave-to {
  opacity: 0;
}
</style>
