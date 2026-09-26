// コードブロックの色分けを、メインスレッドとは別のスレッドで行う（ADR 0046）。
// Shiki の正規表現のコンパイルと照合は重く（TypeScript の文法では 1 行で数百ミリ秒になることがある）、
// メインスレッドで行うと、そのあいだ入力に応えられない（Total Blocking Time）。
import { highlightBlock, initHighlighter } from "./highlight";

export interface HighlightRequest {
  id: number;
  code: string;
  languageClass: string;
}

const ready = initHighlighter();

self.onmessage = async (event: MessageEvent<HighlightRequest>) => {
  const { id, code, languageClass } = event.data;
  try {
    await ready;
    self.postMessage({ id, block: highlightBlock(code, languageClass) });
  } catch (error) {
    self.postMessage({ id, error: error instanceof Error ? error.message : String(error) });
  }
};
