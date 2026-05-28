import { defineStore } from "pinia";
import { ref } from "vue";
import { artifacts as api, type ArtifactPayload } from "../tauri";

/**
 * 右侧抽屉的「成品预览」状态。
 * - current: 当前正在预览的文件（path + 文件名）
 * - payload: 后端读回的内容（html/图片/文本…）
 * - expanded: 抽屉是否放大（让观看更好看）
 * ChatPanel 点击文件 chip → open(path)；RightDrawer 据此渲染预览。
 */
export const useArtifactsStore = defineStore("artifacts", () => {
  const current = ref<{ path: string; name: string } | null>(null);
  const payload = ref<ArtifactPayload | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const expanded = ref(false);

  async function open(path: string) {
    const name = path.split("/").pop() || path;
    current.value = { path, name };
    loading.value = true;
    error.value = null;
    payload.value = null;
    try {
      payload.value = await api.read(path);
    } catch (e: any) {
      error.value = e?.message ?? String(e);
    } finally {
      loading.value = false;
    }
  }

  async function refresh() {
    if (current.value) await open(current.value.path);
  }

  function close() {
    current.value = null;
    payload.value = null;
    error.value = null;
    expanded.value = false;
  }

  function toggleExpand() {
    expanded.value = !expanded.value;
  }

  async function openExternal() {
    if (current.value) {
      try {
        await api.openExternal(current.value.path);
      } catch (_) {
        /* 忽略：打开失败不影响预览 */
      }
    }
  }

  /** 在系统文件管理器中定位并选中当前预览的文件 */
  async function revealInFolder() {
    if (current.value) {
      try {
        await api.reveal(current.value.path);
      } catch (_) {
        /* 忽略：打开失败不影响预览 */
      }
    }
  }

  return {
    current,
    payload,
    loading,
    error,
    expanded,
    open,
    refresh,
    close,
    toggleExpand,
    openExternal,
    revealInFolder,
  };
});
