import { defineStore } from "pinia";
import { ref, computed } from "vue";

/**
 * 协作会话状态：当前「正在干活」的对话集合。
 * ChatPanel 发送时 start()，完成/出错时 finish()。
 * 沙箱「工位」视图据此让对应数字人进入工作态（其余摸鱼）。
 * 为 P3 多开预留：天然支持多个并发 session。
 */
export interface ActiveSession {
  convId: string;
  title: string;
  startedAt: number;
}

export const useSessionsStore = defineStore("sessions", () => {
  const active = ref<ActiveSession[]>([]);

  function start(convId: string, title: string) {
    if (!convId) return;
    if (active.value.some((s) => s.convId === convId)) return;
    active.value = [
      ...active.value,
      { convId, title: title || "新任务", startedAt: Date.now() },
    ];
  }

  function finish(convId: string) {
    if (!convId) return;
    active.value = active.value.filter((s) => s.convId !== convId);
  }

  const busyCount = computed(() => active.value.length);
  const isBusy = computed(() => active.value.length > 0);

  return { active, start, finish, busyCount, isBusy };
});
