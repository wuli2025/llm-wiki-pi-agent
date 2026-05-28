import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// Tauri requires a fixed port; PRD/memory dictates 1420
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "0.0.0.0",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  // 预打包重依赖，避免运行中首次进入「图谱」视图时 Vite 临时优化 + 整页 reload，
  // 那会让 Tauri 误判 beforeDevCommand 退出而整个 dev 栈崩掉。
  optimizeDeps: {
    include: ["cytoscape", "cytoscape-fcose", "marked"],
  },
  build: {
    target: "esnext",
    minify: "esbuild",
    sourcemap: false,
  },
});
