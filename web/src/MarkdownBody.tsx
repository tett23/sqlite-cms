import { useEffect, useState } from "react";
import Markdown, { type Components, type Options } from "react-markdown";
import remarkGfm from "remark-gfm";
import { highlightLoader, type Highlight } from "./highlightLoader";
import { Link } from "./router";

const remarkPlugins: Options["remarkPlugins"] = [remarkGfm];

const remarkRehypeOptions: Options["remarkRehypeOptions"] = {
  footnoteLabel: "脚注",
  footnoteLabelProperties: {},
  footnoteBackLabel: "本文に戻る",
  footnoteBackContent: "↩︎",
};

/** シンタックスハイライトを読み込み、読み込み終わったら描き直す。読み込むまでコードブロックは色なし。 */
function useHighlight(): Highlight | null {
  const [highlight, setHighlight] = useState(highlightLoader.loaded);
  useEffect(() => {
    if (highlight) return;
    let mounted = true;
    highlightLoader.load().then(
      (loaded) => mounted && setHighlight(loaded),
      (error: unknown) => console.error("シンタックスハイライトを読み込めませんでした", error),
    );
    return () => {
      mounted = false;
    };
  }, [highlight]);
  return highlight;
}

function isInternal(href: string | undefined): href is string {
  return href !== undefined && href.startsWith("/") && !href.startsWith("//");
}

const components: Components = {
  a({ node: _node, href, ...props }) {
    return isInternal(href) ? <Link to={href} {...props} /> : <a href={href} {...props} />;
  },
};

export function MarkdownBody({ source, className = "mt-6" }: { source: string; className?: string }) {
  const highlight = useHighlight();
  return (
    <div className={`article-body ${className}`}>
      <Markdown
        skipHtml
        remarkPlugins={remarkPlugins}
        remarkRehypeOptions={remarkRehypeOptions}
        rehypePlugins={highlight?.rehypePlugins}
        components={components}
      >
        {source}
      </Markdown>
    </div>
  );
}
