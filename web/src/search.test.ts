import initSqlJs from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { parseQuery, plainText, search, searchPath, snippet, tagQuery } from "./search";
import { SqliteFile } from "./sqlite";

let db: SqliteFile;

beforeAll(async () => {
  const SQL = await initSqlJs();
  const sql = new SQL.Database();
  const migrations = import.meta.glob<string>("../../migrations/*.sql", { query: "?raw", import: "default", eager: true });
  for (const file of Object.keys(migrations).sort()) sql.exec(migrations[file]);
  const post = (slug: string, title: string, date: string, body: string) =>
    sql.run("INSERT INTO posts (slug, title, published_at, body_md) VALUES (?, ?, ?, ?)", [slug, title, date, body]);
  post("ruby", "ルビを振る", "2026-09-23", "本文に<ruby>振<rt>ふ</rt></ruby>り仮名を付けたいときは、`<ruby>` を使う。");
  post("shiki", "コードの色分け", "2026-09-25", "色分けには **Shiki** を使っていて、VS Code と同じ文法の定義で色を決めている。");
  post("hello", "記事置き場を作った", "2026-09-16", "書いたものを置いておく場所を作った。[仕組み](/articles/x) に書いた。");
  sql.run("INSERT INTO articles (slug, title, published_at, updated_at, description, body_md) VALUES (?, ?, ?, ?, ?, ?)", [
    "getting-started",
    "sqlite-cms の使い方",
    "2026-09-26",
    null,
    null,
    "記事リポジトリを作ってから、Cloudflare に公開するまでの手順。\n\n```sh\nsqlite-cms init my-blog\n```",
  ]);
  // タグ（ADR 0048）。about の本文にも「日本語」があるが、タグは付いていない。
  const tag = (kind: string, slug: string, tags: string[]) =>
    tags.forEach((name, position) => sql.run("INSERT INTO tags VALUES (?, ?, ?, ?)", [kind, slug, position, name]));
  tag("post", "ruby", ["マークアップ", "日本語"]);
  tag("post", "shiki", ["マークアップ", "ハイライト"]);
  tag("article", "getting-started", ["手引き", "HTML"]);
  sql.run("INSERT INTO pages VALUES (?, ?, ?)", ["about", "自己紹介", "組版と日本語の文章が好きです。"]);
  sql.run("INSERT INTO pages VALUES (?, ?, ?)", ["draft", "下書き", "組版の下書き。URL のないページ。"]);
  // sql.js で作った DB を書き出し、ページの表示と同じく自前の読み手で開く（ADR 0047）。
  db = new SqliteFile(sql.export());
  sql.close();
});

describe("語の分け方", () => {
  it("検索する言葉を、空白で区切った語に分け、全角の英数字を半角に、大文字を小文字にする", () => {
    expect(parseQuery("  ＳＨＩＫＩ  使い方 ")).toEqual({ terms: ["shiki", "使い方"], tags: [] });
    expect(parseQuery("!!! 字")).toEqual({ terms: ["字"], tags: [] });
  });

  it("# で始まる語は、タグとして分ける。全角の ＃ も同じに扱う（ADR 0048）", () => {
    expect(parseQuery("#組版 使い方 ＃ＳＱＬｉｔｅ")).toEqual({ terms: ["使い方"], tags: ["組版", "sqlite"] });
    expect(parseQuery("# ## C#入門")).toEqual({ terms: ["c#入門"], tags: [] });
  });

  it("Markdown の記号を大まかに取り除く", () => {
    expect(plainText("## 見出し\n\n**強調**と [リンク](/a) と ![画像](/b.png)\n\n```sh\ncode\n```")).toBe(
      "見出し 強調と リンク と 画像 code",
    );
  });
});

describe("検索", () => {
  const titles = (query: string) => search(db, query).map((r) => r.title);

  it("日本語、英数字、全角の英数字、複数の語で探せる", () => {
    expect(titles("仮名")).toEqual(["ルビを振る"]);
    expect(titles("shiki")).toEqual(["コードの色分け"]);
    expect(titles("ＳＨＩＫＩ")).toEqual(["コードの色分け"]);
    expect(titles("shi")).toEqual(["コードの色分け"]);
    expect(titles("記事 公開")).toEqual(["sqlite-cms の使い方"]);
  });

  it("題名に語を含むものを先に、あとは新しい順に並べる", () => {
    expect(titles("記事")).toEqual(["記事置き場を作った", "sqlite-cms の使い方"]);
  });

  it("1 文字の日本語でも探せる", () => {
    expect(titles("振")).toEqual(["ルビを振る"]);
  });

  it("隣り合わない語の並びは一致しない", () => {
    expect(titles("置き方")).toEqual([]);
  });

  it("コードブロックの中も探せる", () => {
    expect(titles("my-blog")).toEqual(["sqlite-cms の使い方"]);
  });

  it("自己紹介を探せ、URL のないページは出さない", () => {
    const results = search(db, "組版");
    expect(results.map((r) => [r.title, r.path])).toEqual([["自己紹介", "/about"]]);
  });

  it("結果にサイトの中のパスと抜き出しを付ける", () => {
    const [result] = search(db, "文法");
    expect(result.path).toBe("/posts/shiki");
    expect(result.snippet.filter((part) => part.match).map((part) => part.text)).toEqual(["文法"]);
  });

  it("空の言葉では何も返さない", () => {
    expect(search(db, "  ")).toEqual([]);
    expect(search(db, " # ")).toEqual([]);
  });
});

describe("タグで探す（ADR 0048）", () => {
  const titles = (query: string) => search(db, query).map((r) => r.title);

  it("# で始まる語は、そのタグの付いた記事だけに一致する", () => {
    expect(titles("#マークアップ")).toEqual(["コードの色分け", "ルビを振る"]);
    // 本文に「日本語」がある自己紹介は、タグが付いていないので出ない。
    expect(titles("#日本語")).toEqual(["ルビを振る"]);
  });

  it("タグは、全体が一致するものだけを探す。大文字と小文字、全角と半角は区別しない", () => {
    expect(titles("#マーク")).toEqual([]);
    expect(titles("#html")).toEqual(["sqlite-cms の使い方"]);
    expect(titles("＃ＨＴＭＬ")).toEqual(["sqlite-cms の使い方"]);
  });

  it("タグと語、タグとタグを組み合わせられる", () => {
    expect(titles("#マークアップ 振")).toEqual(["ルビを振る"]);
    expect(titles("#マークアップ #ハイライト")).toEqual(["コードの色分け"]);
    expect(titles("#マークアップ #手引き")).toEqual([]);
  });

  it("# のない語は、タグからも探す", () => {
    expect(titles("日本語")).toEqual(["ルビを振る", "自己紹介"]);
    expect(titles("ハイライ")).toEqual(["コードの色分け"]);
  });

  it("結果にタグを付ける", () => {
    expect(search(db, "#ハイライト").map((r) => r.tags)).toEqual([["マークアップ", "ハイライト"]]);
    expect(search(db, "組版").map((r) => r.tags)).toEqual([[]]);
  });

  it("タグのリンクは、# を付けた言葉での検索になる", () => {
    expect(searchPath(tagQuery("組版"))).toBe("/search?q=%23%E7%B5%84%E7%89%88");
  });
});

describe("抜き出し", () => {
  it("最初に語が現れるあたりを抜き出し、語に印を付ける", () => {
    const text = "あ".repeat(50) + "組版" + "い".repeat(100) + "組版";
    const parts = snippet(text, ["組版"]);
    expect(parts[0]).toEqual({ text: "…", match: false });
    expect(parts.find((part) => part.match)?.text).toBe("組版");
    expect(parts[parts.length - 1]).toEqual({ text: "…", match: false });
  });

  it("語がなければ先頭から抜き出す", () => {
    expect(snippet("短い本文", ["なし"])).toEqual([{ text: "短い本文", match: false }]);
  });
});

describe("検索のページのパス", () => {
  it("言葉を前後の空白を除いて q に入れる", () => {
    expect(searchPath("  記事 ")).toBe("/search?q=%E8%A8%98%E4%BA%8B");
    expect(searchPath("a&b c")).toBe("/search?q=a%26b%20c");
  });

  it("言葉が空なら q を付けない", () => {
    expect(searchPath("")).toBe("/search");
    expect(searchPath("   ")).toBe("/search");
  });
});
