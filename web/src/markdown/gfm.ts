// GFM（表、取り消し線、タスクリスト、自動リンク、脚注）の解析だけを登録する remark のプラグイン（ADR 0051）。
// remark-gfm は、Markdown への書き出しの拡張（mdast-util-to-markdown）も登録するので、使わない書き出しの処理が本体の JS に入る。
import type { Extension as FromMarkdownExtension } from "mdast-util-from-markdown";
import { gfmFromMarkdown } from "mdast-util-gfm";
import { gfm } from "micromark-extension-gfm";
import type { Extension as MicromarkExtension } from "micromark-util-types";
import type { Processor } from "unified";

interface ParserData {
  micromarkExtensions?: MicromarkExtension[];
  fromMarkdownExtensions?: Array<FromMarkdownExtension | FromMarkdownExtension[]>;
}

export function remarkGfmParse(this: Processor) {
  const data = this.data() as ParserData;
  (data.micromarkExtensions ??= []).push(gfm());
  (data.fromMarkdownExtensions ??= []).push(gfmFromMarkdown());
}
