import {
  createContext,
  useContext,
  useEffect,
  useState,
  type AnchorHTMLAttributes,
  type MouseEvent,
  type ReactNode,
} from "react";

import { stripBasePath, withBasePath } from "./base";

const NAVIGATE_EVENT = "sqlite-cms:navigate";

const PathContext = createContext("/");

export function Router({ path: fixedPath, children }: { path?: string; children: ReactNode }) {
  const [path, setPath] = useState(() => fixedPath ?? stripBasePath(window.location.pathname));

  useEffect(() => {
    if (fixedPath !== undefined) return;
    const sync = () => setPath(stripBasePath(window.location.pathname));
    window.addEventListener("popstate", sync);
    window.addEventListener(NAVIGATE_EVENT, sync);
    return () => {
      window.removeEventListener("popstate", sync);
      window.removeEventListener(NAVIGATE_EVENT, sync);
    };
  }, [fixedPath]);

  return <PathContext value={fixedPath ?? path}>{children}</PathContext>;
}

export function usePath(): string {
  return useContext(PathContext);
}

/** サイトの中のパス（"/about" など）に移る。URL にはサイトを置くパスを付ける。 */
export function navigate(to: string) {
  window.history.pushState(null, "", withBasePath(to));
  window.dispatchEvent(new Event(NAVIGATE_EVENT));
}

export function matchPath(pattern: string, path: string): Record<string, string> | null {
  const normalized = path.length > 1 ? path.replace(/\/+$/, "") : path;
  const patternParts = pattern.split("/");
  const pathParts = normalized.split("/");
  if (patternParts.length !== pathParts.length) return null;

  const params: Record<string, string> = {};
  for (const [i, part] of patternParts.entries()) {
    const segment = pathParts[i];
    if (part.startsWith(":")) {
      if (segment === "") return null;
      try {
        params[part.slice(1)] = decodeURIComponent(segment);
      } catch {
        return null;
      }
    } else if (part !== segment) {
      return null;
    }
  }
  return params;
}

export interface ClickLike {
  button: number;
  metaKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  defaultPrevented: boolean;
}

export function shouldNavigateInApp(event: ClickLike, target: string | undefined): boolean {
  return (
    !event.defaultPrevented &&
    event.button === 0 &&
    !event.metaKey &&
    !event.ctrlKey &&
    !event.shiftKey &&
    !event.altKey &&
    (target === undefined || target === "_self")
  );
}

export function Link({ to, onClick, ...props }: { to: string } & AnchorHTMLAttributes<HTMLAnchorElement>) {
  const handleClick = (event: MouseEvent<HTMLAnchorElement>) => {
    onClick?.(event);
    if (!shouldNavigateInApp(event, typeof props.target === "string" ? props.target : undefined)) return;
    event.preventDefault();
    navigate(to);
  };
  return <a {...props} href={withBasePath(to)} onClick={handleClick} />;
}
