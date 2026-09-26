// Zenn を参考にした Markdown の拡張（ADR 0025）をまとめた remark のプラグイン。
import type { Root } from "mdast";
import type { Extension as FromMarkdownExtension } from "mdast-util-from-markdown";
import type { Extension as MicromarkExtension } from "micromark-util-types";
import type { Processor } from "unified";
import { containerFromMarkdown, containerSyntax, expandContainers } from "./container";
import { mathFromMarkdown, mathSyntax } from "./math";
import { transformCodeBlocks, transformImageSize, transformInlineFootnotes, transformLinkCards } from "./transforms";
import "./types";

interface ParserData {
  micromarkExtensions?: MicromarkExtension[];
  fromMarkdownExtensions?: Array<FromMarkdownExtension | FromMarkdownExtension[]>;
}

export interface ExtensionOptions {
  /** リンクカードの URL に対応する画像のパス（ADR 0028）。なければ画像なしのカードにする。 */
  linkCardImage?: (url: string) => string | undefined;
}

export function remarkExtensions(this: Processor, options: ExtensionOptions = {}) {
  const data = this.data() as ParserData;
  (data.micromarkExtensions ??= []).push(containerSyntax, mathSyntax);
  (data.fromMarkdownExtensions ??= []).push(containerFromMarkdown, mathFromMarkdown);
  const processor = this;

  return (tree: Root) => {
    // コンテナの中身を先に読み直し、中身にも以降の変換を効かせる。
    expandContainers(tree, (markdown) => processor.parse(markdown) as Root);
    transformCodeBlocks(tree);
    transformInlineFootnotes(tree);
    transformImageSize(tree);
    transformLinkCards(tree, options.linkCardImage);
  };
}
