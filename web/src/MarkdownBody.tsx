import Markdown, { type Components, type Options } from "react-markdown";
import { Link } from "react-router";
import remarkGfm from "remark-gfm";

const remarkPlugins: Options["remarkPlugins"] = [remarkGfm];

const remarkRehypeOptions: Options["remarkRehypeOptions"] = {
  footnoteLabel: "脚注",
  footnoteLabelProperties: {},
  footnoteBackLabel: "本文に戻る",
  footnoteBackContent: "↩︎",
};

function isInternal(href: string | undefined): href is string {
  return href !== undefined && href.startsWith("/") && !href.startsWith("//");
}

const components: Components = {
  a({ node: _node, href, ...props }) {
    return isInternal(href) ? <Link to={href} {...props} /> : <a href={href} {...props} />;
  },
};

export function MarkdownBody({ source, className = "mt-6" }: { source: string; className?: string }) {
  return (
    <div className={`article-body ${className}`}>
      <Markdown
        remarkPlugins={remarkPlugins}
        remarkRehypeOptions={remarkRehypeOptions}
        components={components}
      >
        {source}
      </Markdown>
    </div>
  );
}
