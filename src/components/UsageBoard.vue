<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  X,
  RefreshCw,
  Activity,
  DollarSign,
  Layers,
  Database,
} from "@lucide/vue";
import { useProvidersStore } from "../stores/providers";
import type { TokenBucket } from "../tauri";

const store = useProvidersStore();

type Source = "all" | "pi" | "codex" | "gemini";
type Period = "today" | "week" | "month" | "year";
const source = ref<Source>("all");
const period = ref<Period>("today");

const sources: { key: Source; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "pi", label: "pi 内核" },
  { key: "codex", label: "Codex" },
  { key: "gemini", label: "Gemini" },
];
const periods: { key: Period; label: string }[] = [
  { key: "today", label: "当天" },
  { key: "week", label: "近 7 天" },
  { key: "month", label: "近 30 天" },
  { key: "year", label: "近 1 年" },
];

// 我们的数据全部来自 pi 会话用量落账 (~/Polaris/data/usage.jsonl)。
const hasData = computed(() => source.value === "all" || source.value === "pi");

onMounted(() => {
  store.refreshUsage();
  window.addEventListener("keydown", onEsc);
});
onBeforeUnmount(() => window.removeEventListener("keydown", onEsc));
function onEsc(e: KeyboardEvent) {
  if (e.key === "Escape") store.closeUsage();
}

const bucket = computed<TokenBucket | null>(() => {
  const u = store.usage;
  if (!u || !u.available || !hasData.value) return null;
  return u[period.value];
});
const daily = computed(() => (hasData.value ? store.usage?.daily ?? [] : []));
const dailyMax = computed(() => Math.max(1, ...daily.value.map((d) => d.total)));

function fmtInt(n: number): string {
  return Math.round(n).toLocaleString("en-US");
}
function fmtK(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(1) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "k";
  return String(n);
}
function fmtCost(n: number): string {
  return "$" + n.toFixed(4);
}
function cacheTotal(b: TokenBucket): number {
  return b.cacheRead + b.cacheCreation;
}
</script>

<template>
  <Teleport to="body">
    <div class="ub-overlay" @click="store.closeUsage()">
      <div class="ub" @click.stop>
        <div class="ub-accent" />
        <header class="ub-head">
          <div>
            <div class="ub-title">使用统计</div>
            <div class="ub-sub">查看 AI 模型的使用情况和成本统计 · 数据源 pi 会话用量</div>
          </div>
          <button class="icon-btn" @click="store.closeUsage()"><X :size="18" :stroke-width="1.8" /></button>
        </header>

        <!-- 筛选行 -->
        <div class="ub-filters">
          <div class="src-tabs">
            <button
              v-for="s in sources"
              :key="s.key"
              class="src-tab"
              :class="{ on: source === s.key }"
              @click="source = s.key"
            >
              {{ s.label }}
            </button>
          </div>
          <div class="filter-right">
            <button class="ghost-btn" title="刷新" @click="store.refreshUsage()">
              <RefreshCw :size="13" :stroke-width="1.8" /> 刷新
            </button>
            <div class="period-seg">
              <button
                v-for="pd in periods"
                :key="pd.key"
                :class="{ on: period === pd.key }"
                @click="period = pd.key"
              >
                {{ pd.label }}
              </button>
            </div>
          </div>
        </div>

        <div class="ub-body">
          <template v-if="bucket">
            <!-- 4 卡片 -->
            <div class="cards">
              <div class="card">
                <div class="c-head">
                  <span class="c-lab">总请求数</span>
                  <span class="c-ic blue"><Activity :size="15" :stroke-width="2" /></span>
                </div>
                <div class="c-num">{{ fmtInt(bucket.requests) }}</div>
              </div>

              <div class="card">
                <div class="c-head">
                  <span class="c-lab">总成本<span class="c-est">估算</span></span>
                  <span class="c-ic green"><DollarSign :size="15" :stroke-width="2" /></span>
                </div>
                <div class="c-num">{{ fmtCost(bucket.cost) }}</div>
              </div>

              <div class="card">
                <div class="c-head">
                  <span class="c-lab">总 Token 数</span>
                  <span class="c-ic purple"><Layers :size="15" :stroke-width="2" /></span>
                </div>
                <div class="c-num">{{ fmtInt(bucket.total) }}</div>
                <div class="c-sub">
                  <span>Input</span><b>{{ fmtK(bucket.input) }}</b>
                  <span>Output</span><b>{{ fmtK(bucket.output) }}</b>
                </div>
              </div>

              <div class="card">
                <div class="c-head">
                  <span class="c-lab">缓存 Token</span>
                  <span class="c-ic amber"><Database :size="15" :stroke-width="2" /></span>
                </div>
                <div class="c-num">{{ fmtInt(cacheTotal(bucket)) }}</div>
                <div class="c-sub">
                  <span>创建</span><b>{{ fmtK(bucket.cacheCreation) }}</b>
                  <span>命中</span><b>{{ fmtK(bucket.cacheRead) }}</b>
                </div>
              </div>
            </div>

            <!-- 使用趋势 -->
            <div class="trend">
              <div class="trend-head">
                <span class="sec-title">使用趋势</span>
                <span class="trend-note">近 14 天 · Token</span>
              </div>
              <div class="bars">
                <div
                  v-for="(d, i) in daily"
                  :key="d.date + i"
                  class="bar-col"
                  :title="`${d.label}: ${fmtInt(d.total)} tokens · ${fmtCost(d.cost)}`"
                >
                  <div class="bar-wrap">
                    <div
                      class="bar"
                      :class="{ last: i === daily.length - 1 }"
                      :style="{ height: Math.max(2, (d.total / dailyMax) * 100) + '%' }"
                    />
                  </div>
                  <div class="bar-lab">{{ d.label.slice(-2) }}</div>
                </div>
              </div>
            </div>
          </template>

          <div v-else-if="store.usage && !store.usage.available" class="empty">
            <div class="empty-big">暂无用量数据</div>
            <div class="empty-sub">尚未通过 pi 产生会话</div>
          </div>
          <div v-else class="empty">
            <div class="empty-big">{{ source === "codex" ? "Codex" : "Gemini" }} 暂无数据</div>
            <div class="empty-sub">该看板仅聚合 pi 会话用量；{{ source === "codex" ? "Codex" : "Gemini" }} 用量需对应 CLI 自身记录</div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ub-overlay {
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
@keyframes ov { from { opacity: 0; } }
.ub {
  width: min(860px, 96vw);
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
@keyframes pop { from { opacity: 0; transform: translateY(12px) scale(0.98); } }
.ub-accent {
  height: 3px;
  background: linear-gradient(90deg, var(--primary) 0%, var(--gold) 55%, var(--vermilion) 100%);
}
.ub-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 16px 20px 12px;
}
.ub-title {
  font-family: var(--serif);
  font-size: 19px;
  font-weight: 600;
  color: var(--ink);
  letter-spacing: 1.5px;
}
.ub-sub {
  font-size: 11.5px;
  color: var(--dim);
  margin-top: 3px;
}
.icon-btn {
  border: none;
  background: transparent;
  color: var(--muted);
  width: 30px;
  height: 30px;
  border-radius: 7px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.icon-btn:hover { background: var(--selection-bg); color: var(--text); }

.ub-filters {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 4px 20px 14px;
  flex-wrap: wrap;
}
.src-tabs {
  display: inline-flex;
  gap: 3px;
  padding: 3px;
  background: var(--bg-soft);
  border: 1px solid var(--border-soft);
  border-radius: 10px;
}
.src-tab {
  padding: 6px 14px;
  border: none;
  background: transparent;
  color: var(--text-2);
  font-size: 12.5px;
  border-radius: 7px;
}
.src-tab.on {
  background: var(--primary);
  color: #fff;
  font-weight: 500;
  box-shadow: var(--shadow-sm);
}
.filter-right {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}
.ghost-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--border);
  background: var(--panel);
  color: var(--text-2);
  font-size: 12px;
  padding: 5px 11px;
  border-radius: 8px;
}
.ghost-btn:hover { border-color: var(--primary); color: var(--primary); }
.period-seg {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}
.period-seg button {
  border: none;
  background: var(--panel);
  color: var(--muted);
  font-size: 11.5px;
  padding: 5px 11px;
  border-right: 1px solid var(--border);
}
.period-seg button:last-child { border-right: none; }
.period-seg button.on {
  background: var(--primary-soft);
  color: var(--primary-deep);
  font-weight: 600;
}

.ub-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 20px 22px;
}
.cards {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin-bottom: 22px;
}
.card {
  background: linear-gradient(165deg, var(--panel) 0%, var(--bg-soft) 100%);
  border: 1px solid var(--border-soft);
  border-radius: 13px;
  padding: 15px 15px 14px;
  box-shadow: var(--shadow-sm);
  min-height: 116px;
  display: flex;
  flex-direction: column;
}
.c-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.c-lab {
  font-size: 12px;
  color: var(--text-2);
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.c-est {
  font-size: 9px;
  color: var(--dim);
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  padding: 0 4px;
}
.c-ic {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.c-ic.blue { background: #2c6fff1a; color: #2c6fff; }
.c-ic.green { background: #16a34a1a; color: #16a34a; }
.c-ic.purple { background: #7c5cff1a; color: #7c5cff; }
.c-ic.amber { background: #e8833a1a; color: #e8833a; }
.c-num {
  font-family: var(--mono);
  font-size: 26px;
  font-weight: 700;
  color: var(--ink);
  letter-spacing: -0.5px;
  line-height: 1.1;
}
.c-sub {
  margin-top: auto;
  padding-top: 10px;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 2px 8px;
  font-size: 10.5px;
  color: var(--muted);
}
.c-sub b {
  text-align: right;
  font-family: var(--mono);
  color: var(--text-2);
  font-weight: 600;
}

.trend {
  border-top: 1px solid var(--border-soft);
  padding-top: 16px;
}
.trend-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.sec-title {
  font-family: var(--serif);
  font-size: 13px;
  letter-spacing: 1px;
  color: var(--text-2);
  font-weight: 600;
}
.trend-note {
  font-size: 11px;
  color: var(--dim);
  font-family: var(--mono);
}
.bars {
  display: flex;
  align-items: flex-end;
  gap: 6px;
  height: 120px;
}
.bar-col {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  height: 100%;
}
.bar-wrap {
  flex: 1;
  width: 100%;
  display: flex;
  align-items: flex-end;
  justify-content: center;
}
.bar {
  width: 64%;
  min-height: 2px;
  border-radius: 4px 4px 0 0;
  background: linear-gradient(180deg, var(--primary) 0%, #6f8eb0 130%);
  transition: transform 140ms ease, opacity 140ms ease;
}
.bar-col:hover .bar { transform: scaleY(1.05); opacity: 0.88; }
.bar.last { background: linear-gradient(180deg, var(--gold) 0%, #e9dcb4 130%); }
.bar-lab {
  font-size: 9px;
  color: var(--dim);
  font-family: var(--mono);
}

.empty {
  text-align: center;
  padding: 56px 0;
}
.empty-big {
  font-family: var(--serif);
  font-size: 16px;
  color: var(--text-2);
  letter-spacing: 1px;
}
.empty-sub {
  font-size: 12px;
  color: var(--dim);
  margin-top: 6px;
}
</style>
