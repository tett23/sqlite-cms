import type { Options } from "react-markdown";

export interface Highlight {
  rehypePlugins: Options["rehypePlugins"];
}

export interface HighlightLoader {
  /** 読み込み済みなら返す。まだなら null。 */
  loaded(): Highlight | null;
  /** 読み込みを始める（読み込み中なら同じ Promise を返す）。失敗したら次の呼び出しで読み込み直す。 */
  load(): Promise<Highlight>;
}

export function createHighlightLoader(importer: () => Promise<Highlight>): HighlightLoader {
  let loaded: Highlight | null = null;
  let loading: Promise<Highlight> | null = null;
  return {
    loaded: () => loaded,
    load() {
      loading ??= importer().then(
        (highlight) => {
          loaded = highlight;
          return highlight;
        },
        (error: unknown) => {
          loading = null;
          throw error;
        },
      );
      return loading;
    },
  };
}

// Shiki は大きい（gzip 後で約 160 KB）ので、本体とは別のチャンクにして非同期で読み込む。
export const highlightLoader = createHighlightLoader(() =>
  import("./highlight").then((module) => ({ rehypePlugins: module.createRehypePlugins() })),
);
