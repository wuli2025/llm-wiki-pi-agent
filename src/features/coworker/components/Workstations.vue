<script setup lang="ts">
import { computed } from "vue";
import { useSessionsStore } from "../stores/sessions";
import DigitalWorker from "./DigitalWorker.vue";

const sessions = useSessionsStore();

const SEATS = 9;
// 前 busyCount 个工位为工作态，并尽量贴上对应任务标题
const seats = computed(() =>
  Array.from({ length: SEATS }, (_, i) => {
    const s = sessions.active[i];
    return {
      idx: i,
      busy: !!s,
      title: s?.title ?? "",
    };
  })
);
</script>

<template>
  <div class="office">
    <div class="office-head">
      <span class="oh-title">协作工位</span>
      <span class="oh-sub">
        {{ sessions.busyCount }} 人执笔 · {{ SEATS - sessions.busyCount }} 人摸鱼
      </span>
    </div>

    <div class="grid">
      <div
        v-for="s in seats"
        :key="s.idx"
        class="seat"
        :class="{ working: s.busy }"
      >
        <div class="seat-stage">
          <DigitalWorker :busy="s.busy" />
        </div>
        <div class="seat-foot">
          <span class="dot" :class="{ on: s.busy }"></span>
          <span class="label" :title="s.title">
            {{ s.busy ? s.title || "工作中…" : "摸鱼" }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.office {
  background:
    radial-gradient(120% 80% at 50% 0%, #f2efe7 0%, var(--bg-soft) 60%);
  border: 1px solid var(--hairline);
  border-radius: 8px;
  padding: 16px 18px 18px;
  margin-bottom: 18px;
}
.office-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 12px;
}
.oh-title {
  font-family: var(--serif);
  font-size: 14px;
  letter-spacing: 2px;
  color: var(--ink);
}
.oh-sub {
  font-size: 11.5px;
  color: var(--muted);
  letter-spacing: 0.5px;
}
.grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}
.seat {
  background: var(--panel);
  border: 1px solid var(--hairline);
  border-radius: 8px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  transition: border-color 0.3s, box-shadow 0.3s;
}
.seat.working {
  border-color: var(--primary);
  box-shadow: 0 0 0 1px var(--primary-soft), 0 4px 16px rgba(44, 70, 97, 0.12);
}
.seat-stage {
  height: 96px;
  background:
    linear-gradient(180deg, #fbfaf6 0%, #f4f2ea 100%);
  border-radius: 6px;
  overflow: hidden;
}
.seat-foot {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 7px;
  padding: 0 2px;
}
.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--dim);
  flex-shrink: 0;
}
.dot.on {
  background: var(--primary);
  box-shadow: 0 0 0 2px var(--primary-soft);
  animation: pulse 1.4s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.45; }
}
.label {
  font-size: 11px;
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.seat.working .label {
  color: var(--primary-deep);
  font-weight: 500;
}
</style>
