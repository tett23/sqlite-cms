// @ungap/structured-clone の代わり（ADR 0051）。mdast-util-to-hast と hast-util-sanitize が使う。
// 対応するブラウザはすべて structuredClone を持つので、互換の実装（約 6 KB）を本体の JS に入れない。
export default function structuredClone<T>(value: T): T {
  return globalThis.structuredClone(value);
}
