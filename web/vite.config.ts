/// <reference types="vitest/config" />
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
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

/**
 * index.html が読むスタイルシートを、index.html の中に埋め込む（ADR 0038）。
 * 外部のスタイルシートは読み終わるまで描画を止める。そのあいだに JS と wasm の取得が終わると、
 * 「読み込み中」の画面が描かれる前に JS が動き、最初の描画（FCP）が JS の読み込みを待つことになる。
 * 本体の CSS は小さい（gzip 後で数 KB）ので、埋め込んでも HTML の取得はほとんど遅れない。
 */
function inlineEntryCss(): Plugin {
  return {
    name: "inline-entry-css",
    apply: "build",
    enforce: "post",
    generateBundle(_options, bundle) {
      const html = bundle["index.html"];
      if (html?.type !== "asset") return;
      let source = String(html.source);
      for (const [fileName, file] of Object.entries(bundle)) {
        if (file.type !== "asset" || !fileName.endsWith(".css")) continue;
        const link = source.match(new RegExp(`<link rel="stylesheet"[^>]*href="\\./${fileName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}"[^>]*>`));
        if (!link) continue;
        // url() は index.html からの相対になってしまうので、含むときは埋め込まない。
        const css = String(file.source);
        if (css.includes("url(")) continue;
        source = source.replace(link[0], () => `<style>${css}</style>`);
        // ファイルは中身を空にして残す。非同期のチャンク（mermaid の elk など）は本体のチャンクを依存に持ち、
        // 読み込むときに本体の CSS も読み込むので、消すと 404 になる。中身を残すと、同じ規則が後からもう一度当たる。
        file.source = "/* 中身は index.html に埋め込んである */\n";
      }
      html.source = source;
    },
  };
}

export default defineConfig({
  plugins: [tailwindcss(), katexWoff2Only(), inlineEntryCss()],
  // React の代わりに Preact（preact/compat）を使う。ソースと依存（react-markdown）は react から import したままにする。
  resolve: {
    alias: [
      { find: /^react-dom\/client$/, replacement: "preact/compat/client" },
      { find: /^react-dom$/, replacement: "preact/compat" },
      { find: /^react\/jsx-runtime$/, replacement: "preact/compat/jsx-runtime" },
      { find: /^react\/jsx-dev-runtime$/, replacement: "preact/compat/jsx-dev-runtime" },
      { find: /^react$/, replacement: "preact/compat" },
      // 互換の structuredClone の代わりに、ブラウザのものを使う（ADR 0051）。
      { find: /^@ungap\/structured-clone$/, replacement: fileURLToPath(new URL("./src/structuredClone.ts", import.meta.url)) },
    ],
  },
  // 相対のパスで出力し、どのパスに置いても読めるようにする。index.html のパスは CLI が base_path に合わせて書き換える（ADR 0030）。
  base: "./",
  // コードブロックの色分けの Worker（ADR 0046）は、ES モジュールとして出力する。
  worker: {
    format: "es",
  },
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
    // react-markdown も Preact で描くよう、Vite で変換して別名を効かせる。
    server: { deps: { inline: ["react-markdown"] } },
  },
});
