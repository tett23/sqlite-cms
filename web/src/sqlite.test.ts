import initSqlJs, { type Database, type SqlJsStatic } from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { compareBinary, parseColumns, SqliteFile } from "./sqlite";

let SQL: SqlJsStatic;

beforeAll(async () => {
  SQL = await initSqlJs();
});

/** sql.js で表のすべての行を、rowid の順に列の名前つきで読む（正解として使う）。 */
function expected(db: Database, table: string) {
  const result = db.exec(`SELECT * FROM "${table}" ORDER BY rowid`);
  if (result.length === 0) return [];
  const { columns, values } = result[0];
  return values.map((row) => Object.fromEntries(columns.map((c, i) => [c, row[i]])));
}

function pseudoRandomText(length: number, seed: number) {
  const chars = "あいうえお漢字カナabcXYZ0123456789 \n😀";
  let x = seed;
  return Array.from({ length }, () => {
    x = (x * 1103515245 + 12345) % 2147483648;
    return Array.from(chars)[x % Array.from(chars).length];
  }).join("");
}

describe("SQLite のファイルを読む（ADR 0047）", () => {
  it.each([512, 1024, 4096, 65536])("ページの大きさが %d バイトでも、sql.js と同じ行を読む", (pageSize) => {
    const db = new SQL.Database();
    db.run(`PRAGMA page_size = ${pageSize}`);
    db.run("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT NOT NULL, n INTEGER, f REAL, b BLOB, note TEXT)");
    const insert = db.prepare("INSERT INTO t (name, n, f, b, note) VALUES (?, ?, ?, ?, ?)");
    const numbers = [0, 1, -1, 127, -128, 32767, -32768, 8388607, -8388608, 2147483647, -2147483648, 140737488355327, -140737488355328, Number.MAX_SAFE_INTEGER, Number.MIN_SAFE_INTEGER];
    for (let i = 0; i < 300; i++) {
      // 長い文字列は、あふれたページ（overflow）に続く。行が多いと、内部のページができる。
      const long = pseudoRandomText(i % 7 === 0 ? 5000 + i * 13 : 10 + (i % 50), i);
      insert.run([`名前${i}`, numbers[i % numbers.length], i / 7, new Uint8Array([i % 256, 0, 255]), i % 5 === 0 ? null : long]);
    }
    insert.free();
    const file = new SqliteFile(db.export());
    expect(file.table("t")).toEqual(expected(db, "t"));
    db.close();
  });

  it("列を足す前に入れた行の、足した列は NULL として読む。名前を変えた表と列も読む", () => {
    const db = new SQL.Database();
    db.run("CREATE TABLE articles (slug TEXT PRIMARY KEY, body_html TEXT NOT NULL)");
    db.run("INSERT INTO articles VALUES ('old', '古い')");
    db.run("ALTER TABLE articles RENAME TO posts");
    db.run("ALTER TABLE posts RENAME COLUMN body_html TO body_md");
    db.run("ALTER TABLE posts ADD COLUMN description TEXT");
    db.run("INSERT INTO posts VALUES ('new', '新しい', '要約')");
    const file = new SqliteFile(db.export());
    expect(file.table("posts")).toEqual([
      { slug: "old", body_md: "古い", description: null },
      { slug: "new", body_md: "新しい", description: "要約" },
    ]);
    expect(file.table("posts")).toEqual(expected(db, "posts"));
    expect(() => file.table("missing")).toThrow("表がありません: missing");
    db.close();
  });

  it("sqlite-cms のマイグレーションで作った表を、sql.js と同じに読む", () => {
    const db = new SQL.Database();
    const migrations = import.meta.glob<string>("../../migrations/*.sql", { query: "?raw", import: "default", eager: true });
    for (const file of Object.keys(migrations).sort()) db.exec(migrations[file]);
    db.run("INSERT INTO site VALUES (1, 't', NULL, 'CC0', NULL, '# home', 'd', '[t](/)')");
    db.run("INSERT INTO posts VALUES ('p', 'P', '2026-09-26', ?, 'p')", [pseudoRandomText(20000, 1)]);
    db.run("INSERT INTO articles VALUES ('a', 'A', '2026-09-25', NULL, '要約', '本文', 'a', NULL)");
    db.run("INSERT INTO pages VALUES ('about', '自己紹介', '本文')");
    db.run("INSERT INTO link_cards VALUES ('https://example.com/', '/link-cards/x.png')");
    const file = new SqliteFile(db.export());
    for (const table of ["site", "posts", "articles", "pages", "link_cards"]) {
      expect(file.table(table), table).toEqual(expected(db, table));
    }
    db.close();
  });

  it("SQLite のファイルでなければ失敗する", () => {
    expect(() => new SqliteFile(new Uint8Array(200))).toThrow("SQLite のデータベースファイルではありません");
  });
});

describe("表の定義から列を取り出す", () => {
  it("列の名前と、rowid の別名を取り出し、表の制約は除く", () => {
    expect(parseColumns('CREATE TABLE "posts" (id INTEGER PRIMARY KEY CHECK (id = 1), "the ""x""" TEXT, `b` INT, [c] TEXT DEFAULT \'a,b\', PRIMARY KEY (b), UNIQUE (c))')).toEqual({
      names: ["id", 'the "x"', "b", "c"],
      defaults: [{ value: null }, { value: null }, { value: null }, { value: "a,b" }],
      rowidAlias: "id",
    });
    expect(parseColumns("CREATE TABLE t (slug TEXT PRIMARY KEY, n INTEGER)")).toEqual({
      names: ["slug", "n"],
      defaults: [{ value: null }, { value: null }],
      rowidAlias: null,
    });
  });
});

describe("文字列の比べ方（BINARY）", () => {
  it("UTF-8 のバイトの順（コードポイントの順）で比べる", () => {
    // UTF-16 では ￿ が 😀（サロゲートペア）より大きいが、コードポイントでは小さい。
    expect(Math.sign(compareBinary("￿", "😀"))).toBe(-1);
    expect(Math.sign(compareBinary("2026-09-26", "2026-09-25"))).toBe(1);
    expect(compareBinary("a", "a")).toBe(0);
    expect(Math.sign(compareBinary("a", "ab"))).toBe(-1);
  });
});
