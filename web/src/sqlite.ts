// SQLite のデータベースファイルを読む、小さな読み手（ADR 0047）。
// ページを表示するのに要るのは、表の行をすべて読むことだけなので、sql.js（wasm が gzip 後で約 320 KB）を読み込まずに、
// ファイル形式（https://www.sqlite.org/fileformat2.html）に沿って、表の B-tree とレコードを直接読む。
// 索引、仮想表、WITHOUT ROWID の表、UTF-8 以外の文字の符号化には対応しない（sqlite-cms の DB は使わない）。
// SPA は SQL を使わない（全文検索も、読んだ行を JavaScript で確かめる）。sql.js はテストで、この読み手の正しさを確かめるのに使う。

export type Value = number | string | Uint8Array | null;
export type Row = Record<string, Value>;

const HEADER = "SQLite format 3\u0000";

/** 列の既定値。定数（文字列、数、NULL、TRUE、FALSE、BLOB）だけを読む。式は読まず、その文字列を unsupported に入れる。 */
export type ColumnDefault = { value: Value } | { unsupported: string };

/** 列の制約（型の後ろ）から、DEFAULT の値を読む。DEFAULT がなければ NULL。 */
export function parseDefault(constraints: string): ColumnDefault {
  // 括弧と引用符の外にある DEFAULT を探す。
  let depth = 0;
  let quote: string | null = null;
  let at = -1;
  for (let i = 0; i < constraints.length; i++) {
    const c = constraints[i];
    if (quote) {
      if (c === quote) quote = null;
    } else if (c === "'" || c === '"' || c === "`") {
      quote = c;
    } else if (c === "[") {
      quote = "]";
    } else if (c === "(") {
      depth++;
    } else if (c === ")") {
      depth--;
    } else if (depth === 0 && /^DEFAULT\b/i.test(constraints.slice(i)) && (i === 0 || /\s/.test(constraints[i - 1]))) {
      at = i + "DEFAULT".length;
      break;
    }
  }
  if (at === -1) return { value: null };
  const rest = constraints.slice(at).trimStart();
  const string = rest.match(/^'((?:[^']|'')*)'/);
  if (string) return { value: string[1].replace(/''/g, "'") };
  const blob = rest.match(/^[xX]'([0-9a-fA-F]*)'/);
  if (blob) return { value: new Uint8Array((blob[1].match(/../g) ?? []).map((h) => parseInt(h, 16))) };
  const hex = rest.match(/^([+-]?)0[xX]([0-9a-fA-F]+)\b/);
  if (hex) return { value: (hex[1] === "-" ? -1 : 1) * Number(BigInt.asIntN(64, BigInt(`0x${hex[2]}`))) };
  const number = rest.match(/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?(?![\w.])/);
  if (number) return { value: Number(number[0]) };
  const keyword = rest.match(/^(NULL|TRUE|FALSE)\b/i);
  if (keyword) return { value: { NULL: null, TRUE: 1, FALSE: 0 }[keyword[1].toUpperCase() as "NULL" | "TRUE" | "FALSE"] };
  return { unsupported: rest.split(/\s+(?=NOT\b|NULL\b|PRIMARY\b|UNIQUE\b|CHECK\b|REFERENCES\b|COLLATE\b|GENERATED\b|CONSTRAINT\b)/i)[0] };
}

/**
 * 表の定義から、列の名前と既定値と、rowid の別名（INTEGER PRIMARY KEY）の列を取り出す。
 * INTEGER PRIMARY KEY DESC は、SQLite では rowid の別名にならない（値を列に持つ）。
 */
export function parseColumns(sql: string): { names: string[]; defaults: ColumnDefault[]; rowidAlias: string | null } {
  const start = sql.indexOf("(");
  const end = sql.lastIndexOf(")");
  if (start === -1 || end <= start) throw new Error(`表の定義を読めません: ${sql}`);
  const body = sql.slice(start + 1, end);
  // 括弧と引用符の外にある , で、列の定義に分ける。
  const definitions: string[] = [];
  let depth = 0;
  let quote: string | null = null;
  let current = "";
  for (const c of body) {
    if (quote) {
      if (c === quote) quote = null;
    } else if (c === '"' || c === "'" || c === "`") {
      quote = c;
    } else if (c === "[") {
      quote = "]";
    } else if (c === "(") {
      depth++;
    } else if (c === ")") {
      depth--;
    } else if (c === "," && depth === 0) {
      definitions.push(current);
      current = "";
      continue;
    }
    current += c;
  }
  definitions.push(current);

  const names: string[] = [];
  const defaults: ColumnDefault[] = [];
  let rowidAlias: string | null = null;
  for (const definition of definitions.map((d) => d.trim()).filter(Boolean)) {
    if (/^(CONSTRAINT|PRIMARY|UNIQUE|CHECK|FOREIGN)\b/i.test(definition)) continue;
    const match = definition.match(/^(?:"((?:[^"]|"")*)"|`([^`]*)`|\[([^\]]*)\]|([^\s(]+))\s*(.*)$/s);
    if (!match) throw new Error(`列の定義を読めません: ${definition}`);
    const name = match[1]?.replace(/""/g, '"') ?? match[2] ?? match[3] ?? match[4];
    names.push(name);
    defaults.push(parseDefault(match[5]));
    if (/^INTEGER\b/i.test(match[5]) && /\bPRIMARY\s+KEY\b(?!\s+DESC\b)/i.test(match[5])) rowidAlias = name;
  }
  return { names, defaults, rowidAlias };
}

function defaultValue(table: string, column: string, value: ColumnDefault): Value {
  if ("value" in value) return value.value;
  throw new Error(`${table}.${column} の既定値（${value.unsupported}）は式なので読めません`);
}

/** B-tree の深さの上限。壊れたファイルで、同じページを繰り返したどり続けないためである（実際の DB はずっと浅い）。 */
const MAX_DEPTH = 64;

export class SqliteFile {
  private readonly view: DataView;
  private readonly pageSize: number;
  private readonly usableSize: number;
  private readonly decoder = new TextDecoder();
  private readonly tables = new Map<string, Row[]>();
  private schema: Map<string, { rootPage: number; sql: string }> | null = null;

  constructor(readonly bytes: Uint8Array) {
    if (bytes.length < 100 || new TextDecoder().decode(bytes.subarray(0, 16)) !== HEADER) {
      throw new Error("SQLite のデータベースファイルではありません");
    }
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const size = this.view.getUint16(16);
    this.pageSize = size === 1 ? 65536 : size;
    this.usableSize = this.pageSize - this.view.getUint8(20);
    if (this.view.getUint32(56) > 1) throw new Error("UTF-8 以外で符号化したデータベースには対応していません");
  }

  /** 表があるか。 */
  hasTable(name: string): boolean {
    return this.schemaTable().has(name);
  }

  /** 表のすべての行を、rowid の順に返す。 */
  table(name: string): Row[] {
    let rows = this.tables.get(name);
    if (!rows) {
      const entry = this.schemaTable().get(name);
      if (!entry) throw new Error(`表がありません: ${name}`);
      if (entry.rootPage === 0) throw new Error(`仮想表は読めません: ${name}`);
      if (/\bWITHOUT\s+ROWID\b/i.test(entry.sql)) throw new Error(`WITHOUT ROWID の表は読めません: ${name}`);
      const { names, defaults, rowidAlias } = parseColumns(entry.sql);
      rows = [];
      for (const { rowid, values } of this.scan(entry.rootPage)) {
        const row: Row = {};
        // 列を足す前に入れた行は、足した列の値を持たない。SQLite と同じく、その列の既定値として読む。
        names.forEach((column, i) => (row[column] = i < values.length ? values[i] : defaultValue(name, column, defaults[i])));
        if (rowidAlias) row[rowidAlias] = rowid;
        rows.push(row);
      }
      this.tables.set(name, rows);
    }
    return rows;
  }

  private schemaTable() {
    if (!this.schema) {
      this.schema = new Map();
      // sqlite_master の列は type, name, tbl_name, rootpage, sql。根のページは 1。
      for (const { values } of this.scan(1)) {
        const [type, name, , rootPage, sql] = values;
        if (type === "table" && typeof name === "string" && typeof rootPage === "number" && typeof sql === "string") {
          this.schema.set(name, { rootPage, sql });
        }
      }
    }
    return this.schema;
  }

  private pageOffset(page: number) {
    const offset = (page - 1) * this.pageSize;
    if (!Number.isInteger(page) || page < 1 || offset + this.pageSize > this.bytes.length) {
      throw new Error(`${page} ページ目がファイルの外にあります（ファイルが途中で切れているか、壊れています）`);
    }
    return offset;
  }

  /** 表の B-tree を、左から順にたどる。 */
  private *scan(page: number, depth = 0): Generator<{ rowid: number; values: Value[] }> {
    if (depth > MAX_DEPTH) throw new Error("表の B-tree が深すぎます（ファイルが壊れています）");
    const start = this.pageOffset(page);
    // 1 ページ目は、先頭の 100 バイトがファイルの見出し。
    const header = page === 1 ? start + 100 : start;
    const type = this.view.getUint8(header);
    const cells = this.view.getUint16(header + 3);
    if (type === 0x05) {
      // 内部のページ：セルは（左の子のページ、rowid）。最後に右端の子。
      for (let i = 0; i < cells; i++) {
        const cell = start + this.view.getUint16(header + 12 + i * 2);
        yield* this.scan(this.view.getUint32(cell), depth + 1);
      }
      yield* this.scan(this.view.getUint32(header + 8), depth + 1);
    } else if (type === 0x0d) {
      // 葉のページ：セルは（レコードの大きさ、rowid、レコード）。
      for (let i = 0; i < cells; i++) {
        let pos = start + this.view.getUint16(header + 8 + i * 2);
        const [payloadSize, sizeLength] = this.varint(pos);
        pos += sizeLength;
        const [rowid, rowidLength] = this.varint(pos);
        pos += rowidLength;
        yield { rowid, values: this.record(this.payload(pos, payloadSize)) };
      }
    } else {
      throw new Error(`表の B-tree のページではありません（${page} ページ目、種類 ${type}）`);
    }
  }

  /** セルのレコードを、あふれたページ（overflow）を含めて集める。 */
  private payload(pos: number, size: number): Uint8Array {
    const usable = this.usableSize;
    const maxLocal = usable - 35;
    if (size <= maxLocal) return this.bytes.subarray(pos, pos + size);
    const minLocal = Math.floor(((usable - 12) * 32) / 255) - 23;
    const k = minLocal + ((size - minLocal) % (usable - 4));
    const local = k <= maxLocal ? k : minLocal;
    const out = new Uint8Array(size);
    out.set(this.bytes.subarray(pos, pos + local));
    let written = local;
    let next = this.view.getUint32(pos + local);
    while (written < size) {
      if (next === 0) throw new Error("あふれたページが途中で終わっています");
      const offset = this.pageOffset(next);
      const chunk = Math.min(size - written, usable - 4);
      out.set(this.bytes.subarray(offset + 4, offset + 4 + chunk), written);
      written += chunk;
      next = this.view.getUint32(offset);
    }
    return out;
  }

  /** 可変長の整数（最大 9 バイト）を読み、値と長さを返す。 */
  private varint(pos: number): [number, number] {
    let value = 0n;
    for (let i = 0; i < 8; i++) {
      const byte = this.view.getUint8(pos + i);
      value = (value << 7n) | BigInt(byte & 0x7f);
      if (byte < 0x80) return [Number(BigInt.asIntN(64, value)), i + 1];
    }
    value = (value << 8n) | BigInt(this.view.getUint8(pos + 8));
    return [Number(BigInt.asIntN(64, value)), 9];
  }

  /** レコード（見出しの大きさ、各列の型、値）を読む。 */
  private record(payload: Uint8Array): Value[] {
    const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength);
    const readVarint = (pos: number): [number, number] => {
      let value = 0n;
      for (let i = 0; i < 8; i++) {
        const byte = view.getUint8(pos + i);
        value = (value << 7n) | BigInt(byte & 0x7f);
        if (byte < 0x80) return [Number(value), i + 1];
      }
      return [Number((value << 8n) | BigInt(view.getUint8(pos + 8))), 9];
    };
    const [headerSize, headerLength] = readVarint(0);
    const types: number[] = [];
    for (let pos = headerLength; pos < headerSize; ) {
      const [type, length] = readVarint(pos);
      types.push(type);
      pos += length;
    }
    const values: Value[] = [];
    let pos = headerSize;
    for (const type of types) {
      if (type === 0) {
        values.push(null);
      } else if (type >= 1 && type <= 6) {
        const size = [0, 1, 2, 3, 4, 6, 8][type];
        let value = 0n;
        for (let i = 0; i < size; i++) value = (value << 8n) | BigInt(view.getUint8(pos + i));
        values.push(Number(BigInt.asIntN(size * 8, value)));
        pos += size;
      } else if (type === 7) {
        values.push(view.getFloat64(pos));
        pos += 8;
      } else if (type === 8 || type === 9) {
        values.push(type - 8);
      } else if (type >= 12) {
        const size = Math.floor((type - (type % 2 === 0 ? 12 : 13)) / 2);
        const bytes = payload.subarray(pos, pos + size);
        values.push(type % 2 === 0 ? bytes.slice() : this.decoder.decode(bytes));
        pos += size;
      } else {
        throw new Error(`知らない列の型です: ${type}`);
      }
    }
    return values;
  }
}

/** 文字列を、SQLite の既定の比べ方（BINARY、UTF-8 のバイトの順）で比べる。 */
export function compareBinary(a: string, b: string): number {
  // UTF-8 のバイトの順は、コードポイントの順と同じ。
  const x = Array.from(a);
  const y = Array.from(b);
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const d = x[i].codePointAt(0)! - y[i].codePointAt(0)!;
    if (d !== 0) return d;
  }
  return x.length - y.length;
}
