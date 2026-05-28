/**
 * 文件拖拽落区 —— 基于 Tauri 原生 drag-drop 事件。
 *
 * 为什么不用 HTML5 的 dragover/drop:浏览器出于安全不暴露真实文件路径,
 * 而后端转换 / 入库需要绝对路径。Tauri 的 `onDragDropEvent` 直接给出 OS 路径。
 *
 * 事件是「窗口级」的,但 App.vue 用 v-if 保证同一时刻只挂载一个主视图,
 * 故每个组件各自订阅 + 在 onUnmounted 退订即可,互不打架;再用 `active`
 * 守卫兜底(按当前视图路由),双保险。
 */
import { ref, onMounted, onUnmounted } from "vue";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { isTauri } from "../tauri";

export interface UseFileDropOptions {
  /** 仅当返回 true 时才响应(通常绑定「当前视图是否为本组件」) */
  active?: () => boolean;
  /** 松手放下文件时回调,paths 为绝对路径数组 */
  onDrop: (paths: string[]) => void;
}

export function useFileDrop(opts: UseFileDropOptions) {
  const isOver = ref(false);
  let unlisten: (() => void) | null = null;

  onMounted(async () => {
    if (!isTauri) return;
    try {
      unlisten = await getCurrentWebview().onDragDropEvent((event) => {
        const payload = event.payload as {
          type: "enter" | "over" | "drop" | "leave";
          paths?: string[];
        };
        const active = opts.active ? opts.active() : true;
        if (!active) {
          isOver.value = false;
          return;
        }
        switch (payload.type) {
          case "enter":
          case "over":
            isOver.value = true;
            break;
          case "leave":
            isOver.value = false;
            break;
          case "drop":
            isOver.value = false;
            if (payload.paths && payload.paths.length) {
              opts.onDrop(payload.paths);
            }
            break;
        }
      });
    } catch {
      // 拿不到 webview(极少数环境)时静默降级,不影响其余功能
    }
  });

  onUnmounted(() => {
    if (unlisten) unlisten();
  });

  return { isOver };
}
