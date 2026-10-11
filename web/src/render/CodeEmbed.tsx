// コードの埋め込み（ADR 0065）。GitHub のファイルと Gist の中身を、CLI がビルドのときに取得して DB に入れたものを描く。
import { Fragment, type CSSProperties } from "react";
import type { CodeEmbedFile } from "../db";
import { EXTERNAL_LINK_PROPS, isExternal } from "../externalLink";
import { languageOfFile } from "../highlightLanguages";
import { useHighlightedBlock } from "../useHighlightedBlock";

/** 埋め込みに付けるクラス。 */
export const CODE_EMBED_CLASS = "code-embed";
export const CODE_EMBED_SOURCE_CLASS = "code-embed-source";
export const CODE_EMBED_CODE_CLASS = "code-embed-code";

/** 見出しの、ファイル名の後に書く出どころ（GitHub のファイルは `<owner>/<repo>` と行の範囲、Gist は `Gist`）。 */
export function sourceLabel(url: string, file: CodeEmbedFile): string {
  const parsed = new URL(url);
  if (parsed.host === "gist.github.com") return "Gist";
  const [owner, repo] = parsed.pathname.split("/").filter(Boolean);
  if (!/^#L\d/.test(parsed.hash)) return `${owner}/${repo}`;
  const end = file.startLine + file.content.split("\n").length - 1;
  return `${owner}/${repo} ${end === file.startLine ? `L${file.startLine}` : `L${file.startLine}-L${end}`}`;
}

/**
 * 行番号を描くための pre の style（ADR 0065）。番号は CSS のカウンタで描くので、コードを写したときに番号が混ざらない。
 * 番号の欄の幅は、最後の行の番号の桁数に合わせる。
 */
export function lineNumberStyle(startLine: number, lineCount: number): CSSProperties {
  const last = startLine + Math.max(lineCount, 1) - 1;
  return { counterReset: `line ${startLine - 1}`, "--line-number-width": `${String(last).length}ch` } as CSSProperties;
}

function EmbeddedFile({ url, file }: { url: string; file: CodeEmbedFile }) {
  const language = languageOfFile(file.fileName);
  const languageClass = language === undefined ? undefined : `language-${language}`;
  const [ref, block] = useHighlightedBlock(file.content, languageClass, language !== undefined);
  const lines = file.content.split("\n");
  const style = lineNumberStyle(file.startLine, lines.length);
  return (
    <figure className={CODE_EMBED_CLASS}>
      <figcaption>
        <a href={file.sourceUrl} {...(isExternal(file.sourceUrl) ? EXTERNAL_LINK_PROPS : {})}>
          {file.fileName}
          <span className={CODE_EMBED_SOURCE_CLASS}>{sourceLabel(url, file)}</span>
        </a>
      </figcaption>
      {block ? (
        <pre
          ref={ref}
          className={`${block.className} ${CODE_EMBED_CODE_CLASS}`}
          style={{ ...block.style, ...style }}
          tabIndex={0}
          dangerouslySetInnerHTML={{ __html: block.html }}
        />
      ) : (
        // 色付けするまでも、Shiki と同じく行ごとに span.line で囲み、行番号を出す。
        <pre ref={ref} className={CODE_EMBED_CODE_CLASS} style={style} tabIndex={0}>
          <code>
            {lines.map((line, index) => (
              <Fragment key={index}>
                <span className="line">{line}</span>
                {index < lines.length - 1 ? "\n" : null}
              </Fragment>
            ))}
          </code>
        </pre>
      )}
    </figure>
  );
}

/** URL だけの行の、GitHub のファイルか Gist の中身。ファイルごとに、見出しとコードブロックを出す。 */
export function CodeEmbed({ url, files }: { url: string; files: readonly CodeEmbedFile[] }) {
  return (
    <>
      {files.map((file) => (
        <EmbeddedFile key={file.sourceUrl} url={url} file={file} />
      ))}
    </>
  );
}
