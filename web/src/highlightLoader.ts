import type { Options } from "react-markdown";
import { createLoader } from "./lazyLoader";

export interface Highlight {
  rehypePlugins: Options["rehypePlugins"];
}

// Shiki は大きい（gzip 後で約 160 KB）ので、本体とは別のチャンクにして非同期で読み込む。
export const highlightLoader = createLoader<Highlight>(() =>
  import("./highlight").then((module) => ({ rehypePlugins: module.createRehypePlugins() })),
);
