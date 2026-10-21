import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

// Tauri 前端构建配置：固定端口，不清理终端输出
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "chrome105",
    outDir: "dist",
    assetsDir: "assets",
    emptyOutDir: true,
  },
});
