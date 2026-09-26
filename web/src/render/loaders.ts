import { createLoader } from "../lazyLoader";

export const katexLoader = createLoader(() => import("./katex"));
export const mermaidLoader = createLoader(() => import("./mermaid"));
