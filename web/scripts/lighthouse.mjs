// example を sqlite-cms serve で配信し、Lighthouse で計測する（ADR 0019）。
// アクセシビリティとベストプラクティスは 100 点でなければ失敗する（ADR 0024）。ほかの項目は計測して記録するだけ。
import { spawn } from "node:child_process";
import { appendFile, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as chromeLauncher from "chrome-launcher";
import lighthouse from "lighthouse";
import { PAGES } from "./pages.mjs";
import { startServer } from "./server.mjs";

const webDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const siteDir = path.resolve(webDir, process.env.SITE_DIR ?? "../example");
const outDir = path.resolve(webDir, process.env.LIGHTHOUSE_OUT ?? "lighthouse-reports");


const CATEGORIES = ["performance", "accessibility", "best-practices", "seo"];

/** 必ず満たす点数（0〜1）。ここにない項目は計測して記録するだけ。 */
const REQUIRED = { accessibility: 1, "best-practices": 1 };

function failingAudits(lhr, category) {
  // 重みが 0 の監査（ソースマップの有無など）は点数に影響しないので、表示しない。
  return lhr.categories[category].auditRefs
    .filter((ref) => ref.weight > 0)
    .map((ref) => lhr.audits[ref.id])
    .filter((audit) => audit.score !== null && audit.score < 1)
    .map((audit) => ({
      title: audit.title,
      selectors: (audit.details?.items ?? []).map((item) => item.node?.selector).filter(Boolean),
    }));
}

const percent = (score) => (score === null || score === undefined ? "-" : String(Math.round(score * 100)));

/** pages を一つの Chrome で順に計測し、レポートを書き出して、点数と必須の項目の失敗を返す。 */
async function measure(origin, pages) {
  const chrome = await chromeLauncher.launch({
    chromeFlags: ["--headless=new", ...(process.env.CI ? ["--no-sandbox"] : [])],
  });
  const results = [];
  /** 項目の日本語の名前（Lighthouse のレポートから取る）。 */
  const titles = {};
  try {
    for (const [name, pathname] of pages) {
      const runner = await lighthouse(origin + pathname, {
        port: chrome.port,
        output: ["html", "json"],
        logLevel: "error",
        onlyCategories: CATEGORIES,
        // レポートと監査の名前を日本語にする。
        locale: "ja",
      });
      const [html, json] = runner.report;
      await writeFile(path.join(outDir, `${name}.report.html`), html);
      await writeFile(path.join(outDir, `${name}.report.json`), json);

      for (const c of CATEGORIES) titles[c] ??= runner.lhr.categories[c]?.title ?? c;
      const scores = Object.fromEntries(CATEGORIES.map((c) => [c, runner.lhr.categories[c]?.score ?? null]));
      const failures = Object.entries(REQUIRED)
        .filter(([category, min]) => (scores[category] ?? 0) < min)
        .map(([category, min]) => ({ category, min, audits: failingAudits(runner.lhr, category) }));
      results.push({ name, pathname, scores, failures });
    }
  } finally {
    chrome.kill();
  }
  return { results, titles };
}

/**
 * LIGHTHOUSE_CONCURRENCY（既定 1）個の子プロセスに、ページを分けて計測させる（ADR 0044）。
 * Lighthouse は一つのプロセスの中で並行して動かせないので、子プロセスごとに別の Chrome を使う。
 * 並行して計測すると CPU を取り合うので、パフォーマンスの点数は一つずつ計測したときより揺れる。
 */
async function measureConcurrently(origin, concurrency) {
  // 重いページが一つの子プロセスに偏らないよう、順に配る。
  const groups = Array.from({ length: concurrency }, (_, i) => PAGES.filter((_, index) => index % concurrency === i));
  const outputs = await Promise.all(
    groups.map(async (pages, i) => {
      const resultFile = path.join(outDir, `.worker-${i}.json`);
      const child = spawn(process.execPath, [fileURLToPath(import.meta.url)], {
        stdio: "inherit",
        env: { ...process.env, LIGHTHOUSE_WORKER: JSON.stringify({ origin, pages, resultFile }) },
      });
      const code = await new Promise((resolve) => child.on("exit", resolve));
      if (code !== 0) throw new Error(`計測の子プロセスが失敗しました（${code}）`);
      const output = JSON.parse(await readFile(resultFile, "utf8"));
      await rm(resultFile);
      return output;
    }),
  );
  const order = new Map(PAGES.map(([name], index) => [name, index]));
  return {
    results: outputs.flatMap((o) => o.results).sort((a, b) => order.get(a.name) - order.get(b.name)),
    titles: Object.assign({}, ...outputs.map((o) => o.titles)),
  };
}

async function main() {
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });

  const concurrency = Math.max(1, Math.min(PAGES.length, Number(process.env.LIGHTHOUSE_CONCURRENCY ?? 1)));
  const server = await startServer(siteDir);
  let measured;
  try {
    measured = concurrency === 1 ? await measure(server.origin, PAGES) : await measureConcurrently(server.origin, concurrency);
  } finally {
    server.stop();
  }
  const { results, titles } = measured;

  const title = (category) => titles[category] ?? category;
  const header = `| ページ | ${CATEGORIES.map(title).join(" | ")} |\n|---|${CATEGORIES.map(() => "---:").join("|")}|`;
  const rows = results.map((r) => `| \`${r.pathname}\` | ${CATEGORIES.map((c) => percent(r.scores[c])).join(" | ")} |`);
  const summary = `## Lighthouse\n\n${header}\n${rows.join("\n")}\n\n必須: ${Object.entries(REQUIRED)
    .map(([c, min]) => `${title(c)} ${percent(min)} 点`)
    .join("、")}\n`;
  await writeFile(path.join(outDir, "summary.md"), summary);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, summary);
  console.log(summary);

  const failed = results.filter((r) => r.failures.length > 0);
  for (const r of failed) {
    for (const f of r.failures) {
      console.error(`✗ ${r.pathname}: ${title(f.category)} が ${percent(r.scores[f.category])} 点（必須 ${percent(f.min)} 点）`);
      for (const audit of f.audits) {
        console.error(`    - ${audit.title}${audit.selectors.length ? `: ${audit.selectors.join(", ")}` : ""}`);
      }
    }
  }
  console.log(`レポート: ${outDir}`);
  if (failed.length > 0) process.exitCode = 1;
}

const worker = process.env.LIGHTHOUSE_WORKER;
if (worker) {
  // 子プロセス：受け持ったページを計測し、結果をファイルに書く。
  const { origin, pages, resultFile } = JSON.parse(worker);
  await writeFile(resultFile, JSON.stringify(await measure(origin, pages)));
} else {
  await main();
}
