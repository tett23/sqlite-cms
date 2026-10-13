// E2E の場面を動かす（ADR 0044、0071）。e2e.mjs から切り出し、やり直しの決まりを単体テストで確かめる。
import { isUnresponsive } from "./cdp.mjs";

/**
 * 一つの場面を、新しいタブで動かす。timeout を過ぎても終わらなければ失敗にする。
 * Chrome が命令に応えなかった（isUnresponsive）ときだけ、新しいタブで一度やり直す（ADR 0071）。ほかの失敗はやり直さない。
 * openTab は新しいタブを開く関数、args はタブを受け取って場面に渡す引数を作る関数。
 */
export async function runScenario({ name, fn }, { openTab, args, timeout }) {
  const start = Date.now();
  const once = async () => {
    let tab;
    let timer;
    try {
      tab = await openTab();
      await Promise.race([
        fn(args(tab)),
        new Promise((_, reject) => {
          timer = setTimeout(() => reject(new Error(`${timeout / 1000} 秒で終わらない`)), timeout);
        }),
      ]);
    } finally {
      clearTimeout(timer);
      await tab?.close().catch(() => {});
    }
  };
  try {
    await once();
    return { name, ok: true, ms: Date.now() - start };
  } catch (first) {
    if (!isUnresponsive(first)) return { name, ok: false, ms: Date.now() - start, error: first.message };
    try {
      await once();
      return { name, ok: true, ms: Date.now() - start, retried: first.message };
    } catch (second) {
      return { name, ok: false, ms: Date.now() - start, error: second.message, retried: first.message };
    }
  }
}

/** 結果の一行（失敗とやり直しの理由を含む）。 */
export function formatResult(result) {
  const head = `${result.ok ? "✓" : "✗"} ${result.name}（${(result.ms / 1000).toFixed(1)} 秒）`;
  const retried = result.retried === undefined ? [] : [`${result.ok ? "（Chrome が応えず、やり直して通った）" : "（Chrome が応えず、やり直しても失敗した）"}`, `    1 回目: ${result.retried}`];
  const error = result.ok ? [] : [`    ${result.error}`];
  return [head + (retried[0] ?? ""), ...retried.slice(1), ...error].join("\n");
}
