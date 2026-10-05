// Core Web Vitals（LCP、INP、CLS）を、ブラウザの性能の記録（PerformanceObserver）から計算する（ADR 0050）。
// 計算の規則は、Chrome の定義（https://web.dev/articles/vitals）と web-vitals のライブラリに合わせる。

/** 「良好」とされる上限（https://web.dev/articles/vitals#core-web-vitals）。これを超えたら失敗にする。 */
export const THRESHOLDS = { lcp: 2500, inp: 200, cls: 0.1 };

/** ページに入れて、性能の記録を集めるスクリプト。集めたものは window.__vitals に入る。 */
export const COLLECTOR = `
window.__vitals = { lcp: [], shifts: [], events: [] };
new PerformanceObserver((list) => {
  for (const e of list.getEntries()) window.__vitals.lcp.push({ startTime: e.startTime });
}).observe({ type: "largest-contentful-paint", buffered: true });
new PerformanceObserver((list) => {
  for (const e of list.getEntries()) window.__vitals.shifts.push({ value: e.value, startTime: e.startTime, hadRecentInput: e.hadRecentInput });
}).observe({ type: "layout-shift", buffered: true });
new PerformanceObserver((list) => {
  for (const e of list.getEntries()) window.__vitals.events.push({ name: e.name, interactionId: e.interactionId, duration: e.duration });
}).observe({ type: "event", buffered: true, durationThreshold: 16 });
`;

/**
 * Largest Contentful Paint（ms）。最後に記録された候補の時刻。ブラウザは、入力の後は候補を記録しない。
 * @param {{ startTime: number }[]} entries
 */
export function lcp(entries) {
  return entries.length === 0 ? null : entries[entries.length - 1].startTime;
}

/**
 * Cumulative Layout Shift。入力の直後のずれを除き、前のずれから 1 秒以内、最初のずれから 5 秒以内のずれを一つのまとまりにして、
 * 合計が最大のまとまりの値を返す（session window）。
 * @param {{ value: number, startTime: number, hadRecentInput: boolean }[]} shifts
 */
export function cls(shifts) {
  let max = 0;
  let current = 0;
  let first = 0;
  let last = 0;
  for (const shift of [...shifts].filter((s) => !s.hadRecentInput).sort((a, b) => a.startTime - b.startTime)) {
    if (current > 0 && shift.startTime - last < 1000 && shift.startTime - first < 5000) {
      current += shift.value;
    } else {
      current = shift.value;
      first = shift.startTime;
    }
    last = shift.startTime;
    max = Math.max(max, current);
  }
  return max;
}

/**
 * Interaction to Next Paint（ms）。操作（interactionId）ごとに、その操作のイベントのうち最も長い時間を取り、
 * 操作の数の 98 パーセンタイルにあたるもの（50 回ごとに、最も長いものを一つずつ除く）を返す。操作がなければ null。
 * @param {{ interactionId: number, duration: number }[]} events
 */
export function inp(events) {
  const byInteraction = new Map();
  for (const event of events) {
    if (!event.interactionId) continue;
    byInteraction.set(event.interactionId, Math.max(byInteraction.get(event.interactionId) ?? 0, event.duration));
  }
  const durations = [...byInteraction.values()].sort((a, b) => b - a);
  if (durations.length === 0) return null;
  return durations[Math.min(durations.length - 1, Math.floor(durations.length / 50))];
}

/**
 * 計測した値を「良好」の上限と比べ、超えたものを返す。値が null（計測できなかった）のも失敗にする。
 * @param {{ lcp: number | null, inp: number | null, cls: number }} metrics
 */
export function failures(metrics) {
  return Object.entries(THRESHOLDS)
    .filter(([name, limit]) => metrics[name] === null || metrics[name] > limit)
    .map(([name, limit]) => ({ name, value: metrics[name], limit }));
}
