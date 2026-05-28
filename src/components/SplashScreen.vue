<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";

const emit = defineEmits<{ (e: "done"): void }>();

// 三层星点：远(暗小) / 中 / 近(亮大)，box-shadow 铺满视口，营造星河纵深
function genStars(count: number, maxOpacity: number): string {
  const w = Math.max(window.innerWidth, 1600);
  const h = Math.max(window.innerHeight, 1000);
  const parts: string[] = [];
  for (let i = 0; i < count; i++) {
    const x = Math.floor(Math.random() * w);
    const y = Math.floor(Math.random() * h);
    const o = (0.25 + Math.random() * (maxOpacity - 0.25)).toFixed(2);
    parts.push(`${x}px ${y}px rgba(225,232,245,${o})`);
  }
  return parts.join(", ");
}

const farStars = ref(genStars(140, 0.55));
const midStars = ref(genStars(70, 0.8));
const nearStars = ref(genStars(28, 1));

let timer: number | undefined;
let finished = false;

function finish() {
  if (finished) return;
  finished = true;
  emit("done");
}

function onKey() {
  finish();
}

onMounted(() => {
  // 留足时间走完渐入动画 + 停顿，再交给父级淡出
  timer = window.setTimeout(finish, 3200);
  window.addEventListener("keydown", onKey);
});

onBeforeUnmount(() => {
  if (timer) window.clearTimeout(timer);
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <div class="splash" @click="finish" title="点击进入">
    <div class="sky">
      <div class="layer far" :style="{ boxShadow: farStars }"></div>
      <div class="layer mid" :style="{ boxShadow: midStars }"></div>
      <div class="layer near" :style="{ boxShadow: nearStars }"></div>
    </div>

    <div class="aurora"></div>

    <!-- 北极星：发光主星 + 十字光芒 -->
    <div class="polaris">
      <span class="core"></span>
      <span class="ray ray-v"></span>
      <span class="ray ray-h"></span>
      <span class="halo"></span>
    </div>

    <div class="verse">
      <p class="line l1">愿北极星能够照亮你前路的所有黑暗</p>
      <p class="line l2">在混乱的时代坚守本心</p>
    </div>

    <div class="wordmark">北极星 Lite · Polaris</div>
    <div class="hint">点击任意处进入</div>
  </div>
</template>

<style scoped>
.splash {
  position: fixed;
  inset: 0;
  z-index: 9999;
  overflow: hidden;
  background:
    radial-gradient(120% 90% at 50% 8%, #16243a 0%, #0c1320 45%, #070a12 100%);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  user-select: none;
}

/* ── 星场 ── */
.sky {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 1px;
  height: 1px;
  border-radius: 50%;
  background: transparent;
  opacity: 0;
  animation: starsIn 1.6s ease forwards;
}
.layer.far {
  animation-delay: 0.1s;
  animation-name: starsIn, twinkleFar;
  animation-duration: 1.6s, 5.5s;
  animation-iteration-count: 1, infinite;
  animation-timing-function: ease, ease-in-out;
}
.layer.mid {
  animation-delay: 0.25s;
}
.layer.near {
  animation-delay: 0.4s;
  animation-name: starsIn, twinkleNear;
  animation-duration: 1.6s, 3.8s;
  animation-iteration-count: 1, infinite;
  animation-timing-function: ease, ease-in-out;
}
@keyframes starsIn {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes twinkleFar {
  0%, 100% { opacity: 0.85; }
  50% { opacity: 0.55; }
}
@keyframes twinkleNear {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}

/* 顶部极光晕染 */
.aurora {
  position: absolute;
  top: -10%;
  left: 50%;
  width: 70vw;
  height: 40vh;
  transform: translateX(-50%);
  background: radial-gradient(
    50% 60% at 50% 40%,
    rgba(70, 110, 160, 0.22) 0%,
    rgba(40, 70, 110, 0.08) 40%,
    transparent 72%
  );
  filter: blur(10px);
  pointer-events: none;
  opacity: 0;
  animation: starsIn 2s ease 0.3s forwards;
}

/* ── 北极星 ── */
.polaris {
  position: relative;
  width: 10px;
  height: 10px;
  margin-bottom: 54px;
  opacity: 0;
  animation: starPop 1.4s cubic-bezier(0.2, 0.7, 0.2, 1) 0.3s forwards;
}
.core {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: #fff;
  box-shadow:
    0 0 8px 2px rgba(255, 255, 255, 0.95),
    0 0 22px 6px rgba(180, 205, 245, 0.7),
    0 0 60px 18px rgba(120, 160, 220, 0.35);
  animation: corePulse 3.2s ease-in-out infinite;
}
.halo {
  position: absolute;
  left: 50%;
  top: 50%;
  width: 120px;
  height: 120px;
  transform: translate(-50%, -50%);
  border-radius: 50%;
  background: radial-gradient(
    circle,
    rgba(150, 185, 235, 0.18) 0%,
    transparent 65%
  );
}
.ray {
  position: absolute;
  left: 50%;
  top: 50%;
  background: linear-gradient(
    var(--dir, to right),
    transparent,
    rgba(210, 225, 250, 0.85),
    transparent
  );
}
.ray-v {
  width: 1.5px;
  height: 86px;
  transform: translate(-50%, -50%);
  background: linear-gradient(
    to bottom,
    transparent,
    rgba(210, 225, 250, 0.8),
    transparent
  );
}
.ray-h {
  width: 86px;
  height: 1.5px;
  transform: translate(-50%, -50%);
  background: linear-gradient(
    to right,
    transparent,
    rgba(210, 225, 250, 0.8),
    transparent
  );
}
@keyframes starPop {
  0% { opacity: 0; transform: scale(0.2); }
  60% { opacity: 1; }
  100% { opacity: 1; transform: scale(1); }
}
@keyframes corePulse {
  0%, 100% { box-shadow: 0 0 8px 2px rgba(255,255,255,0.95), 0 0 22px 6px rgba(180,205,245,0.7), 0 0 60px 18px rgba(120,160,220,0.35); }
  50% { box-shadow: 0 0 10px 3px rgba(255,255,255,1), 0 0 30px 9px rgba(180,205,245,0.85), 0 0 78px 26px rgba(120,160,220,0.45); }
}

/* ── 箴言 ── */
.verse {
  text-align: center;
  font-family: var(--serif);
}
.line {
  margin: 0;
  color: #e7edf7;
  letter-spacing: 0.32em;
  text-indent: 0.32em;
  font-weight: 400;
  opacity: 0;
  transform: translateY(10px);
  text-shadow: 0 1px 20px rgba(120, 160, 220, 0.3);
}
.line.l1 {
  font-size: 21px;
  line-height: 2;
  animation: verseIn 1.4s cubic-bezier(0.2, 0.7, 0.2, 1) 0.7s forwards;
}
.line.l2 {
  font-size: 18px;
  color: #c8d4e6;
  margin-top: 6px;
  animation: verseIn 1.4s cubic-bezier(0.2, 0.7, 0.2, 1) 1.25s forwards;
}
@keyframes verseIn {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}

.wordmark {
  position: absolute;
  bottom: 64px;
  font-family: var(--serif);
  font-size: 13px;
  letter-spacing: 0.5em;
  text-indent: 0.5em;
  color: rgba(168, 188, 218, 0.7);
  opacity: 0;
  animation: verseIn 1.4s ease 1.9s forwards;
}
.hint {
  position: absolute;
  bottom: 30px;
  font-size: 11px;
  letter-spacing: 0.25em;
  text-indent: 0.25em;
  color: rgba(140, 160, 190, 0.4);
  opacity: 0;
  animation: verseIn 1.2s ease 2.6s forwards;
}
</style>
