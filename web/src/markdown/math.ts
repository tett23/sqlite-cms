// 数式の記法（ADR 0025）。Zenn と同じく $$ で囲んだブロックと、$ で囲んだインラインの数式を読む。
// ここでは数式の文字列を取り出すだけで、描画は KaTeX を非同期で読み込んでから行う（math/katex.ts）。
import type { Extension as FromMarkdownExtension } from "mdast-util-from-markdown";
import type { Code, Construct, Effects, Extension, State, TokenizeContext } from "micromark-util-types";
import { asciiDigit, fenceConstruct, fenceFromMarkdown, markdownLineEnding, markdownSpace, type FenceTokens } from "./fence";
import type { InlineMath, MathBlock } from "./types";

const DOLLAR = 36;
const BACKSLASH = 92;

/** 数式のブロックとインラインの数式に付けるクラス。sanitize で許可し、描画の部品が探す。 */
export const MATH_DISPLAY_CLASS = "math-display";
export const MATH_INLINE_CLASS = "math-inline";

const flowTokens: FenceTokens = {
  block: "mathFlow",
  fence: "mathFlowFence",
  sequence: "mathFlowFenceSequence",
  info: "mathFlowFenceInfo",
  value: "mathFlowValue",
  lineEnding: "mathFlowLineEnding",
};

// $$ の行には何も書かない。$$x$$ のように同じ行に書いたものは段落として読む。
const mathFlow = fenceConstruct({
  name: "mathFlow",
  marker: DOLLAR,
  minSize: 2,
  tokens: flowTokens,
  acceptInfo: (info) => info === "",
});

/**
 * $ で囲んだインラインの数式。金額などの $ を数式と取り違えにくくするため、markdown-it-katex と同じ規則にする。
 * - 開きの $ の直後は空白でない。
 * - 開きの後の最初の $ だけを閉じの候補にする。\$ は候補にならない。
 * - 閉じの $ の直前は空白でなく、直後は数字でない。満たさなければ数式にしない（$5 と $10 を数式にしない）。
 * - 改行をまたがない。
 */
const mathText: Construct = { name: "mathText", tokenize: tokenizeMathText };

function tokenizeMathText(this: TokenizeContext, effects: Effects, ok: State, nok: State): State {
  let previous: Code = null;
  return start;

  function start(code: Code): State | undefined {
    effects.enter("mathText");
    effects.enter("mathTextSequence");
    effects.consume(code);
    effects.exit("mathTextSequence");
    return open;
  }

  function open(code: Code): State | undefined {
    if (code === null || code === DOLLAR || markdownSpace(code) || markdownLineEnding(code)) return nok(code);
    effects.enter("mathTextData");
    return data(code);
  }

  function data(code: Code): State | undefined {
    if (code === null || markdownLineEnding(code)) return nok(code);
    if (code === DOLLAR) {
      if (markdownSpace(previous)) return nok(code);
      effects.exit("mathTextData");
      effects.enter("mathTextSequence");
      effects.consume(code);
      effects.exit("mathTextSequence");
      return closeAfter;
    }
    previous = code;
    effects.consume(code);
    return code === BACKSLASH ? escaped : data;
  }

  function escaped(code: Code): State | undefined {
    if (code === null || markdownLineEnding(code)) return nok(code);
    previous = code;
    effects.consume(code);
    return data;
  }

  function closeAfter(code: Code): State | undefined {
    if (asciiDigit(code)) return nok(code);
    effects.exit("mathText");
    return ok(code);
  }
}

export const mathSyntax: Extension = {
  flow: { [DOLLAR]: mathFlow },
  text: { [DOLLAR]: mathText },
};

const flowFromMarkdown = fenceFromMarkdown<MathBlock>(
  flowTokens,
  () => ({ type: "math", value: "" }),
  (node, { value }) => {
    node.value = value;
    node.data = {
      hName: "div",
      hProperties: { className: [MATH_DISPLAY_CLASS] },
      hChildren: [{ type: "text", value }],
    };
  },
);

const textFromMarkdown: FromMarkdownExtension = {
  enter: {
    mathText(token) {
      this.enter({ type: "inlineMath", value: "" } satisfies InlineMath as never, token);
    },
  },
  exit: {
    mathTextData(token) {
      const node = this.stack[this.stack.length - 1] as unknown as InlineMath;
      node.value = this.sliceSerialize(token);
    },
    mathText(token) {
      const node = this.stack[this.stack.length - 1] as unknown as InlineMath;
      node.data = {
        hName: "span",
        hProperties: { className: [MATH_INLINE_CLASS] },
        hChildren: [{ type: "text", value: node.value }],
      };
      this.exit(token);
    },
  },
};

export const mathFromMarkdown: FromMarkdownExtension[] = [flowFromMarkdown, textFromMarkdown];
