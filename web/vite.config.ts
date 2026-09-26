/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    copyPublicDir: false,
    // 非同期で読み込む Shiki のチャンク（ADR 0017）が約 1,070 kB ある。
    // 意図しない増加に気付けるよう、上限は今の大きさの少し上に置く。
    chunkSizeWarningLimit: 1100,
  },
  test: {
    environment: "node",
  },
});
