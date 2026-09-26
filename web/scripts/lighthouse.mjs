// example を sqlite-cms serve で配信し、Lighthouse で計測する（ADR 0019）。
// アクセシビリティとベストプラクティスは 100 点でなければ失敗する（ADR 0024）。ほかの項目は計測して記録するだけ。
import { spawn } from "node:child_process";
import { appendFile, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import * as chromeLauncher from "chrome-launcher";
import lighthouse from "lighthouse";

const webDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const siteDir = path.resolve(webDir, process.env.SITE_DIR ?? "../example");
const outDir = path.resolve(webDir, process.env.LIGHTHOUSE_OUT ?? "lighthouse-reports");

const PAGES = [
  ["top", "/"],
  ["article-syntax", "/articles/syntax"],
  ["article-extensions", "/articles/extensions"],
  ["article-getting-started", "/articles/getting-started"],
  ["post-ruby", "/posts/ruby"],
  ["about", "/about"],
];

const CATEGORIES = ["performance", "accessibility", "best-practices", "seo"];

/** 必ず満たす点数（0〜1）。ここにない項目は計測して記録するだけ。 */
const REQUIRED = { accessibility: 1, "best-practices": 1 };

// SQLITE_CMS_BIN を指定すると、そのバイナリ（リリースのビルドなど）で配信する。なければ cargo run で起動する。
function startServer() {
  // 自動反映の接続（Server-Sent Events）が開いたままだと計測の終わりを待たせるので、使わない（ADR 0034）。
  const serve = ["serve", siteDir, "--port", "0", "--no-reload"];
  const [command, args] = process.env.SQLITE_CMS_BIN
    ? [process.env.SQLITE_CMS_BIN, serve]
    : ["cargo", ["run", "--quiet", "--manifest-path", path.join(webDir, "../cli/Cargo.toml"), "--", ...serve]];
  const child = spawn(command, args, { stdio: ["ignore", "pipe", "inherit"] });
  return new Promise((resolve, reject) => {
    child.on("exit", (code) => reject(new Error(`sqlite-cms serve が終了しました（${code}）`)));
    createInterface({ input: child.stdout }).once("line", (line) => {
      const origin = line.match(/^(http:\/\/[^/\s]+)/)?.[1];
      if (!origin) return reject(new Error(`URL が出力されていません: ${line}`));
      resolve({ origin, stop: () => child.kill() });
    });
  });
}

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

async function main() {
  await rm(outDir, { recursive: true, force: true });
  await mkdir(outDir, { recursive: true });

  const server = await startServer();
  const chrome = await chromeLauncher.launch({
    chromeFlags: ["--headless=new", ...(process.env.CI ? ["--no-sandbox"] : [])],
  });

  const results = [];
  /** 項目の日本語の名前（Lighthouse のレポートから取る）。 */
  const titles = {};
  try {
    for (const [name, pathname] of PAGES) {
      const runner = await lighthouse(server.origin + pathname, {
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
    server.stop();
  }

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

await main();
