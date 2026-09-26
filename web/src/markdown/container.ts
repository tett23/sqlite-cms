// :::message と :::details（ADR 0025）。
// Zenn（markdown-it-container）と同じく、開きの行から、開き以上の長さの ::: だけの行までを中身とし、
// 中身を Markdown として読み直す。入れ子にするときは外側のコロンを増やす（::::details の中に :::message）。
import type { Root, RootContent } from "mdast";
import type { Extension } from "micromark-util-types";
import { fenceConstruct, fenceFromMarkdown, type FenceTokens } from "./fence";
import type { ZennContainer } from "./types";

const COLON = 58;

/** メッセージの枠に付けるクラス。sanitize で許可する。 */
export const MESSAGE_CLASS = "message";
export const MESSAGE_ALERT_CLASS = "message-alert";

/** :::details に見出しがないときの見出し。 */
export const DEFAULT_DETAILS_TITLE = "詳細";

const tokens: FenceTokens = {
  block: "zennContainer",
  fence: "zennContainerFence",
  sequence: "zennContainerFenceSequence",
  info: "zennContainerFenceInfo",
  value: "zennContainerValue",
  lineEnding: "zennContainerLineEnding",
};

interface ContainerInfo {
  kind: ZennContainer["kind"];
  alert: boolean;
  title: string;
}

/** 開きの行の :::の後の文字列を読む。知らない名前なら null（その行は段落として読む）。 */
export function parseContainerInfo(info: string): ContainerInfo | null {
  const [, name, rest = ""] = info.match(/^(\S+)(?:\s+(.*))?$/) ?? [];
  if (name === "message") {
    const variant = rest.trim();
    if (variant === "") return { kind: "message", alert: false, title: "" };
    if (variant === "alert") return { kind: "message", alert: true, title: "" };
    return null;
  }
  if (name === "details") return { kind: "details", alert: false, title: rest.trim() };
  return null;
}

export const containerSyntax: Extension = {
  flow: {
    [COLON]: fenceConstruct({
      name: "zennContainer",
      marker: COLON,
      minSize: 3,
      tokens,
      acceptInfo: (info) => parseContainerInfo(info) !== null,
    }),
  },
};

export const containerFromMarkdown = fenceFromMarkdown<ZennContainer>(
  tokens,
  () => ({ type: "zennContainer", kind: "message", alert: false, title: "", value: "", children: [] }),
  (node, { info, value }) => {
    const parsed = parseContainerInfo(info)!;
    node.kind = parsed.kind;
    node.alert = parsed.alert;
    node.title = parsed.title;
    node.value = value;
  },
);

/**
 * コンテナの中身を Markdown として読み直し、HTML の要素を割り当てる。
 * - :::message → <div class="message" role="note">
 * - :::message alert → <div class="message message-alert" role="note">
 * - :::details 見出し → <details><summary>見出し</summary>…</details>
 *
 * 中身は別の文書として読むので、外側に書いた脚注やリンクの定義は中から参照できない。
 */
export function expandContainers(tree: { children: RootContent[] }, parse: (markdown: string) => Root) {
  for (const child of tree.children) {
    if (child.type === "zennContainer") {
      const inner = parse(child.value);
      expandContainers(inner, parse);
      child.value = "";
      child.children = inner.children as ZennContainer["children"];
      if (child.kind === "details") {
        child.data = { hName: "details", hProperties: {} };
        child.children.unshift({
          type: "zennSummary",
          data: { hName: "summary" },
          children: [{ type: "text", value: child.title || DEFAULT_DETAILS_TITLE }],
        } as never);
      } else {
        child.data = {
          hName: "div",
          hProperties: {
            className: child.alert ? [MESSAGE_CLASS, MESSAGE_ALERT_CLASS] : [MESSAGE_CLASS],
            role: "note",
          },
        };
      }
    } else if ("children" in child) {
      expandContainers(child as { children: RootContent[] }, parse);
    }
  }
}
