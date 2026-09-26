import { useEffect, useSyncExternalStore } from "react";

/**
 * 大きなライブラリを、本体とは別のチャンクにして、必要になったときに初めて読み込む（ADR 0029）。
 * 読み込みを始めるのは、それを使う部品（コードブロック、数式、図）が表示されたときだけにする。
 */
export interface LazyLoader<T> {
  /** 読み込み済みなら返す。まだなら null。 */
  loaded(): T | null;
  /** 読み込みを始める（読み込み中なら同じ Promise を返す）。失敗したら次の呼び出しで読み込み直す。 */
  load(): Promise<T>;
  /** 読み込み終わったときに呼ぶ関数を登録する。登録を解く関数を返す。 */
  subscribe(listener: () => void): () => void;
}

export function createLoader<T>(importer: () => Promise<T>): LazyLoader<T> {
  let loaded: T | null = null;
  let loading: Promise<T> | null = null;
  const listeners = new Set<() => void>();
  return {
    loaded: () => loaded,
    load() {
      loading ??= importer().then(
        (module) => {
          loaded = module;
          for (const listener of listeners) listener();
          return module;
        },
        (error: unknown) => {
          loading = null;
          throw error;
        },
      );
      return loading;
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

function afterIdle(fn: () => void): () => void {
  if (typeof requestIdleCallback === "function") {
    const id = requestIdleCallback(fn, { timeout: 2000 });
    return () => cancelIdleCallback(id);
  }
  const id = setTimeout(fn, 200);
  return () => clearTimeout(id);
}

/**
 * いま描いている内容が画面に出て、ブラウザの手が空いたときに fn を呼ぶ（ADR 0038）。取り消す関数を返す。
 * 色分け、数式、図のライブラリは、本文が描かれた後に読み込めば足りる。
 * 本文の描画（LCP）より前に大きなチャンクの取得を始めると、回線を取り合って本文の表示が遅れる。
 * 手が空くのを待つだけでは、本文を描く前に手が空いて取得が始まることがあるので、次の描画（requestAnimationFrame の後）を待ってから待つ。
 */
export function whenIdle(fn: () => void): () => void {
  if (typeof requestAnimationFrame !== "function") return afterIdle(fn);
  let cancel: () => void;
  const frame = requestAnimationFrame(() => {
    // requestAnimationFrame はその描画の直前に呼ばれるので、描画の後に回す。
    const id = setTimeout(() => {
      cancel = afterIdle(fn);
    }, 0);
    cancel = () => clearTimeout(id);
  });
  cancel = () => cancelAnimationFrame(frame);
  return () => cancel();
}

const turns: (() => void)[] = [];

function takeTurn() {
  turns.shift()?.();
  if (turns.length > 0) setTimeout(takeTurn, 0);
}

/**
 * fn を、ほかの fn と一つずつ、別のタスクで順に呼ぶ（ADR 0038）。取り消す関数を返す。
 * コードブロックの色分けは一つずつなら短いが、画面にあるブロックをまとめて一度に色分けすると長いタスクになり、入力への応答（TBT）が遅れる。
 */
export function inTurn(fn: () => void): () => void {
  const turn = () => fn();
  turns.push(turn);
  if (turns.length === 1) setTimeout(takeTurn, 0);
  return () => {
    const index = turns.indexOf(turn);
    if (index >= 0) turns.splice(index, 1);
  };
}

/** 読み込み済みなら返し、読み込み終わったら描き直す。読み込みは始めない。 */
export function useLoaded<T>(loader: LazyLoader<T>): T | null {
  return useSyncExternalStore(loader.subscribe, loader.loaded);
}

/**
 * 表示されたときに、ブラウザの手が空くのを待って読み込みを始め（when が false なら始めない）、読み込み終わったら描き直す。読み込むまでは null。
 * 読み込めなかったときは onError を呼び、null のままにする（読み込み前の表示が残る）。
 */
export function useLazy<T>(loader: LazyLoader<T>, onError?: (error: unknown) => void, when = true): T | null {
  const module = useLoaded(loader);
  useEffect(() => {
    if (module || !when) return;
    return whenIdle(() => {
      loader.load().catch((error: unknown) => onError?.(error));
    });
  }, [loader, module, onError, when]);
  return module;
}
