// 自前の構文拡張が使うトークンと mdast のノードの型を登録する。
import type { BlockContent, DefinitionContent, Literal, Parent, PhrasingContent } from "mdast";

/** :::message と :::details（ADR 0025）。 */
export interface ZennContainer extends Parent {
  type: "zennContainer";
  kind: "message" | "details";
  /** :::message alert のとき true。 */
  alert: boolean;
  /** :::details の見出し。 */
  title: string;
  /** 中身の Markdown。読み直して children にした後は空にする。 */
  value: string;
  children: Array<BlockContent | DefinitionContent>;
}

/** $$ で囲んだ数式。 */
export interface MathBlock extends Literal {
  type: "math";
}

/** $ で囲んだ数式。 */
export interface InlineMath extends Literal {
  type: "inlineMath";
}

/** ファイル名付きのコードブロック（figure）と、そのファイル名（figcaption）。 */
export interface CodeFigure extends Parent {
  type: "codeFigure";
  children: Array<CodeFigureCaption | BlockContent>;
}

export interface CodeFigureCaption extends Parent {
  type: "codeFigureCaption";
  children: PhrasingContent[];
}

/** :::details の見出し（summary）。 */
export interface ZennSummary extends Parent {
  type: "zennSummary";
  children: PhrasingContent[];
}

declare module "mdast" {
  interface RootContentMap {
    zennContainer: ZennContainer;
    zennSummary: ZennSummary;
    math: MathBlock;
    inlineMath: InlineMath;
    codeFigure: CodeFigure;
    codeFigureCaption: CodeFigureCaption;
  }
  interface BlockContentMap {
    zennContainer: ZennContainer;
    math: MathBlock;
    codeFigure: CodeFigure;
  }
  interface PhrasingContentMap {
    inlineMath: InlineMath;
  }
}

declare module "micromark-util-types" {
  interface TokenTypeMap {
    zennContainer: "zennContainer";
    zennContainerFence: "zennContainerFence";
    zennContainerFenceSequence: "zennContainerFenceSequence";
    zennContainerFenceInfo: "zennContainerFenceInfo";
    zennContainerValue: "zennContainerValue";
    zennContainerLineEnding: "zennContainerLineEnding";
    mathFlow: "mathFlow";
    mathFlowFence: "mathFlowFence";
    mathFlowFenceSequence: "mathFlowFenceSequence";
    mathFlowFenceInfo: "mathFlowFenceInfo";
    mathFlowValue: "mathFlowValue";
    mathFlowLineEnding: "mathFlowLineEnding";
    mathText: "mathText";
    mathTextSequence: "mathTextSequence";
    mathTextData: "mathTextData";
  }
}
