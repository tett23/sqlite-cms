// ブラウザでの動きを確かめる E2E のテスト（ADR 0044）。
// example を sqlite-cms serve で配信し、ヘッドレスの Chrome を Chrome DevTools Protocol（CDP）で動かす。
// ページの表示、色分け、数式、図、画面の中の移動、ヘッダの検索と候補、カスタムのヘッダと既定のヘッダを確かめる。
import { appendFile, cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as chromeLauncher from "chrome-launcher";
import { sleep, Tab } from "./cdp.mjs";
import { PAGES } from "./pages.mjs";
import { startServer } from "./server.mjs";

const webDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const siteDir = path.resolve(webDir, process.env.SITE_DIR ?? "../example");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertEqual(actual, expected, message) {
  const [a, e] = [JSON.stringify(actual), JSON.stringify(expected)];
  if (a !== e) throw new Error(`${message}: ${a}（期待 ${e}）`);
}

const noProblems = (tab, where) => assertEqual(tab.problems, [], `${where} でエラー`);

/** 一つのシナリオを待つ時間。止まったときに、CI のジョブの制限時間まで待たずに失敗させる。 */
const SCENARIO_TIMEOUT = 120000;

/** Shiki の読み込み。Worker（highlight.worker-….js）が Shiki の本体を含む（ADR 0046）。 */
const SHIKI = /\/highlight\.worker-[^/]*\.js$/;

/** 図や数式を待つ時間。後から読み込むライブラリは、描画の後に手が空くのを待つ（ADR 0038）。 */
const LAZY_TIMEOUT = 20000;

/** 画面の中の要素の数。 */
const count = (tab, selector) => tab.eval(`document.querySelectorAll(${JSON.stringify(selector)}).length`);

const scenarios = [];
const scenario = (name, fn) => scenarios.push({ name, fn });

scenario("計測するすべてのページが、エラーなく描かれ、レイアウトがずれない（数式と図でずれる既知のページを除く）", async ({ tab, origin }) => {
  // 数式と図は、描く前（TeX や図の文字列）と描いた後で高さが変わる（ADR 0040、0044 の未解決の問題）。
  // 図の多い記事は、CI（Linux）では最初の画面に図が入り、ずれる（手元の macOS では入らず、ずれない）。
  const shifting = new Set(["/articles/heavy-math", "/articles/complex-mixed", "/articles/heavy-diagrams"]);
  for (const [, pathname] of PAGES) {
    await tab.goto(origin + pathname);
    assert((await tab.eval("document.title")) !== "", `${pathname} の題名が空`);
    // 後から読み込むライブラリ（色分けなど）が描き終わるのを少し待つ。
    await sleep(1000);
    noProblems(tab, pathname);
    if (shifting.has(pathname)) continue;
    const shift = await tab.eval("window.__layoutShift");
    assert(shift === 0, `${pathname} でレイアウトがずれた（${shift.toFixed(3)}）`);
  }
});

scenario("画面の外のコードブロックは、近くに来てから色を付ける。コードのないページでは Shiki を読まない（ADR 0029、0038）", async ({ tab, origin }) => {
  await tab.goto(origin + "/about");
  await sleep(1500);
  assert(!tab.requests.some((url) => SHIKI.test(url)), "コードのないページで Shiki を読み込んだ");

  await tab.goto(origin + "/articles/heavy-code");
  await tab.waitFor("document.querySelectorAll('pre.shiki').length > 0", { timeout: LAZY_TIMEOUT, message: "最初の画面のコードに色が付かない" });
  // コードのあるページでは読み込みが見えることを確かめる（見えなければ、上の「読み込まない」の確認が意味を持たない）。
  // Worker の中からの通信（wasm）はページの側からは見えないので、Worker のスクリプト（Shiki の本体を含む）で判断する。
  assert(tab.requests.some((url) => SHIKI.test(url)), "コードのあるページで Shiki の読み込みが見えない");
  await tab.waitFor("document.querySelectorAll('pre.shiki').length > 0", { timeout: LAZY_TIMEOUT, message: "最初の画面のコードに色が付かない" });
  await sleep(1000);
  const before = await count(tab, "pre.shiki");
  assert(before < 32, `画面の外のコードにも色が付いている（${before} 個）`);
  // 少しずつ最後まで送り、どのブロックも一度は画面の近くを通るようにする。
  await tab.eval("(async () => { for (let y = 0; y < document.body.scrollHeight; y += 600) { window.scrollTo(0, y); await new Promise((r) => setTimeout(r, 50)); } })()");
  await tab.waitFor("document.querySelectorAll('pre.shiki').length === 32", { timeout: LAZY_TIMEOUT, message: "送っても色が付かないコードがある" });
});

scenario("色分け、数式、図を描き、書き誤りだけを元の文字列のまま出す", async ({ tab, origin }) => {
  // 縦に長い画面にして、すべての要素を画面の近くに置く。
  await tab.size(1200, 30000);
  const expectations = [
    ["/articles/heavy-code", { "pre.shiki": 32 }],
    ["/articles/heavy-math", { ".math-error": 1 }],
    ["/articles/heavy-diagrams", { ".mermaid-diagram": 10, ".diagram-error": 1 }],
    ["/articles/complex-mixed", { "pre.shiki": 8, ".mermaid-diagram": 7, ".math-error": 0, "a.link-card": 2 }],
  ];
  for (const [pathname, counts] of expectations) {
    await tab.goto(origin + pathname);
    for (const [selector, expected] of Object.entries(counts)) {
      await tab.waitFor(`document.querySelectorAll(${JSON.stringify(selector)}).length === ${expected}`, {
        timeout: LAZY_TIMEOUT,
        message: `${pathname} の ${selector} が ${expected} 個にならない（${await count(tab, selector)} 個）`,
      });
    }
  }
  await tab.goto(origin + "/articles/heavy-math");
  await tab.waitFor("document.querySelectorAll('.katex').length > 90", { timeout: LAZY_TIMEOUT, message: "数式が描かれない" });
  await tab.size(1024, 800);
});

scenario("リンクカードは画面の近くに来てから部品を読み込み、画像はカードの大きさの WebP で配信する（ADR 0049）", async ({ tab, origin }) => {
  // カードの画像は loading="lazy" なので、縦に長い画面にして、どのカードも画面の中に置く。
  await tab.size(1200, 30000);
  await tab.goto(origin + "/articles/complex-mixed");
  await tab.waitFor("document.querySelectorAll('a.link-card').length === 2", { timeout: LAZY_TIMEOUT, message: "カードが 2 枚描かれない" });
  assert(tab.requests.some((url) => /\/LinkCard-[^/]*\.js$/.test(url)), "カードの部品を読み込んでいない");
  await tab.waitFor("[...document.querySelectorAll('img.link-card-image')].every((img) => img.complete && img.naturalWidth > 0)", {
    message: "カードの画像が読み込まれない",
  });
  assertEqual(
    await tab.eval("[...document.querySelectorAll('img.link-card-image')].map((img) => [img.getAttribute('src').endsWith('.webp'), img.naturalWidth, img.naturalHeight])"),
    [[true, 240, 126], [true, 240, 126]],
    "カードの画像",
  );
  // カードのないページでは、カードの部品を読み込まない。
  await tab.size(1024, 800);
  await tab.goto(origin + "/about");
  await sleep(1000);
  assert(!tab.requests.some((url) => /\/LinkCard-[^/]*\.js$/.test(url)), "カードのないページでカードの部品を読み込んだ");
  noProblems(tab, "リンクカード");
});

scenario("長い本文は先頭から少しずつ描き、最後まで描く。本文の画像には大きさを付ける（ADR 0051）", async ({ tab, origin }) => {
  await tab.goto(origin + "/articles/heavy-long");
  await tab.waitFor(
    "[...document.querySelectorAll('.article-body h3')].some((h) => h.textContent === 'CJK の記号と句読点') && document.querySelector('.article-body section.footnotes') !== null",
    { message: "長い本文が最後まで描かれない" },
  );
  // 先頭の部分に足した定義の脚注の欄は描かず、全体の脚注の欄だけが、本文の最後にある。
  assertEqual(await tab.eval("document.querySelectorAll('.article-body section.footnotes').length"), 1, "脚注の欄の数");
  assert(await tab.eval("document.querySelector('.article-body').lastElementChild.matches('section.footnotes')"), "脚注の欄が本文の最後にない");
  await tab.goto(origin + "/articles/complex-mixed");
  const image = `document.querySelector('.article-body img[src$="/media/architecture.svg"]')`;
  await tab.waitFor(`${image} !== null`, { message: "本文の画像が描かれない" });
  assertEqual(await tab.eval(`[${image}.getAttribute("width"), ${image}.getAttribute("height")]`), ["640", "150"], "本文の画像の大きさ");
  noProblems(tab, "少しずつ描く");
});

scenario("elk は、図で指定したときだけ読み込む（ADR 0040）", async ({ tab, origin }) => {
  await tab.size(1200, 30000);
  // 図の数。すべて描き終わるまで待ってから、elk を読み込んだかを見る（一つ目が描けた時点では、elk の図がまだのことがある）。
  for (const [pathname, diagrams, expected] of [["/articles/complex-mixed", 7, false], ["/articles/heavy-diagrams", 10, true]]) {
    await tab.goto(origin + pathname);
    await tab.waitFor(`document.querySelectorAll('.mermaid-diagram').length === ${diagrams}`, {
      timeout: LAZY_TIMEOUT,
      message: `${pathname} の図が ${diagrams} 個描かれない`,
    });
    assertEqual(tab.requests.some((url) => /\/elk-[^/]*\.js$/.test(url)), expected, `${pathname} で elk を読み込んだか`);
  }
  await tab.size(1024, 800);
});

scenario("リンクで、ページを読み直さずに移り、戻れる", async ({ tab, origin }) => {
  await tab.goto(origin + "/articles/syntax");
  await tab.eval("window.__marker = true");
  await tab.eval("document.querySelector('header a[href=\"/archive\"]').click()");
  await tab.waitFor("location.pathname === '/archive' && document.querySelector('h1')?.textContent === 'すべての記事'");
  assert(await tab.eval("window.__marker === true"), "ページを読み直した");
  await tab.eval("history.back()");
  await tab.waitFor("location.pathname === '/articles/syntax' && document.querySelector('h1')?.textContent === '記法の一覧'");
});

scenario("ヘッダの検索ボックスで、候補を 5 件まで出し、キーで選んで移れる（ADR 0042）", async ({ tab, origin }) => {
  await tab.goto(origin + "/articles/syntax");
  await tab.eval("document.querySelector('#header-search').focus()");
  await tab.type("記事");
  // 検索に使う sql.js は、検索ボックスに焦点が来てから読み込む（ADR 0047）。
  await tab.waitFor("document.querySelectorAll('#header-search-suggestions [role=option]').length === 5", { message: "候補が 5 件出ない" });
  assertEqual(await tab.eval("document.querySelector('#header-search').getAttribute('aria-expanded')"), "true", "aria-expanded");
  await tab.key("ArrowDown");
  await tab.key("ArrowDown");
  assertEqual(await tab.eval("document.querySelector('#header-search').getAttribute('aria-activedescendant')"), "header-search-suggestion-1", "選んだ候補");
  const target = await tab.eval("document.querySelector('#header-search-suggestion-1').textContent");
  await tab.key("Enter");
  await tab.waitFor("location.pathname.startsWith('/articles/') && location.pathname !== '/articles/syntax'");
  assert(target.startsWith(await tab.eval("document.querySelector('h1').textContent")), `選んだ候補（${target}）に移っていない`);

  await tab.eval("document.querySelector('#header-search').focus()");
  await tab.type("存在しない言葉");
  await tab.waitFor("document.querySelector('#header-search-suggestions').parentElement.textContent.includes('一致する記事はありません')", {
    message: "一致しないことを出さない",
  });
  await tab.key("Escape");
  assertEqual(await tab.eval("document.querySelector('#header-search-suggestions').parentElement.hidden"), true, "Escape で閉じない");
  noProblems(tab, "ヘッダの検索");
});

scenario("検索のページは、URL の言葉で結果を出し、ヘッダから探すと結果が変わる", async ({ tab, origin }) => {
  await tab.goto(origin + "/search?q=Rust");
  await tab.waitFor("document.querySelector('main [role=status]')?.textContent.endsWith('件')");
  assertEqual(await tab.eval("document.querySelector('#search-query').value"), "Rust", "検索欄の言葉");
  assertEqual(await count(tab, "[role=search]"), 2, "検索のランドマークの数");
  await tab.eval("document.querySelector('#header-search').focus()");
  await tab.type("数式");
  await tab.key("Enter");
  await tab.waitFor("decodeURIComponent(location.search) === '?q=数式' && document.querySelector('#search-query').value === '数式'");
  await tab.eval("history.back()");
  await tab.waitFor("decodeURIComponent(location.search) === '?q=Rust' && document.querySelector('#search-query').value === 'Rust'");
});

scenario("記事のタグを出し、タグを押すとそのタグの付いた記事だけを探す（ADR 0048）", async ({ tab, origin }) => {
  await tab.goto(origin + "/posts/ruby");
  assertEqual(await tab.eval("[...document.querySelectorAll('article ul[aria-label=タグ] a')].map((a) => a.textContent)"), ["#書き方", "#HTML", "#組版"], "記事のタグ");
  await tab.eval("window.__marker = true");
  await tab.eval("[...document.querySelectorAll('article ul[aria-label=タグ] a')].find((a) => a.textContent === '#組版').click()");
  await tab.waitFor("location.pathname === '/search' && decodeURIComponent(location.search) === '?q=#組版' && document.querySelector('main [role=status]')?.textContent.endsWith('件')");
  assert(await tab.eval("window.__marker === true"), "ページを読み直した");
  assertEqual(await tab.eval("document.querySelector('#search-query').value"), "#組版", "検索欄の言葉");
  // 本文に「組版」とある記事はほかにもあるが、タグの付いた記事だけが出る。
  assertEqual(await tab.eval("[...document.querySelectorAll('main > div > ul > li > a')].map((a) => a.textContent)"), ["ルビを振る"], "タグで探した結果");
  assert((await count(tab, "main > div > ul > li")) < (await tab.eval("(async () => { history.replaceState(null, '', '/search?q=組版'); dispatchEvent(new Event('sqlite-cms:navigate')); await new Promise((r) => setTimeout(r, 300)); return document.querySelectorAll('main > div > ul > li').length; })()")), "語で探すと、タグで探すより多く出るはず");

  // ヘッダの検索ボックスでも、# で始まる語でタグを探せる。
  await tab.goto(origin + "/about");
  await tab.eval("document.querySelector('#header-search').focus()");
  await tab.type("#日記");
  await tab.waitFor("document.querySelectorAll('#header-search-suggestions [role=option]').length === 2", { message: "タグの候補が 2 件出ない" });
  noProblems(tab, "タグ");
});

scenario("知らないページは「見つかりません」を出す", async ({ tab, origin }) => {
  await tab.goto(origin + "/articles/does-not-exist");
  assert((await tab.eval("document.querySelector('main').textContent")).includes("見つかりません"), "見つかりませんを出さない");
});

scenario("content/header.md のヘッダを描く（ADR 0043）", async ({ tab, origin }) => {
  await tab.goto(origin + "/");
  assertEqual(await tab.eval("document.querySelector('meta[name=\"sqlite-cms-header\"]')?.content"), "custom", "目印");
  assertEqual(await count(tab, "header .site-header-body"), 1, "カスタムのヘッダ");
  assertEqual(await count(tab, "header .site-header-body form[role=search]"), 1, "パーシャルの検索ボックス");
  assertEqual(await tab.eval("document.querySelector('header .site-header-body p a').textContent"), "tett23の記事置き場", "サイト名");
});

scenario("content/header.md がなければ、既定のヘッダを描く", async ({ tab, defaultOrigin }) => {
  await tab.goto(defaultOrigin + "/");
  assertEqual(await tab.eval("document.querySelector('meta[name=\"sqlite-cms-header\"]')"), null, "目印");
  assertEqual(await count(tab, "header .site-header-body"), 0, "カスタムのヘッダ");
  assertEqual(await count(tab, "header nav a"), 4, "既定の案内");
  assertEqual(await count(tab, "header form[role=search]"), 1, "検索ボックス");
  noProblems(tab, "既定のヘッダ");
});

scenario("サイトをドメインの直下でない場所（base_path）に置いても動く（ADR 0030）", async ({ tab, baseOrigin }) => {
  await tab.goto(baseOrigin + "/blog/articles/syntax");
  assertEqual(await tab.eval("document.querySelector('h1').textContent"), "記法の一覧", "本文");
  await tab.eval("window.__marker = true");
  await tab.eval("document.querySelector('header a[href=\"/blog/archive\"]').click()");
  await tab.waitFor("location.pathname === '/blog/archive' && document.querySelector('h1')?.textContent === 'すべての記事'");
  assert(await tab.eval("window.__marker === true"), "ページを読み直した");
  assertEqual(await tab.eval("document.querySelector('header form[role=search]').getAttribute('action')"), "/blog/search", "検索ボックスの送り先");
  await tab.eval("document.querySelector('#header-search').focus()");
  await tab.type("Rust");
  await tab.key("Enter");
  await tab.waitFor("location.pathname === '/blog/search' && location.search === '?q=Rust' && document.querySelector('main [role=status]')?.textContent.endsWith('件')");
  // 画像（/media/…）もサイトを置くパスから取る。
  await tab.goto(baseOrigin + "/blog/articles/syntax");
  await sleep(1000);
  assert(tab.requests.some((url) => url.includes("/blog/media/")), "画像をサイトを置くパスから取っていない");
  noProblems(tab, "base_path");
});

async function main() {
  // 既定のヘッダを確かめるため、content/header.md を除いた写しも配信する。認証情報（.env）は写さない。
  const copy = async (prefix) => {
    const dir = await mkdtemp(path.join(os.tmpdir(), prefix));
    await cp(siteDir, dir, { recursive: true, filter: (src) => !path.basename(src).startsWith(".env") });
    return dir;
  };
  const defaultSite = await copy("sqlite-cms-e2e-default-");
  await rm(path.join(defaultSite, "content", "header.md"), { force: true });
  // base_path を確かめるため、/blog/ に置く写しも配信する。url のパスは base_path とそろえる必要がある。
  const baseSite = await copy("sqlite-cms-e2e-base-");
  const toml = await readFile(path.join(baseSite, "site.toml"), "utf8");
  // base_path は、表（[license] など）の中に入らないよう先頭に書く。
  await writeFile(path.join(baseSite, "site.toml"), `base_path = "/blog/"\n${toml.replace(/^url = .*$/m, 'url = "https://example.com/blog/"')}`);

  const [server, defaultServer, baseServer] = await Promise.all([startServer(siteDir), startServer(defaultSite), startServer(baseSite)]);
  const chrome = await chromeLauncher.launch({ chromeFlags: ["--headless=new", ...(process.env.CI ? ["--no-sandbox"] : [])] });
  const results = [];
  try {
    for (const { name, fn } of scenarios) {
      const start = Date.now();
      let tab;
      try {
        tab = await Tab.open(chrome.port);
        await tab.size(1024, 800);
        let timer;
        await Promise.race([
          fn({ tab, origin: server.origin, defaultOrigin: defaultServer.origin, baseOrigin: baseServer.origin }),
          new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error(`${SCENARIO_TIMEOUT / 1000} 秒で終わらない`)), SCENARIO_TIMEOUT);
          }),
        ]).finally(() => clearTimeout(timer));
        results.push({ name, ok: true, ms: Date.now() - start });
      } catch (error) {
        results.push({ name, ok: false, ms: Date.now() - start, error: error.message });
      } finally {
        await tab?.close().catch(() => {});
      }
    }
  } finally {
    chrome.kill();
    for (const s of [server, defaultServer, baseServer]) s.stop();
    for (const dir of [defaultSite, baseSite]) await rm(dir, { recursive: true, force: true });
  }

  const lines = results.map((r) => `${r.ok ? "✓" : "✗"} ${r.name}（${(r.ms / 1000).toFixed(1)} 秒）${r.ok ? "" : `\n    ${r.error}`}`);
  const failed = results.filter((r) => !r.ok).length;
  const summary = `## E2E\n\n${lines.join("\n")}\n\n${results.length - failed} 件成功、${failed} 件失敗\n`;
  console.log(summary);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, summary.replace(/\n    /g, "\n    - "));
  if (failed > 0) process.exitCode = 1;
}

await main();
