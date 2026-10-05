// 数式を、メインスレッドとは別のスレッドで組む（ADR 0053）。
// KaTeX は DOM を使わずに HTML の文字列を作れる。KaTeX の JS（gzip 後で約 75 KB）の読み込みと、数式を組む処理がメインスレッドから外れる。
import { renderMath } from "./katex";

export interface MathRequest {
  id: number;
  tex: string;
  displayMode: boolean;
}

self.onmessage = (event: MessageEvent<MathRequest>) => {
  const { id, tex, displayMode } = event.data;
  self.postMessage({ id, html: renderMath(tex, displayMode) });
};
