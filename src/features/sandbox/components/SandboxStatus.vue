<script setup lang="ts">
import { ref, onMounted, onActivated } from "vue";
import {
  sandbox,
  cube,
  type SandboxStatus,
  type CubeConfig,
  type CubeStatus,
} from "../api";
import Workstations from "../../coworker/components/Workstations.vue";

// KeepAlive 的 include 按组件 name 匹配 → 显式命名，确保本视图被缓存
defineOptions({ name: "SandboxStatus" });
// 挂载/状态就绪时通知 App 收起加载条
const emit = defineEmits<{ ready: [] }>();

const status = ref<SandboxStatus | null>(null);
const log = ref<string[]>([]);
const busy = ref(false);
const execCmd = ref("pi --version");
const execOut = ref("");

// CubeSandbox (E2B) 后端配置
const cubeCfg = ref<CubeConfig>({ backend: "docker", endpoint: "", apiKey: "" });
const cubeStat = ref<CubeStatus | null>(null);
const cubeBusy = ref(false);

async function refresh() {
  try {
    status.value = await sandbox.status();
  } catch (e: any) {
    log.value.unshift(`[refresh] ${e?.message ?? e}`);
  }
}
async function loadCube() {
  try {
    cubeCfg.value = await cube.configGet();
    cubeStat.value = await cube.status();
  } catch (e: any) {
    log.value.unshift(`[cube] ${e?.message ?? e}`);
  }
}
async function saveCube() {
  cubeBusy.value = true;
  try {
    cubeCfg.value = await cube.configSet(cubeCfg.value);
    cubeStat.value = await cube.status();
    log.value.unshift(`[cube] 已保存 · backend=${cubeCfg.value.backend}`);
  } catch (e: any) {
    log.value.unshift(`[cube error] ${e?.message ?? e}`);
  } finally {
    cubeBusy.value = false;
  }
}
async function testCube() {
  cubeBusy.value = true;
  try {
    cubeStat.value = await cube.status();
  } finally {
    cubeBusy.value = false;
  }
}
onMounted(() => {
  // 不 await：状态异步填入即可，不被 docker 慢查询拖住
  refresh();
  loadCube();
  // 等数字人 / 状态卡片画出来后再通知 App 收起加载条（遮住挂载那一下的卡顿）
  setTimeout(() => emit("ready"), 420);
});

// KeepAlive：缓存后切回不重新挂载（瞬开）；顺手异步刷新状态保持新鲜，不阻塞
onActivated(() => {
  refresh();
});

async function build() {
  busy.value = true;
  log.value.unshift("[build] 开始构建镜像 polaris-sandbox:alpine …");
  try {
    const out = await sandbox.build();
    log.value.unshift(out);
  } catch (e: any) {
    log.value.unshift(`[build error] ${e?.message ?? e}`);
  } finally {
    busy.value = false;
    await refresh();
  }
}
async function start() {
  busy.value = true;
  try {
    log.value.unshift(await sandbox.start());
  } catch (e: any) {
    log.value.unshift(`[start error] ${e?.message ?? e}`);
  } finally {
    busy.value = false;
    await refresh();
  }
}
async function stop() {
  busy.value = true;
  try {
    log.value.unshift(await sandbox.stop());
  } catch (e: any) {
    log.value.unshift(`[stop error] ${e?.message ?? e}`);
  } finally {
    busy.value = false;
    await refresh();
  }
}
async function runCmd() {
  if (!execCmd.value.trim()) return;
  busy.value = true;
  execOut.value = "(running …)";
  try {
    execOut.value = await sandbox.exec(execCmd.value);
  } catch (e: any) {
    execOut.value = `[error] ${e?.message ?? e}`;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="sandbox">
    <div class="head">
      <div class="title">安全沙箱层</div>
      <div class="sub">
        基于 <code>node:22-alpine</code> 的极简镜像 · 内置轻量内核
        <code>pi</code> (npm 全局装) · 在容器内隔离执行
      </div>
    </div>

    <!-- 协作工位：9 个古风数字人，有任务执笔工作，无任务摸鱼 -->
    <Workstations />

    <!-- CubeSandbox (E2B) 后端：替换 Docker 方案的可选后端 -->
    <div class="cube">
      <div class="cube-head">
        <span class="cube-title">沙箱后端 · CubeSandbox (E2B)</span>
        <span
          class="cube-badge"
          :class="{
            on: cubeStat?.reachable,
            warn: cubeStat?.configured && !cubeStat?.reachable,
          }"
        >
          {{
            !cubeStat?.configured
              ? "未配置"
              : cubeStat?.reachable
              ? "已连通"
              : "不可达"
          }}
        </span>
      </div>
      <div class="cube-desc">
        CubeSandbox 是腾讯云基于 RustVMM+KVM 的微虚机沙箱，兼容 E2B。它需运行在
        <strong>Linux/KVM</strong>（远程主机 / WSL2 / 云），把端点 URL 填到这里即可把执行从
        Docker 切到 CubeSandbox。
      </div>
      <div class="cube-row">
        <label>后端</label>
        <select v-model="cubeCfg.backend">
          <option value="docker">Docker（本机，默认）</option>
          <option value="e2b">CubeSandbox / E2B（端点）</option>
        </select>
      </div>
      <div class="cube-row">
        <label>端点 URL</label>
        <input
          v-model="cubeCfg.endpoint"
          placeholder="https://your-cubesandbox-host:port"
        />
      </div>
      <div class="cube-row">
        <label>API Key</label>
        <input v-model="cubeCfg.apiKey" placeholder="可空（E2B_API_KEY）" />
      </div>
      <div class="cube-actions">
        <button class="primary-btn" :disabled="cubeBusy" @click="saveCube">
          保存
        </button>
        <button class="secondary-btn" :disabled="cubeBusy" @click="testCube">
          测试连接
        </button>
      </div>
      <div v-if="cubeStat?.note" class="cube-note">▸ {{ cubeStat.note }}</div>
    </div>

    <div class="status-grid">
      <div class="stat-card" :class="{ ok: status?.docker_installed }">
        <div class="stat-label">Docker CLI</div>
        <div class="stat-value">
          {{ status?.docker_installed ? "已安装" : "未安装" }}
        </div>
      </div>
      <div class="stat-card" :class="{ ok: status?.docker_running }">
        <div class="stat-label">Docker Daemon</div>
        <div class="stat-value">
          {{ status?.docker_running ? "运行中" : "未运行" }}
        </div>
      </div>
      <div class="stat-card" :class="{ ok: status?.image_built }">
        <div class="stat-label">镜像</div>
        <div class="stat-value">
          {{ status?.image_built ? "已构建" : "未构建" }}
        </div>
        <div class="stat-sub">{{ status?.image_name }}</div>
      </div>
      <div class="stat-card" :class="{ ok: status?.container_running }">
        <div class="stat-label">容器</div>
        <div class="stat-value">
          {{ status?.container_running ? "运行中" : "已停止" }}
        </div>
        <div class="stat-sub">{{ status?.container_name }}</div>
      </div>
    </div>

    <div v-if="status?.notes?.length" class="notes">
      <div v-for="(n, i) in status.notes" :key="i" class="note">▸ {{ n }}</div>
    </div>

    <div class="actions">
      <button class="primary-btn" :disabled="busy" @click="build">
        构建镜像
      </button>
      <button class="primary-btn" :disabled="busy" @click="start">
        启动容器
      </button>
      <button class="secondary-btn" :disabled="busy" @click="stop">
        停止容器
      </button>
      <button class="secondary-btn" @click="refresh">刷新状态</button>
    </div>

    <div class="exec-row">
      <input
        v-model="execCmd"
        placeholder="在沙箱内执行命令(如 pi --version)"
        @keydown.enter="runCmd"
      />
      <button class="primary-btn" :disabled="busy" @click="runCmd">执行</button>
    </div>
    <pre v-if="execOut" class="exec-out">{{ execOut }}</pre>

    <div class="log-head">操作日志</div>
    <div class="log">
      <div v-if="log.length === 0" class="log-empty">(暂无日志)</div>
      <div v-for="(l, i) in log" :key="i" class="log-line">{{ l }}</div>
    </div>
  </div>
</template>

<style scoped>
.sandbox {
  position: relative;
  padding: 22px 30px;
  height: 100vh;
  overflow-y: auto;
  background: var(--bg);
}
.head {
  margin-bottom: 20px;
}
.title {
  font-family: var(--serif);
  font-size: 18px;
  letter-spacing: 2px;
  color: var(--ink);
}
.sub {
  margin-top: 6px;
  color: var(--muted);
  font-size: 12px;
}
.sub code {
  background: var(--code-bg);
  padding: 1px 6px;
  border-radius: 2px;
  font-family: var(--mono);
  font-size: 11px;
}

.cube {
  background: var(--panel);
  border: 1px solid var(--hairline);
  border-left: 2px solid var(--primary);
  border-radius: 6px;
  padding: 16px 18px;
  margin-bottom: 18px;
}
.cube-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.cube-title {
  font-family: var(--serif);
  font-size: 14px;
  letter-spacing: 1px;
  color: var(--ink);
}
.cube-badge {
  font-size: 11px;
  padding: 2px 9px;
  border-radius: 20px;
  background: var(--bg-soft);
  color: var(--muted);
  border: 1px solid var(--border);
}
.cube-badge.on {
  background: #e7f3ec;
  color: #2e7d52;
  border-color: #bfe0cc;
}
.cube-badge.warn {
  background: var(--vermilion-soft);
  color: var(--vermilion);
  border-color: #f0c8c2;
}
.cube-desc {
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.8;
  margin-bottom: 12px;
}
.cube-desc strong {
  color: var(--primary-deep);
}
.cube-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.cube-row label {
  width: 64px;
  font-size: 12px;
  color: var(--muted);
  flex-shrink: 0;
}
.cube-row input,
.cube-row select {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 4px;
  font-size: 12.5px;
  background: var(--bg);
  color: var(--text);
  font-family: var(--mono);
}
.cube-row input:focus,
.cube-row select:focus {
  outline: none;
  border-color: var(--primary);
}
.cube-actions {
  display: flex;
  gap: 8px;
  margin: 12px 0 6px;
}
.cube-note {
  font-size: 11.5px;
  color: var(--text-2);
  background: var(--bg-soft);
  border-radius: 4px;
  padding: 8px 10px;
  line-height: 1.6;
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 14px;
  margin-bottom: 16px;
}
.stat-card {
  background: var(--panel);
  border: 1px solid var(--hairline);
  border-left: 2px solid var(--vermilion);
  border-radius: 3px;
  padding: 14px 16px;
}
.stat-card.ok {
  border-left-color: #4a9c5a;
}
.stat-label {
  font-size: 10.5px;
  color: var(--muted);
  letter-spacing: 1.5px;
  text-transform: uppercase;
  font-family: var(--sans);
}
.stat-value {
  font-family: var(--serif);
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
  margin-top: 6px;
}
.stat-sub {
  font-size: 10.5px;
  color: var(--dim);
  margin-top: 4px;
  font-family: var(--mono);
}

.notes {
  background: var(--bg-soft);
  border: 1px solid var(--hairline);
  border-left: 2px solid var(--gold);
  padding: 10px 14px;
  border-radius: 3px;
  margin-bottom: 18px;
}
.note {
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.7;
}

.actions {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.primary-btn {
  padding: 7px 14px;
  background: var(--ink);
  color: #fafaf7;
  border: none;
  border-radius: 4px;
  font-size: 12.5px;
}
.primary-btn:hover:not(:disabled) {
  background: var(--primary);
}
.primary-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.secondary-btn {
  padding: 7px 14px;
  background: var(--panel);
  color: var(--text-2);
  border: 1px solid var(--border);
  border-radius: 4px;
  font-size: 12.5px;
}
.secondary-btn:hover:not(:disabled) {
  border-color: var(--primary);
  color: var(--primary);
}

.exec-row {
  display: flex;
  gap: 6px;
  margin-bottom: 8px;
}
.exec-row input {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 3px;
  font-size: 12.5px;
  font-family: var(--mono);
  background: var(--bg);
}
.exec-row input:focus {
  outline: none;
  border-color: var(--primary);
}
.exec-out {
  background: var(--bg-soft);
  border: 1px solid var(--hairline);
  padding: 12px 14px;
  border-radius: 3px;
  font-family: var(--mono);
  font-size: 12px;
  color: var(--ink-2);
  white-space: pre-wrap;
  margin-bottom: 16px;
  max-height: 200px;
  overflow-y: auto;
}

.log-head {
  font-family: var(--serif);
  font-size: 11px;
  letter-spacing: 1.5px;
  color: var(--muted);
  text-transform: uppercase;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--hairline);
}
.log {
  margin-top: 8px;
  font-family: var(--mono);
  font-size: 11.5px;
  color: var(--ink-2);
  max-height: 260px;
  overflow-y: auto;
}
.log-line {
  padding: 3px 0;
  white-space: pre-wrap;
}
.log-empty {
  color: var(--dim);
  font-style: italic;
}
</style>
