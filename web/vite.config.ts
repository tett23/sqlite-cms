/// <reference types="vitest/config" />
import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

/**
 * KaTeX の CSS から woff と ttf のフォントの指定を外し、woff2 だけを残す（ADR 0025）。
 * 対応するブラウザはすべて woff2 を読めるので、ほかの形式はビルドの出力（とバイナリ）を大きくするだけである。
 */
function katexWoff2Only(): Plugin {
  return {
    name: "katex-woff2-only",
    enforce: "pre",
    transform(code, id) {
      if (!/[\\/]katex[\\/]dist[\\/]katex(\.min)?\.css$/.test(id)) return null;
      return { code: code.replace(/,url\([^)]*\.(?:woff|ttf)\) format\("(?:woff|truetype)"\)/g, ""), map: null };
    },
  };
}

export default defineConfig({
  plugins: [react(), tailwindcss(), katexWoff2Only()],
  // 相対のパスで出力し、どのパスに置いても読めるようにする。index.html のパスは CLI が base_path に合わせて書き換える（ADR 0030）。
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    copyPublicDir: false,
    // 非同期で読み込むチャンクのうち、最大は mermaid の elk レイアウト（約 1,460 kB。図で elk を指定したときだけ読み込む、ADR 0025）、
    // 次が Shiki（約 1,070 kB、ADR 0017）。意図しない増加に気付けるよう、上限は今の最大の少し上に置く。
    chunkSizeWarningLimit: 1500,
  },
  test: {
    environment: "node",
  },
});
