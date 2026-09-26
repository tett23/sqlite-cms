// micromark の構文拡張の共通部品。
// コードブロック（```）と同じ規則で、:::（コンテナ）と $$（数式）のフェンスを読む。
import type { CompileContext, Extension as FromMarkdownExtension } from "mdast-util-from-markdown";
import type { Code, Construct, Effects, State, TokenizeContext, TokenType } from "micromark-util-types";

// micromark の文字コード。改行は負の値（CR: -5、LF: -4、CRLF: -3）、タブは -2、タブを埋める仮想の空白は -1。
export function markdownLineEnding(code: Code): boolean {
  return code !== null && code < -2;
}

export function markdownSpace(code: Code): boolean {
  return code === -2 || code === -1 || code === 32;
}

export function asciiDigit(code: Code): boolean {
  return code !== null && code >= 48 && code <= 57;
}

/** 空白を読み飛ばす。max を指定すると、その数未満の空白だけを読む（micromark-factory-space と同じ規則）。 */
export function factorySpace(effects: Effects, ok: State, type: TokenType, max?: number): State {
  const limit = max ? max - 1 : Number.POSITIVE_INFINITY;
  let size = 0;
  return start;

  function start(code: Code): State | undefined {
    if (markdownSpace(code)) {
      effects.enter(type);
      return prefix(code);
    }
    return ok(code);
  }

  function prefix(code: Code): State | undefined {
    if (markdownSpace(code) && size++ < limit) {
      effects.consume(code);
      return prefix;
    }
    effects.exit(type);
    return ok(code);
  }
}

/** フェンスの構文が使うトークンの名前。 */
export interface FenceTokens {
  block: TokenType;
  fence: TokenType;
  sequence: TokenType;
  info: TokenType;
  value: TokenType;
  lineEnding: TokenType;
}

export interface FenceSpec {
  name: string;
  /** フェンスの文字（: や $）。 */
  marker: number;
  /** フェンスの最小の長さ。 */
  minSize: number;
  tokens: FenceTokens;
  /** 開きのフェンスの後の文字列を受け付けるか。受け付けなければ、その行は段落などとして読まれる。 */
  acceptInfo(info: string): boolean;
}

const nonLazyContinuation: Construct = { partial: true, tokenize: tokenizeNonLazyContinuation };

/**
 * フェンスで囲んだブロック。micromark-core-commonmark の codeFenced と同じ規則で読む。
 * - 閉じのフェンスは、開きと同じ文字で、開き以上の長さのものだけ。それより短い行は中身になる。
 * - 閉じがなければ、文書（やリストの項目）の終わりまでが中身になる。
 * - 中身は加工せずに読む。開きのフェンスの字下げの分だけ、各行の先頭の空白を取り除く。
 */
export function fenceConstruct(spec: FenceSpec): Construct {
  const { tokens } = spec;
  return { name: spec.name, concrete: true, tokenize };

  function tokenize(this: TokenizeContext, effects: Effects, ok: State, nok: State): State {
    const self = this;
    const closeStart: Construct = { partial: true, tokenize: tokenizeCloseStart };
    let initialPrefix = 0;
    let sizeOpen = 0;
    let info = "";
    return start;

    function start(code: Code): State | undefined {
      const tail = self.events[self.events.length - 1];
      initialPrefix = tail && tail[1].type === "linePrefix" ? tail[2].sliceSerialize(tail[1], true).length : 0;
      effects.enter(tokens.block);
      effects.enter(tokens.fence);
      effects.enter(tokens.sequence);
      return sequenceOpen(code);
    }

    function sequenceOpen(code: Code): State | undefined {
      if (code === spec.marker) {
        sizeOpen++;
        effects.consume(code);
        return sequenceOpen;
      }
      if (sizeOpen < spec.minSize) return nok(code);
      effects.exit(tokens.sequence);
      return factorySpace(effects, infoBefore, "whitespace")(code);
    }

    function infoBefore(code: Code): State | undefined {
      if (code === null || markdownLineEnding(code)) return infoAfter(code);
      effects.enter(tokens.info);
      return infoInside(code);
    }

    function infoInside(code: Code): State | undefined {
      if (code === null || markdownLineEnding(code)) {
        info = self.sliceSerialize(effects.exit(tokens.info));
        return infoAfter(code);
      }
      effects.consume(code);
      return infoInside;
    }

    function infoAfter(code: Code): State | undefined {
      if (!spec.acceptInfo(info.trim())) return nok(code);
      effects.exit(tokens.fence);
      return self.interrupt ? ok(code) : effects.check(nonLazyContinuation, atNonLazyBreak, after)(code);
    }

    function atNonLazyBreak(code: Code): State | undefined {
      return effects.attempt(closeStart, after, contentBefore)(code);
    }

    function contentBefore(code: Code): State | undefined {
      effects.enter(tokens.lineEnding);
      effects.consume(code);
      effects.exit(tokens.lineEnding);
      return contentStart;
    }

    function contentStart(code: Code): State | undefined {
      return initialPrefix > 0 && markdownSpace(code)
        ? factorySpace(effects, beforeContentChunk, "linePrefix", initialPrefix + 1)(code)
        : beforeContentChunk(code);
    }

    function beforeContentChunk(code: Code): State | undefined {
      if (code === null || markdownLineEnding(code)) {
        return effects.check(nonLazyContinuation, atNonLazyBreak, after)(code);
      }
      effects.enter(tokens.value);
      return contentChunk(code);
    }

    function contentChunk(code: Code): State | undefined {
      if (code === null || markdownLineEnding(code)) {
        effects.exit(tokens.value);
        return beforeContentChunk(code);
      }
      effects.consume(code);
      return contentChunk;
    }

    function after(code: Code): State | undefined {
      effects.exit(tokens.block);
      return ok(code);
    }

    function tokenizeCloseStart(this: TokenizeContext, effects: Effects, ok: State, nok: State): State {
      let size = 0;
      return startBefore;

      function startBefore(code: Code): State | undefined {
        effects.enter("lineEnding");
        effects.consume(code);
        effects.exit("lineEnding");
        return start;
      }

      function start(code: Code): State | undefined {
        effects.enter(tokens.fence);
        const indentedCode = !self.parser.constructs.disable.null?.includes("codeIndented");
        return factorySpace(effects, beforeSequenceClose, "linePrefix", indentedCode ? 4 : undefined)(code);
      }

      function beforeSequenceClose(code: Code): State | undefined {
        if (code !== spec.marker) return nok(code);
        effects.enter(tokens.sequence);
        return sequenceClose(code);
      }

      function sequenceClose(code: Code): State | undefined {
        if (code === spec.marker) {
          size++;
          effects.consume(code);
          return sequenceClose;
        }
        if (size < sizeOpen) return nok(code);
        effects.exit(tokens.sequence);
        return factorySpace(effects, sequenceCloseAfter, "whitespace")(code);
      }

      function sequenceCloseAfter(code: Code): State | undefined {
        if (code === null || markdownLineEnding(code)) {
          effects.exit(tokens.fence);
          return ok(code);
        }
        return nok(code);
      }
    }
  }
}

function tokenizeNonLazyContinuation(this: TokenizeContext, effects: Effects, ok: State, nok: State): State {
  const self = this;
  return start;

  function start(code: Code): State | undefined {
    if (code === null) return nok(code);
    effects.enter("lineEnding");
    effects.consume(code);
    effects.exit("lineEnding");
    return lineStart;
  }

  function lineStart(code: Code): State | undefined {
    return self.parser.lazy[self.now().line] ? nok(code) : ok(code);
  }
}

/** フェンスで囲んだブロックを読んだ結果。 */
export interface FenceResult {
  /** 開きのフェンスの後の文字列（前後の空白を除く）。 */
  info: string;
  /** 中身（加工しない）。 */
  value: string;
}

/**
 * フェンスのトークンを mdast のノードにする。
 * create でノードを作り、閉じるときに finish で info と中身を渡す。
 */
export function fenceFromMarkdown<T extends { type: string }>(
  tokens: FenceTokens,
  create: () => T,
  finish: (node: T, result: FenceResult) => void,
): FromMarkdownExtension {
  const state = new WeakMap<object, FenceResult>();

  function current(context: CompileContext): T {
    return context.stack[context.stack.length - 1] as unknown as T;
  }

  return {
    enter: {
      [tokens.block](token) {
        const node = create();
        state.set(node, { info: "", value: "" });
        this.enter(node as never, token);
      },
    },
    exit: {
      [tokens.info](token) {
        state.get(current(this))!.info = this.sliceSerialize(token).trim();
      },
      [tokens.value](token) {
        state.get(current(this))!.value += this.sliceSerialize(token);
      },
      [tokens.lineEnding]() {
        state.get(current(this))!.value += "\n";
      },
      [tokens.block](token) {
        const node = current(this);
        const result = state.get(node)!;
        // 中身は開きのフェンスの改行から始まる。
        finish(node, { info: result.info, value: result.value.replace(/^\n/, "") });
        this.exit(token);
      },
    },
  };
}
