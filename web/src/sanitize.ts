import { defaultSchema, type Options as Schema } from "rehype-sanitize";

/** Markdown と GFM が作る要素。外すと Markdown の出力が壊れる。 */
const MARKDOWN_TAGS = [
  "a",
  "blockquote",
  "br",
  "code",
  "del",
  "em",
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "hr",
  "img",
  "input",
  "li",
  "ol",
  "p",
  "pre",
  "section",
  "strong",
  "sup",
  "table",
  "tbody",
  "td",
  "th",
  "thead",
  "tr",
  "ul",
];

/** Markdown では書けないので、本文に HTML で書くことを許す要素（ADR 0018）。 */
export const EXTRA_TAGS = [
  "details",
  "summary",
  "kbd",
  "sub",
  "sup",
  "ruby",
  "rt",
  "rp",
  "mark",
  "abbr",
  "dl",
  "dt",
  "dd",
  "br",
];

type AttributeDefinitions = NonNullable<Schema["attributes"]>;

function withoutPlainId(definitions: AttributeDefinitions): AttributeDefinitions {
  return Object.fromEntries(
    Object.entries(definitions).map(([tag, attributes]) => [
      tag,
      attributes.filter((attribute) => attribute !== "id" && !(Array.isArray(attribute) && attribute[0] === "id")),
    ]),
  );
}

const baseAttributes = withoutPlainId(defaultSchema.attributes ?? {});

export const sanitizeSchema: Schema = {
  ...defaultSchema,
  tagNames: [...new Set([...MARKDOWN_TAGS, ...EXTRA_TAGS])],
  // 脚注の ID には remark-rehype がすでに user-content- を付けている。ここで重ねて付けるとリンクが壊れる。
  clobberPrefix: "",
  attributes: {
    ...baseAttributes,
    // 本文の要素がページ内の要素と同じ ID を持たないよう、脚注が作る ID だけを許す。
    "*": [...(baseAttributes["*"] ?? []), ["id", /^user-content-/, "footnote-label"]],
    abbr: ["title"],
    details: ["open"],
  },
  protocols: { ...defaultSchema.protocols, href: ["http", "https", "mailto"] },
};
