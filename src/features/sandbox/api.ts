/**
 * 板块⑤ 安全沙箱 —— 前端 API 切片 (Feature-Sliced Design)
 *
 * 每个 feature 自带 api / types / components。本文件从原 `src/tauri.ts` 的
 * "Sandbox module" 段落抽出。invoke 命令名保持不变 (sandbox_status 等),
 * 故行为与重构前完全一致 —— 这是纯结构调整。
 */
import { invoke } from "../../tauri";

export interface SandboxStatus {
  docker_installed: boolean;
  docker_running: boolean;
  image_built: boolean;
  image_name: string;
  container_running: boolean;
  container_name: string;
  notes: string[];
}

export const sandbox = {
  status: () => invoke<SandboxStatus>("sandbox_status"),
  build: () => invoke<string>("sandbox_build_image"),
  start: () => invoke<string>("sandbox_start"),
  stop: () => invoke<string>("sandbox_stop"),
  exec: (cmd: string) => invoke<string>("sandbox_exec", { cmd }),
};

// ───────────── CubeSandbox (E2B 兼容) 后端 ─────────────
export interface CubeConfig {
  backend: string; // "docker" | "e2b"
  endpoint: string;
  apiKey: string;
}
export interface CubeStatus {
  backend: string;
  endpoint: string;
  configured: boolean;
  reachable: boolean;
  note: string;
}
export const cube = {
  configGet: () => invoke<CubeConfig>("cube_config_get"),
  configSet: (config: CubeConfig) =>
    invoke<CubeConfig>("cube_config_set", { config }),
  status: () => invoke<CubeStatus>("cube_status"),
};
