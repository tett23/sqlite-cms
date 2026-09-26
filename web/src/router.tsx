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

/** いまの場所。path はサイトを置くパスを除いたパス、search は `?q=…` などの問い合わせ（なければ空）。 */
interface Location {
  path: string;
  search: string;
}

const LocationContext = createContext<Location>({ path: "/", search: "" });

function currentLocation(): Location {
  return { path: stripBasePath(window.location.pathname), search: window.location.search };
}

export function Router({
  path: fixedPath,
  search: fixedSearch = "",
  children,
}: {
  path?: string;
  search?: string;
  children: ReactNode;
}) {
  const [location, setLocation] = useState<Location>(() =>
    fixedPath !== undefined ? { path: fixedPath, search: fixedSearch } : currentLocation(),
  );

  useEffect(() => {
    if (fixedPath !== undefined) return;
    const sync = () =>
      setLocation((previous) => {
        const next = currentLocation();
        return previous.path === next.path && previous.search === next.search ? previous : next;
      });
    window.addEventListener("popstate", sync);
    window.addEventListener(NAVIGATE_EVENT, sync);
    return () => {
      window.removeEventListener("popstate", sync);
      window.removeEventListener(NAVIGATE_EVENT, sync);
    };
  }, [fixedPath]);

  return <LocationContext value={location}>{children}</LocationContext>;
}

export function usePath(): string {
  return useContext(LocationContext).path;
}

/** URL の問い合わせ（`?q=…`）。ページを移ったときと、navigate で置き換えたときに変わる（ADR 0042）。 */
export function useSearch(): string {
  return useContext(LocationContext).search;
}

/**
 * サイトの中のパス（"/about" など）に移る。URL にはサイトを置くパスを付ける。
 * replace のときは履歴を増やさずに URL を置き換える（検索のページで、入力に合わせて ?q= を変えるときなど）。
 */
export function navigate(to: string, { replace = false }: { replace?: boolean } = {}) {
  const url = withBasePath(to);
  if (replace) window.history.replaceState(null, "", url);
  else window.history.pushState(null, "", url);
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
