import { useEffect, useState } from "react";
import { mermaidLoader } from "./loaders";

type State = { svg: string } | { error: true } | null;

/** mermaid の図。描画するまでは図の文字列をコードとして表示する。 */
export function Diagram({ source }: { source: string }) {
  const [state, setState] = useState<State>(null);
  useEffect(() => {
    let mounted = true;
    setState(null);
    mermaidLoader
      .load()
      .then((mermaid) => mermaid.renderDiagram(source))
      .then(
        (svg) => mounted && setState({ svg }),
        () => mounted && setState({ error: true }),
      );
    return () => {
      mounted = false;
    };
  }, [source]);

  if (state && "svg" in state) {
    return <div className="mermaid-diagram" dangerouslySetInnerHTML={{ __html: state.svg }} />;
  }
  return (
    <>
      {state && <p className="diagram-error">図を描画できませんでした。</p>}
      <pre>
        <code className="language-mermaid">{source}</code>
      </pre>
    </>
  );
}
