// Core Web Vitals（LCP、INP、CLS）を、スマートフォンに近い条件の Chrome で計測する（ADR 0050）。
// example を sqlite-cms serve で配信し、計測するページ（pages.mjs）を一つずつ開いて、本物の入力で操作する。
// どれかのページで「良好」の上限（LCP 2.5 秒、INP 200 ms、CLS 0.1）を超えたら失敗する。
//
// Lighthouse の通常の計測（ページを開くだけ）は INP を測れないので、操作まで含めて自前で計測する。
import { appendFile, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchChrome, sleep, Tab } from "./cdp.mjs";
import { PAGES, shard } from "./pages.mjs";
import { startServer } from "./server.mjs";
import { cls, COLLECTOR, failures, inp, lcp, THRESHOLDS } from "./vitals.mjs";

const webDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const siteDir = path.resolve(webDir, process.env.SITE_DIR ?? "../example");
const outDir = path.resolve(webDir, process.env.WEB_VITALS_OUT ?? "web-vitals-reports");
/** 計測するページ。WEB_VITALS_SHARD（"番号/数"）を渡すと、分けたうちの一つだけを計測する（ADR 0052）。 */
const pages = shard(PAGES, process.env.WEB_VITALS_SHARD);

/**
 * 計測の条件。Lighthouse のスマートフォンの計測（moto g power と遅い 4G）と同じ値にする。
 * 通信の遅さは、Lighthouse が DevTools で絞るときの値（往復の遅延を 3.75 倍、帯域を 0.9 倍にしたもの）を使う。
 * CPU は、WEB_VITALS_CPU_THROTTLE（倍率、既定 4）だけ遅くする。
 */
const CONDITIONS = {
  width: 412,
  height: 823,
  deviceScaleFactor: 1.75,
  cpuThrottle: Number(process.env.WEB_VITALS_CPU_THROTTLE ?? 4),
  latency: 562.5,
  downloadThroughput: (1474.56 * 1024) / 8,
  uploadThroughput: (675 * 1024) / 8,
};

/** ページを開いた後、遅れて読み込むもの（色分け、数式、図）が描き終わるのを待つ時間。 */
const SETTLE_MS = 2000;

/** 要素の中央を、本物の入力（CDP の Input）で押す。 */
async function click(tab, selector) {
  const point = await tab.eval(`(() => {
    const element = document.querySelector(${JSON.stringify(selector)});
    if (!element) return null;
    element.scrollIntoView({ block: "center" });
    const rect = element.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
  })()`);
  if (!point) throw new Error(`押す要素がありません: ${selector}`);
  for (const type of ["mousePressed", "mouseReleased"]) {
    await tab.send("Input.dispatchMouseEvent", { type, x: point.x, y: point.y, button: "left", clickCount: 1 });
  }
}

/** 一文字ずつ、本物のキーの入力（keydown、keyup）で打つ。INP は、キーの入力を一つの操作として数える。 */
async function typeKeys(tab, text) {
  for (const char of text) {
    await tab.send("Input.dispatchKeyEvent", { type: "keyDown", key: char, text: char });
    await tab.send("Input.dispatchKeyEvent", { type: "keyUp", key: char });
    await sleep(100);
  }
}

async function pressKey(tab, key, windowsVirtualKeyCode) {
  for (const type of ["keyDown", "keyUp"]) await tab.send("Input.dispatchKeyEvent", { type, key, code: key, windowsVirtualKeyCode });
}

/** 操作：検索ボックスに打ち（候補を描き直す）、Escape で閉じ、ヘッダの「一覧」でページを移る。 */
async function interact(tab) {
  await click(tab, "#header-search");
  await typeKeys(tab, "sql");
  await pressKey(tab, "Escape", 27);
  await sleep(300);
  await click(tab, 'header a[href$="/archive"]');
  await sleep(1000);
}

/** 一つのページを開いて計測する。 */
async function measure(port, origin, pathname) {
  const tab = await Tab.open(port);
  try {
    await tab.send("Network.setCacheDisabled", { cacheDisabled: true });
    await tab.send("Emulation.setDeviceMetricsOverride", {
      width: CONDITIONS.width,
      height: CONDITIONS.height,
      deviceScaleFactor: CONDITIONS.deviceScaleFactor,
      mobile: true,
    });
    await tab.send("Emulation.setCPUThrottlingRate", { rate: CONDITIONS.cpuThrottle });
    await tab.send("Network.emulateNetworkConditions", {
      offline: false,
      latency: CONDITIONS.latency,
      downloadThroughput: CONDITIONS.downloadThroughput,
      uploadThroughput: CONDITIONS.uploadThroughput,
    });
    await tab.send("Page.addScriptToEvaluateOnNewDocument", { source: COLLECTOR });
    await tab.goto(origin + pathname);
    await sleep(SETTLE_MS);
    // LCP は最初の入力で決まるので、操作の前に読む。
    const lcpEntries = await tab.eval("window.__vitals.lcp");

    await interact(tab);

    const { shifts, events } = await tab.eval("({ shifts: window.__vitals.shifts, events: window.__vitals.events })");
    const metrics = { lcp: lcp(lcpEntries), inp: inp(events), cls: cls(shifts) };
    return { pathname, metrics, failures: failures(metrics), problems: tab.problems };
  } finally {
    await tab.close().catch(() => {});
  }
}

/**
 * 計測の前に、計測するページを絞らずに一度ずつ開き、計測と同じ操作をする（ADR 0052）。
 * 起動した直後の Chrome（描画のプロセスやフォントの準備）と機械は遅く、最初に計測するページだけ LCP と INP が悪く出る
 * （CI で、ページを分けた計測の最初のページが LCP 2.58 秒、INP 160 ms になった。ほかのページは 2.0 秒前後）。
 * 開くだけでは、操作で初めて動く JS（検索など）のコンパイルが最初のページの INP に入り、最初のページだけ INP が 130〜280 ms になった
 * （ほかのページは 40〜80 ms。2 ページ目からは、同じプロセスでコンパイルしたものが使い回される）。
 * 計測ではキャッシュを使わないので、ここで開いても、計測するページの読み込みは速くならない。
 */
async function warmUp(port, origin) {
  const tab = await Tab.open(port);
  try {
    for (const [, pathname] of pages) {
      await tab.goto(origin + pathname);
      await sleep(300);
      await interact(tab);
    }
  } finally {
    await tab.close().catch(() => {});
  }
}

const format = {
  lcp: (value) => (value === null ? "-" : `${(value / 1000).toFixed(2)} 秒`),
  inp: (value) => (value === null ? "-" : `${Math.round(value)} ms`),
  cls: (value) => value.toFixed(3),
};

async function main() {
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });
  const server = await startServer(siteDir);
  const chrome = await launchChrome();
  const results = [];
  try {
    await warmUp(chrome.port, server.origin);
    // CPU を取り合うと INP が揺れるので、一つずつ計測する。
    for (const [, pathname] of pages) results.push(await measure(chrome.port, server.origin, pathname));
  } finally {
    chrome.kill();
    server.stop();
  }

  await writeFile(path.join(outDir, "web-vitals.json"), JSON.stringify({ conditions: CONDITIONS, thresholds: THRESHOLDS, results }, null, 2));
  const rows = results.map(
    (r) =>
      `| \`${r.pathname}\` | ${format.lcp(r.metrics.lcp)} | ${format.inp(r.metrics.inp)} | ${format.cls(r.metrics.cls)} | ${r.failures.length === 0 ? "良好" : "✗"} |`,
  );
  const title = process.env.WEB_VITALS_SHARD ? `Core Web Vitals（${process.env.WEB_VITALS_SHARD}）` : "Core Web Vitals";
  const summary = `## ${title}\n\n| ページ | LCP | INP | CLS | 判定 |\n|---|---:|---:|---:|---|\n${rows.join("\n")}\n\n上限: LCP ${format.lcp(THRESHOLDS.lcp)}、INP ${format.inp(THRESHOLDS.inp)}、CLS ${THRESHOLDS.cls}（CPU ${CONDITIONS.cpuThrottle} 倍の遅さ、遅い 4G、412 × 823）\n`;
  await writeFile(path.join(outDir, "summary.md"), summary);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, summary);
  console.log(summary);

  const failed = results.filter((r) => r.failures.length > 0);
  for (const r of failed) {
    for (const f of r.failures) {
      console.error(`✗ ${r.pathname}: ${f.name.toUpperCase()} が ${f.value === null ? "計測できません" : format[f.name](f.value)}（上限 ${format[f.name](f.limit)}）`);
    }
  }
  if (failed.length > 0) process.exitCode = 1;
}

await main();
