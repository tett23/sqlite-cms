// 自前の SQLite の読み手（ADR 0047）を、sql.js（本物の SQLite）を正解として、いろいろな DB で確かめる。
import initSqlJs, { type Database, type SqlJsStatic, type SqlValue } from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { compareBinary, parseDefault, SqliteFile } from "./sqlite";

let SQL: SqlJsStatic;

beforeAll(async () => {
  SQL = await initSqlJs();
});

/** 種を決めた乱数（同じ種なら同じ DB を作る）。 */
function random(seed: number) {
  let x = seed >>> 0 || 1;
  const next = () => {
    // xorshift32
    x ^= x << 13;
    x >>>= 0;
    x ^= x >>> 17;
    x ^= x << 5;
    x >>>= 0;
    return x / 0x100000000;
  };
  const int = (n: number) => Math.floor(next() * n);
  const pick = <T>(items: readonly T[]): T => items[int(items.length)];
  return { next, int, pick };
}

type Random = ReturnType<typeof random>;

/** 各種の値。整数は、SQLite が 1、2、3、4、6、8 バイトで保存する境目の前後を含む。 */
const INTEGERS = [
  0, 1, -1, 2, 127, 128, -128, -129, 255, 256, 32767, 32768, -32768, -32769, 8388607, 8388608, -8388608, -8388609,
  2147483647, 2147483648, -2147483648, -2147483649, 140737488355327, 140737488355328, -140737488355328, -140737488355329,
  Number.MAX_SAFE_INTEGER, Number.MIN_SAFE_INTEGER,
];
const FLOATS = [0.5, -0.5, 1e-300, -1e300, 3.141592653589793, 1.7976931348623157e308, Number.EPSILON, 2 ** -1074, 1 / 3];
const CHARACTERS = Array.from("aZ09 _-あいう漢字カナ　、。「」é́‍😀👨‍👩‍👧𠮷\n\t'\"\\%");

function randomText(r: Random, length: number) {
  return Array.from({ length }, () => r.pick(CHARACTERS)).join("");
}

function randomValue(r: Random): SqlValue {
  switch (r.int(7)) {
    case 0:
      return null;
    case 1:
      return r.pick(INTEGERS);
    case 2:
      return r.pick(FLOATS);
    case 3:
      return new Uint8Array(Array.from({ length: r.pick([0, 1, 10, 600, 3000]) }, () => r.int(256)));
    case 4:
      // 長い文字列は、あふれたページに続く。
      return randomText(r, r.pick([0, 1, 50, 400, 2000, 9000]));
    default:
      return randomText(r, r.int(40));
  }
}

/** sql.js で表のすべての行を、rowid の順に列の名前つきで読む（正解）。 */
function expected(db: Database, table: string) {
  const result = db.exec(`SELECT * FROM "${table}" ORDER BY rowid`);
  if (result.length === 0) return [];
  const { columns, values } = result[0];
  return values.map((row) => Object.fromEntries(columns.map((c, i) => [c, row[i]])));
}

/** 表の名前（sqlite_ で始まるものを除く）。 */
function tables(db: Database): string[] {
  const result = db.exec("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name");
  return result.length === 0 ? [] : result[0].values.map(([name]) => name as string);
}

function expectSameRows(db: Database) {
  const file = new SqliteFile(db.export());
  for (const table of tables(db)) {
    expect(file.table(table), table).toEqual(expected(db, table));
  }
}

const TYPES = ["INTEGER", "TEXT", "REAL", "BLOB", "NUMERIC", ""];

describe("乱数で作った DB を、sql.js と同じに読む（ADR 0047）", () => {
  it.each(Array.from({ length: 24 }, (_, i) => i + 1))("種 %d", (seed) => {
    const r = random(seed * 7919);
    const db = new SQL.Database();
    db.run(`PRAGMA page_size = ${r.pick([512, 1024, 2048, 4096, 8192, 65536])}`);
    if (r.int(3) === 0) db.run("PRAGMA auto_vacuum = FULL");
    for (let t = 0; t < 1 + r.int(3); t++) {
      const columns = Array.from({ length: 1 + r.int(12) }, (_, i) => `c${i} ${r.pick(TYPES)}`);
      // rowid の別名の列を持つ表と、持たない表。
      const withId = r.int(2) === 0;
      db.run(`CREATE TABLE t${t} (${withId ? "id INTEGER PRIMARY KEY, " : ""}${columns.join(", ")})`);
      const placeholders = columns.map(() => "?").join(", ");
      const insert = db.prepare(`INSERT INTO t${t} (${withId ? "id, " : ""}${columns.map((_, i) => `c${i}`).join(", ")}) VALUES (${withId ? "?, " : ""}${placeholders})`);
      const ids = new Set<number>();
      for (let row = 0; row < r.int(400); row++) {
        const values = columns.map(() => randomValue(r));
        if (withId) {
          // rowid は、負の値や大きな値を含め、挿入の順と違う順にする。
          let id = r.pick([r.int(1_000_000) - 500_000, r.pick(INTEGERS), row]);
          while (ids.has(id)) id++;
          ids.add(id);
          insert.run([id, ...values]);
        } else {
          insert.run(values);
        }
      }
      insert.free();
      // 削除と更新で、ページに空きや、長さの変わったレコードを作る。
      if (r.int(2) === 0) db.run(`DELETE FROM t${t} WHERE rowid % ${2 + r.int(4)} = 0`);
      if (r.int(2) === 0) db.run(`UPDATE t${t} SET c0 = ? WHERE rowid % 3 = 1`, [randomValue(r)]);
      // 列を足す前の行は、足した列を持たない。
      if (r.int(2) === 0) db.run(`ALTER TABLE t${t} ADD COLUMN added TEXT DEFAULT 'd${seed}'`);
    }
    if (r.int(3) === 0) db.run("VACUUM");
    expectSameRows(db);
    db.close();
  });
});

describe("あふれたページの境目（ADR 0047）", () => {
  it.each([512, 1024, 4096])("ページが %d バイトのとき、ページに収まる長さとあふれる長さの境目のどれも読む", (pageSize) => {
    const db = new SQL.Database();
    db.run(`PRAGMA page_size = ${pageSize}`);
    db.run("CREATE TABLE t (s TEXT)");
    const insert = db.prepare("INSERT INTO t VALUES (?)");
    // レコードの大きさを 1 バイトずつ変え、ページの 3 倍を少し超えるまで入れる。
    // ページに収まる上限、あふれる部分の長さの計算の二つの場合分け、あふれたページが 1 枚と 2 枚以上の場合を、すべて通る。
    for (let length = 0; length <= pageSize * 3 + 20; length++) insert.run(["x".repeat(length)]);
    insert.free();
    const rows = new SqliteFile(db.export()).table("t");
    expect(rows.map((row) => (row.s as string).length)).toEqual(Array.from({ length: pageSize * 3 + 21 }, (_, i) => i));
    expect(rows).toEqual(expected(db, "t"));
    db.close();
  });
});

describe("表の形（ADR 0047）", () => {
  it("行が多く、内部のページが何段にもなる B-tree を読む", () => {
    const db = new SQL.Database();
    db.run("PRAGMA page_size = 512");
    db.run("CREATE TABLE t (n INTEGER, s TEXT)");
    const insert = db.prepare("INSERT INTO t VALUES (?, ?)");
    for (let i = 0; i < 20000; i++) insert.run([i, `行 ${i}`]);
    insert.free();
    // 512 バイトのページに 20000 行なら、根から葉まで 3 段以上になる。
    const pages = db.exec("PRAGMA page_count")[0].values[0][0] as number;
    expect(pages).toBeGreaterThan(500);
    const rows = new SqliteFile(db.export()).table("t");
    expect(rows).toHaveLength(20000);
    expect(rows).toEqual(expected(db, "t"));
    db.close();
  });

  it("列が多い表（レコードの見出しの大きさが 2 バイトの可変長整数になる）を読む", () => {
    const db = new SQL.Database();
    const columns = Array.from({ length: 200 }, (_, i) => `c${i}`);
    db.run(`CREATE TABLE t (${columns.join(", ")})`);
    db.run(`INSERT INTO t VALUES (${columns.map(() => "?").join(", ")})`, columns.map((_, i) => (i % 3 === 0 ? `値${i}`.repeat(30) : i % 3 === 1 ? i * 1000 : null)));
    expectSameRows(db);
    db.close();
  });

  it("rowid の端の値（負の数、0、9 バイトの可変長整数になる大きな値）を読む", () => {
    const db = new SQL.Database();
    db.run("CREATE TABLE t (id INTEGER PRIMARY KEY, s TEXT)");
    for (const id of ["-9223372036854775808", "-1", "0", "1", "72057594037927935", "72057594037927936", "9223372036854775807"]) {
      db.run(`INSERT INTO t VALUES (${id}, '${id}')`);
    }
    expectSameRows(db);
    db.close();
  });

  it("NUL を含む文字列を、切り捨てずに読む", () => {
    const db = new SQL.Database();
    db.run("CREATE TABLE t (s TEXT)");
    // sql.js は NUL の後ろを切り捨てるので、SQLite の hex() と比べる。
    db.run("INSERT INTO t VALUES (CAST(X'6100620063' AS TEXT)), (CAST(X'00' AS TEXT))");
    const hexes = db.exec("SELECT hex(s) FROM t ORDER BY rowid")[0].values.map(([h]) => h);
    expect(hexes).toEqual(["6100620063", "00"]);
    expect(new SqliteFile(db.export()).table("t")).toEqual([{ s: "a\u0000b\u0000c" }, { s: "\u0000" }]);
    db.close();
  });

  it("INTEGER PRIMARY KEY DESC は rowid の別名ではないので、列の値を読む", () => {
    const db = new SQL.Database();
    db.run("CREATE TABLE t (id INTEGER PRIMARY KEY DESC, s TEXT)");
    db.run("INSERT INTO t VALUES (5, 'a'), (3, 'b')");
    expectSameRows(db);
    db.close();
  });

  it("索引と仮想表があっても、ほかの表を読む。仮想表と WITHOUT ROWID の表は、読めないことを知らせる", () => {
    const db = new SQL.Database();
    db.run("CREATE TABLE t (slug TEXT PRIMARY KEY, n INTEGER UNIQUE)");
    db.run("CREATE INDEX t_n ON t (n)");
    db.run("INSERT INTO t VALUES ('a', 1), ('b', 2)");
    db.run("CREATE VIRTUAL TABLE f USING fts4(body)");
    db.run("INSERT INTO f VALUES ('本文')");
    db.run("CREATE TABLE w (k TEXT PRIMARY KEY, v TEXT) WITHOUT ROWID");
    const file = new SqliteFile(db.export());
    expect(file.table("t")).toEqual(expected(db, "t"));
    expect(() => file.table("f")).toThrow("仮想表は読めません: f");
    expect(() => file.table("w")).toThrow("WITHOUT ROWID の表は読めません: w");
    db.close();
  });
});

describe("列を足す前に入れた行の、足した列（ADR 0047）", () => {
  it("SQLite と同じく、足した列の既定値として読む", () => {
    const db = new SQL.Database();
    db.run("CREATE TABLE t (a TEXT)");
    db.run("INSERT INTO t VALUES ('old')");
    const defaults = ["'文字列''引用符'", "-3", "+2.5e3", "0x1F", "-0x10", "x'00ff'", "NULL", "TRUE", "FALSE", "''"];
    defaults.forEach((value, i) => db.run(`ALTER TABLE t ADD COLUMN c${i} DEFAULT ${value}`));
    db.run("ALTER TABLE t ADD COLUMN plain TEXT");
    db.run("ALTER TABLE t ADD COLUMN constrained TEXT NOT NULL DEFAULT 'x' CHECK (constrained <> 'DEFAULT 1')");
    db.run(`INSERT INTO t VALUES (${Array.from({ length: defaults.length + 3 }, () => "'new'").join(", ")})`);
    expectSameRows(db);
    db.close();
  });

  it("既定値の書き方を読む。式は読まない", () => {
    expect(parseDefault("TEXT DEFAULT 'a b' NOT NULL")).toEqual({ value: "a b" });
    expect(parseDefault("INTEGER NOT NULL DEFAULT -12")).toEqual({ value: -12 });
    expect(parseDefault("REAL DEFAULT .5")).toEqual({ value: 0.5 });
    expect(parseDefault("TEXT CHECK (x <> 'DEFAULT 1')")).toEqual({ value: null });
    expect(parseDefault("TEXT")).toEqual({ value: null });
    expect(parseDefault("TEXT DEFAULT (1 + 1)")).toEqual({ unsupported: "(1 + 1)" });
    expect(parseDefault("TEXT DEFAULT CURRENT_TIMESTAMP NOT NULL")).toEqual({ unsupported: "CURRENT_TIMESTAMP" });
  });
});

describe("読めない DB と壊れた DB（ADR 0047）", () => {
  function sampleDb(pageSize = 512) {
    const db = new SQL.Database();
    db.run(`PRAGMA page_size = ${pageSize}`);
    db.run("CREATE TABLE t (s TEXT)");
    const insert = db.prepare("INSERT INTO t VALUES (?)");
    for (let i = 0; i < 300; i++) insert.run([`行 ${i}`]);
    insert.free();
    const bytes = db.export();
    db.close();
    return bytes;
  }

  it("UTF-8 以外で符号化した DB は、読めないことを知らせる", () => {
    const db = new SQL.Database();
    db.run('PRAGMA encoding = "UTF-16le"');
    db.run("CREATE TABLE t (s TEXT)");
    db.run("INSERT INTO t VALUES ('a')");
    expect(() => new SqliteFile(db.export())).toThrow("UTF-8 以外で符号化したデータベースには対応していません");
    db.close();
  });

  it("途中で切れたファイルは、切れていることを知らせる", () => {
    const bytes = sampleDb();
    expect(() => new SqliteFile(bytes.slice(0, bytes.length - 512)).table("t")).toThrow("ファイルの外にあります");
    expect(() => new SqliteFile(bytes.slice(0, 99))).toThrow("SQLite のデータベースファイルではありません");
  });

  it("同じページをたどり続ける壊れた B-tree は、深すぎることを知らせる", () => {
    const bytes = sampleDb();
    // 表の根のページ（内部のページ）の右端の子を、根そのものにする。
    const root = new SqliteFile(bytes);
    const rootPage = new SQL.Database(bytes).exec("SELECT rootpage FROM sqlite_master WHERE name = 't'")[0].values[0][0] as number;
    const view = new DataView(bytes.buffer, bytes.byteOffset);
    const header = (rootPage - 1) * 512;
    expect(view.getUint8(header)).toBe(0x05);
    view.setUint32(header + 8, rootPage);
    expect(root).toBeDefined();
    expect(() => new SqliteFile(bytes).table("t")).toThrow("表の B-tree が深すぎます");
  });

  it("表の B-tree でないページを指していれば、そのことを知らせる", () => {
    const bytes = sampleDb();
    const rootPage = new SQL.Database(bytes).exec("SELECT rootpage FROM sqlite_master WHERE name = 't'")[0].values[0][0] as number;
    new DataView(bytes.buffer, bytes.byteOffset).setUint8((rootPage - 1) * 512, 0x0a);
    expect(() => new SqliteFile(bytes).table("t")).toThrow(`表の B-tree のページではありません（${rootPage} ページ目、種類 10）`);
  });
});

describe("文字列の比べ方（ADR 0047）", () => {
  it("乱数で作った文字列を、SQLite の ORDER BY（BINARY）と同じ順に並べる", () => {
    const r = random(42);
    const strings = Array.from({ length: 500 }, () => randomText(r, r.int(6)));
    strings.push("", "\uFFFF", "😀", "\u0000", "a\u0000b", "a");
    // sql.js は文字列を C の文字列として渡すので、NUL の後ろを切り捨てる。
    // NUL を含む文字列も確かめるため、UTF-8 のバイト列（16 進）として書き、並べた結果も 16 進で受け取る。
    const hex = (text: string) =>
      Array.from(new TextEncoder().encode(text), (b) => b.toString(16).padStart(2, "0").toUpperCase()).join("");
    const db = new SQL.Database();
    db.run("CREATE TABLE t (s TEXT)");
    for (const text of strings) db.run(`INSERT INTO t VALUES (CAST(X'${hex(text)}' AS TEXT))`);
    const ordered = db.exec("SELECT hex(s) FROM t ORDER BY s")[0].values.map(([h]) => h);
    expect([...strings].sort(compareBinary).map(hex)).toEqual(ordered);
    db.close();
  });
});
