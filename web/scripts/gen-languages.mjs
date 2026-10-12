// Shiki の言語の id と別名から、本体に置く言語名の一覧（src/highlightLanguageNames.generated.ts）を作る（ADR 0067）。
// Shiki を上げて言語が変わったら、単体テストが落ちるので、npm run gen:languages で作り直す。
import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { bundledLanguagesInfo } from "shiki/langs";

const ids = {};
for (const { id, aliases = [] } of [...bundledLanguagesInfo].sort((a, b) => a.id.localeCompare(b.id))) {
  ids[id] = id;
  for (const alias of aliases) ids[alias] ??= id;
}
const entries = Object.entries(ids).map(([name, id]) => `  ${JSON.stringify(name)}: ${JSON.stringify(id)},`);
const source = `// npm run gen:languages で作る（scripts/gen-languages.mjs、ADR 0067）。手で書き換えない。

/** Shiki の言語の名前と別名から、言語の id への対応。Shiki を読み込む前に、色を付けるかを判断するために本体に持つ。 */
export const LANGUAGE_IDS: Readonly<Record<string, string>> = {
${entries.join("\n")}
};
`;
await writeFile(fileURLToPath(new URL("../src/highlightLanguageNames.generated.ts", import.meta.url)), source);
console.log(`${Object.keys(ids).length} の名前を書きました`);
