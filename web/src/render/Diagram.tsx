import { useEffect, useState } from "react";
import { whenIdle } from "../lazyLoader";
import { useNearViewport } from "../nearViewport";
import { mermaidLoader } from "./loaders";

type State = { svg: string } | { error: true } | null;

/** mermaid の図。描画するまでは図の文字列をコードとして表示する。 */
export function Diagram({ source }: { source: string }) {
  const [state, setState] = useState<State>(null);
  // 図が画面の近くに来たときに、初めて mermaid を読み込む（ADR 0038）。
  const [ref, near] = useNearViewport<HTMLPreElement>();
  useEffect(() => {
    if (!near) return;
    let mounted = true;
    setState(null);
    const cancel = whenIdle(() => {
      mermaidLoader
        .load()
        .then((mermaid) => mermaid.renderDiagram(source))
        .then(
          (svg) => mounted && setState({ svg }),
          () => mounted && setState({ error: true }),
        );
    });
    return () => {
      mounted = false;
      cancel();
    };
  }, [source, near]);

  if (state && "svg" in state) {
    return <div className="mermaid-diagram" dangerouslySetInnerHTML={{ __html: state.svg }} />;
  }
  return (
    <>
      {state && <p className="diagram-error">図を描画できませんでした。</p>}
      <pre ref={ref}>
        <code className="language-mermaid">{source}</code>
      </pre>
    </>
  );
}
