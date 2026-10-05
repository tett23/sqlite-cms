// 長い本文の先頭の部分（ADR 0051）。最初の画面には先頭の部分だけを解析して描き、全体の解析は画面に出た後に行う。
// 先頭の部分を描いた結果が、全体を描いた結果の先頭と同じになるところでだけ切る。

/** 先頭の部分の、最低の文字数。最初の画面（狭い画面で 1500 字ほど）より十分に多くする。 */
export const FIRST_PART_MIN_CHARS = 2000;

/**
 * リンクの参照の定義と脚注の定義（`[名前]: …`、`[^1]: …`）。先頭の部分より後ろにあると、先頭の部分だけでは参照を解決できないので、
 * 先頭の部分の後ろに足す。リストや引用の中のもの（字下げ、> やリストの印の後ろ）も見つけるため、広めに探す。
 */
const DEFINITION = /^(?:[ \t>]|[*+-][ \t]|\d{1,9}[.)][ \t])*\[[^\]\n]+\]:/;
/** 先頭の部分に足せる定義。ブロックの外で、列の先頭から始まるもの。 */
const TOP_LEVEL_DEFINITION = /^ {0,3}\[[^\]\n]+\]:/;

/** コードのフェンス（``` か ~~~）の開き。 */
const CODE_FENCE = /^ {0,3}(`{3,}|~{3,})/;

/** :::message などの開き（ADR 0025）と閉じ。 */
const CONTAINER_OPEN = /^ {0,3}(:{3,})[ \t]*\S/;
const CONTAINER_CLOSE = /^ {0,3}(:{3,})[ \t]*$/;

/** 空行では終わらない HTML のブロック（CommonMark の種類 1 から 5）の開き。 */
const HTML_BLOCK_OPEN = /^ {0,3}<(?:(script|pre|style|textarea)(?=[\s>]|$)|(!--)|(\?)|(![A-Za-z])|(!\[CDATA\[))/i;
const HTML_BLOCK_CLOSE = [/<\/(?:script|pre|style|textarea)>/i, /-->/, /\?>/, />/, /\]\]>/];

/** 列の先頭から始まる ATX の見出し。見出しは、リスト、引用、表、段落を終わらせ、遅延継続にもならない。 */
const HEADING = /^#{1,6}(?:[ \t]|$)/;

/**
 * 本文の先頭の部分。minChars 文字目以降で、どのブロックの中でもない、空行の後の見出しの直前までに、
 * それより後ろの定義（一行で、前後が空行のもの）を足して返す。
 * そこで切れないとき（見出しがない、足せない形の定義が後ろにある）は null を返し、呼び出し側は全体を描く。
 */
export function firstPart(source: string, minChars = FIRST_PART_MIN_CHARS): string | null {
  if (source.length <= minChars) return null;
  const lines = source.split("\n");
  let cut: number | null = null;
  const definitions: string[] = [];
  let fence: { char: string; length: number } | null = null;
  let math = false;
  const containers: number[] = [];
  let htmlClose: RegExp | null = null;
  let previousBlank = true;
  let previousDefinition = false;
  let offset = 0;
  for (const [index, line] of lines.entries()) {
    const start = offset;
    offset += line.length + 1;
    const blank = line.trim() === "";
    const wasBlank = previousBlank;
    const afterDefinition = previousDefinition;
    previousBlank = blank;
    previousDefinition = false;
    const inBlock = fence !== null || htmlClose !== null || math;
    if (cut !== null && !inBlock && DEFINITION.test(line)) {
      // 足せるのは、ブロックの外の、前後が空行か定義の一行の定義だけ。
      const next = lines[index + 1] ?? "";
      const alone = (wasBlank || afterDefinition) && (next.trim() === "" || TOP_LEVEL_DEFINITION.test(next));
      if (containers.length > 0 || !TOP_LEVEL_DEFINITION.test(line) || !alone) return null;
      definitions.push(line);
      previousDefinition = true;
      continue;
    }
    if (fence) {
      const close = line.match(CODE_FENCE);
      if (close && close[1][0] === fence.char && close[1].length >= fence.length && line.slice(line.indexOf(close[1]) + close[1].length).trim() === "") {
        fence = null;
      }
      continue;
    }
    if (htmlClose) {
      if (htmlClose.test(line)) htmlClose = null;
      continue;
    }
    if (math) {
      if (line.trim() === "$$") math = false;
      continue;
    }
    if (cut === null && start >= minChars && wasBlank && containers.length === 0 && HEADING.test(line)) {
      cut = start;
    }
    const open = line.match(CODE_FENCE);
    if (open) {
      fence = { char: open[1][0], length: open[1].length };
      continue;
    }
    if (line.trim().startsWith("$$")) {
      // 同じ行で閉じていなければ、閉じの $$ の行までが数式。
      const rest = line.trim().slice(2);
      if (!rest.includes("$$")) math = true;
      continue;
    }
    const html = line.match(HTML_BLOCK_OPEN);
    if (html) {
      const kind = html.slice(1).findIndex((group) => group !== undefined);
      if (!HTML_BLOCK_CLOSE[kind].test(line.slice(html[0].length))) htmlClose = HTML_BLOCK_CLOSE[kind];
      continue;
    }
    const containerClose = line.match(CONTAINER_CLOSE);
    if (containerClose && containers.length > 0) {
      // 開き以上の長さのコロンで閉じる。
      let index = containers.length - 1;
      while (index >= 0 && containerClose[1].length < containers[index]) index--;
      if (index >= 0) containers.length = index;
      continue;
    }
    const containerOpen = line.match(CONTAINER_OPEN);
    if (containerOpen) containers.push(containerOpen[1].length);
  }
  if (cut === null) return null;
  const head = source.slice(0, cut);
  return definitions.length === 0 ? head : `${head}${definitions.join("\n\n")}\n`;
}
