<script setup lang="ts">
// 豆包式「发现新版本」中央轻薄对话框：启动自动检测到新版后浮现，
// 点「立即更新」后台下载安装并自动重启；「以后再说」仅关本弹窗（板块仍可更新）。
import { Sparkles, X, LoaderCircle } from "@lucide/vue";
import {
  updateVersion,
  updateNotes,
  updating,
  updateProgress,
  updateError,
  dialogDismissed,
  applyUpdate,
  dismissUpdate,
} from "../composables/useUpdater";
</script>

<template>
  <Transition name="upd-fade">
    <div v-if="updateVersion && !dialogDismissed" class="upd-mask">
      <Transition name="upd-pop" appear>
        <div class="upd-card">
          <button
            v-if="!updating"
            class="upd-x"
            title="以后再说"
            @click="dismissUpdate"
          >
            <X :size="16" :stroke-width="2" />
          </button>

          <div class="upd-badge"><Sparkles :size="22" :stroke-width="1.6" /></div>

          <div class="upd-title">
            发现新版本 <span class="upd-ver">v{{ updateVersion }}</span>
          </div>

          <p v-if="updateError" class="upd-desc err">{{ updateError }}</p>
          <p v-else-if="updating" class="upd-desc">
            正在下载更新… 完成后自动重启生效
          </p>
          <p v-else class="upd-desc">有新内容更新，点击即可立即更新 ✨</p>

          <div v-if="updateNotes && !updating" class="upd-notes">{{ updateNotes }}</div>

          <div v-if="updating" class="upd-bar">
            <div class="upd-bar-fill" :style="{ width: updateProgress + '%' }"></div>
          </div>

          <button class="upd-go" :disabled="updating" @click="applyUpdate">
            <LoaderCircle
              v-if="updating"
              :size="15"
              :stroke-width="2"
              class="spin"
            />
            <span>{{ updating ? `更新中 ${updateProgress}%` : "立即更新" }}</span>
          </button>

          <button
            v-if="!updating"
            class="upd-later"
            @click="dismissUpdate"
          >
            以后再说
          </button>
        </div>
      </Transition>
    </div>
  </Transition>
</template>

<style scoped>
.upd-mask {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(20, 18, 14, 0.18);
  backdrop-filter: blur(2px);
}
.upd-card {
  position: relative;
  width: 332px;
  max-width: calc(100vw - 40px);
  padding: 26px 24px 18px;
  background: var(--panel);
  border: 1px solid var(--border-soft);
  border-radius: 18px;
  box-shadow: var(--shadow-lg);
  text-align: center;
}
.upd-x {
  position: absolute;
  top: 10px;
  right: 10px;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--muted);
  border-radius: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.upd-x:hover {
  background: var(--bg-soft);
  color: var(--text);
}
.upd-badge {
  width: 52px;
  height: 52px;
  margin: 2px auto 14px;
  border-radius: 15px;
  background: var(--primary-soft);
  color: var(--primary);
  display: flex;
  align-items: center;
  justify-content: center;
}
.upd-title {
  font-family: var(--serif);
  font-size: 16.5px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: 0.5px;
}
.upd-ver {
  color: var(--primary);
  font-weight: 700;
}
.upd-desc {
  margin: 8px 0 0;
  font-size: 12.5px;
  color: var(--muted);
  line-height: 1.6;
}
.upd-desc.err {
  color: var(--vermilion);
}
.upd-notes {
  margin: 12px 0 2px;
  max-height: 92px;
  overflow-y: auto;
  padding: 8px 11px;
  background: var(--bg-soft);
  border-radius: 9px;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--text-2);
  text-align: left;
  white-space: pre-wrap;
}
.upd-bar {
  margin: 16px 2px 4px;
  height: 5px;
  border-radius: 3px;
  background: var(--border-soft);
  overflow: hidden;
}
.upd-bar-fill {
  height: 100%;
  background: var(--primary);
  border-radius: 3px;
  transition: width 0.2s ease;
}
.upd-go {
  margin-top: 18px;
  width: 100%;
  padding: 11px 0;
  border: none;
  border-radius: 11px;
  background: var(--ink);
  color: #fafaf7;
  font-size: 13.5px;
  font-weight: 600;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  letter-spacing: 1px;
  transition: background 0.15s, transform 0.1s;
}
.upd-go:hover:not(:disabled) {
  background: var(--primary);
}
.upd-go:active:not(:disabled) {
  transform: scale(0.99);
}
.upd-go:disabled {
  opacity: 0.85;
  cursor: default;
}
.upd-later {
  margin-top: 8px;
  width: 100%;
  padding: 6px 0;
  border: none;
  background: transparent;
  color: var(--muted);
  font-size: 12px;
}
.upd-later:hover {
  color: var(--text);
}
.spin {
  animation: upd-spin 0.9s linear infinite;
}
@keyframes upd-spin {
  to {
    transform: rotate(360deg);
  }
}

/* 遮罩淡入 + 卡片轻弹 */
.upd-fade-enter-active,
.upd-fade-leave-active {
  transition: opacity 0.22s ease;
}
.upd-fade-enter-from,
.upd-fade-leave-to {
  opacity: 0;
}
.upd-pop-enter-active {
  transition: opacity 0.26s ease, transform 0.26s cubic-bezier(0.2, 0.8, 0.2, 1);
}
.upd-pop-enter-from {
  opacity: 0;
  transform: translateY(10px) scale(0.96);
}
</style>
