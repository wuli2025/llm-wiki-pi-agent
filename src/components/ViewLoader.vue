<script setup lang="ts">
/**
 * 视图快速加载条 —— 进入「图谱 / 沙箱」等重视图时盖一层，遮住挂载期的卡顿。
 * 进度条用 transform: scaleX 的 CSS 动画驱动（走合成器线程），
 * 即使主线程被 cytoscape 建图 / 大量 SVG 挂载短暂阻塞，条子依旧丝滑。
 * 父组件用 v-if 控制显隐，外层包 <Transition name="vl"> 即可淡出。
 */
defineProps<{ dark?: boolean; label?: string }>();
</script>

<template>
  <div class="vl" :class="{ dark }">
    <div class="vl-inner">
      <div class="vl-track"><div class="vl-bar"></div></div>
      <div v-if="label" class="vl-label">{{ label }}</div>
    </div>
  </div>
</template>

<style scoped>
.vl {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg);
}
.vl.dark {
  background: #04060e;
}
.vl-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  width: 240px;
  max-width: 60%;
}
.vl-track {
  width: 100%;
  height: 3px;
  border-radius: 3px;
  overflow: hidden;
  background: rgba(44, 70, 97, 0.12);
}
.vl.dark .vl-track {
  background: rgba(150, 180, 255, 0.16);
}
.vl-bar {
  height: 100%;
  width: 100%;
  border-radius: 3px;
  transform-origin: left center;
  background: linear-gradient(90deg, var(--primary), #6fb3ff);
  box-shadow: 0 0 8px rgba(111, 179, 255, 0.5);
  /* ease-out 缓慢逼近 92%（不到顶，像"还在加载"）：快视图早早淡出、慢视图也不显得卡住 */
  animation: vl-fill 1.9s cubic-bezier(0.12, 0.78, 0.25, 1) forwards;
}
.vl.dark .vl-bar {
  background: linear-gradient(90deg, #5fa8e6, #efa838);
  box-shadow: 0 0 10px rgba(239, 168, 56, 0.45);
}
@keyframes vl-fill {
  0% {
    transform: scaleX(0.04);
  }
  100% {
    transform: scaleX(0.92);
  }
}
.vl-label {
  font-family: var(--serif);
  font-size: 11.5px;
  letter-spacing: 2px;
  color: var(--muted);
}
.vl.dark .vl-label {
  color: rgba(190, 208, 240, 0.7);
}
</style>

<!-- 非 scoped：父层 <Transition name="vl"> 的淡出类作用在本组件根元素上。
     进入不做淡入（点击即满显，更跟手），只在离开时淡出。 -->
<style>
.vl-leave-active {
  transition: opacity 0.3s ease;
}
.vl-leave-to {
  opacity: 0;
}
</style>
