import { useEffect, useMemo, useState, type KeyboardEvent, type ReactNode } from "react";
import type { Database } from "sql.js";
import { getArticle, getLinkCardImages, getPage, getPost, getSite, listAll, listArticles, listPosts, loadDb } from "./db";
import { pageDescription, setMetaDescription } from "./documentMeta";
import { MarkdownBody } from "./MarkdownBody";
import { withBasePath } from "./base";
import { Link, matchPath, navigate, usePath, useSearch } from "./router";
import { search, searchPath, type SearchKind } from "./search";
import { moveActive, suggest } from "./suggest";

let dbPromise: Promise<Database> | null = null;
/** 読み込み終わった DB。読み込み済みなら、部品は最初の描画から使う。 */
let loadedDb: Database | undefined;
function getDb(): Promise<Database> {
  dbPromise ??= loadDb().then((db) => (loadedDb = db));
  return dbPromise;
}

/**
 * DB を読み込み、読み込み終わったら描き直す。
 * 読み込み済みなら最初の描画から返す。読み込み中の表示を一度描いてから本文に替えると、その下の要素（フッタ）がずれ、ページを移るたびに読み込み中の表示が一瞬見える。
 */
function useDb(): { db?: Database; error?: string } {
  const [state, setState] = useState<{ db?: Database; error?: string }>(() => (loadedDb ? { db: loadedDb } : {}));
  useEffect(() => {
    if (state.db) return;
    let mounted = true;
    getDb().then(
      (db) => mounted && setState({ db }),
      (e: unknown) => mounted && setState({ error: e instanceof Error ? e.message : String(e) }),
    );
    return () => {
      mounted = false;
    };
  }, [state.db]);
  return state;
}

/** ページのタイトルと meta description を設定する。 */
function useDocumentMeta(db: Database | undefined, title: string | null, description?: string | null) {
  useEffect(() => {
    if (!db) return;
    const site = getSite(db);
    document.title = title ? `${title} - ${site.title}` : site.title;
    setMetaDescription(document, pageDescription(site.description, description));
  }, [db, title, description]);
}

function Loading({ error }: { error?: string }) {
  if (error) return <p role="alert">読み込みに失敗しました: {error}</p>;
  return (
    <p className="loading" role="status">
      読み込み中
    </p>
  );
}

function Home() {
  const { db, error } = useDb();
  useDocumentMeta(db, null);
  if (!db) return <Loading error={error} />;

  const { homeMd } = getSite(db);
  const articles = listArticles(db);
  const posts = listPosts(db);

  return (
    <div>
      {homeMd && <MarkdownBody source={homeMd} className="mb-8" linkCardImages={getLinkCardImages(db)} />}

      <section className="mb-10">
        <h2 className="mb-3 text-lg font-bold">記事</h2>
        {articles.length === 0 ? (
          <p>まだありません。</p>
        ) : (
          <ul className="space-y-4">
            {articles.map((a) => (
              <li key={a.slug}>
                <Link to={`/articles/${a.slug}`}>{a.title}</Link>
                <span className="ml-3 text-sm text-gray-600">
                  <time>{a.publishedAt}</time>
                </span>
                {a.description && <p className="mt-1 text-sm text-gray-700">{a.description}</p>}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section>
        <h2 className="mb-3 text-lg font-bold">ブログ</h2>
        {posts.length === 0 ? (
          <p>まだありません。</p>
        ) : (
          <ul className="space-y-2">
            {posts.map((p) => (
              <li key={p.slug}>
                <time className="mr-3 text-gray-600">{p.publishedAt}</time>
                <Link to={`/posts/${p.slug}`}>{p.title}</Link>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function PostPage({ slug }: { slug: string }) {
  const { db, error } = useDb();
  const post = db ? getPost(db, slug) : null;
  useDocumentMeta(db, post?.title ?? null);
  if (!db) return <Loading error={error} />;
  if (!post) return <NotFound />;

  return (
    <article>
      <h1 className="text-2xl font-bold">{post.title}</h1>
      <p className="mt-1 text-gray-600">
        <time>{post.publishedAt}</time>
      </p>
      <MarkdownBody source={post.bodyMd} linkCardImages={getLinkCardImages(db)} />
    </article>
  );
}

function ArticlePage({ slug }: { slug: string }) {
  const { db, error } = useDb();
  const article = db ? getArticle(db, slug) : null;
  useDocumentMeta(db, article?.title ?? null, article?.description);
  if (!db) return <Loading error={error} />;
  if (!article) return <NotFound />;

  return (
    <article>
      <h1 className="text-2xl font-bold">{article.title}</h1>
      <p className="mt-1 text-gray-600">
        <time>{article.publishedAt}</time>
        {article.updatedAt && <span className="ml-3">（{article.updatedAt} 改稿）</span>}
      </p>
      <MarkdownBody source={article.bodyMd} linkCardImages={getLinkCardImages(db)} />
    </article>
  );
}

function AboutPage() {
  const { db, error } = useDb();
  const page = db ? getPage(db, "about") : null;
  useDocumentMeta(db, page?.title ?? null);
  if (!db) return <Loading error={error} />;
  if (!page) return <NotFound />;

  return (
    <article>
      <h1 className="text-2xl font-bold">{page.title}</h1>
      <MarkdownBody source={page.bodyMd} linkCardImages={getLinkCardImages(db)} />
    </article>
  );
}

function NotFound() {
  const { db } = useDb();
  useDocumentMeta(db, "見つかりません");
  return <p>見つかりません。</p>;
}

const KIND_LABELS: Record<SearchKind, string> = { post: "ブログ", article: "記事", page: "ページ" };

/** 全文検索（ADR 0031）。入力に合わせて結果を出し、言葉を URL（?q=）に残す。 */
function SearchPage() {
  const { db, error } = useDb();
  const urlQuery = new URLSearchParams(useSearch()).get("q") ?? "";
  const [query, setQuery] = useState(urlQuery);
  useDocumentMeta(db, "検索");
  const results = useMemo(() => (db && query.trim() ? search(db, query) : null), [db, query]);

  // 入力に合わせて、履歴を増やさずに URL（?q=）を変える。
  useEffect(() => {
    navigate(searchPath(query), { replace: true });
  }, [query]);
  // ヘッダの検索ボックスやブラウザの戻るで URL が変わったときは、入力欄をそれに合わせる（ADR 0042）。
  useEffect(() => {
    setQuery((current) => (current.trim() === urlQuery ? current : urlQuery));
  }, [urlQuery]);

  return (
    <div>
      <h1 className="mb-4 text-xl font-bold">検索</h1>
      <form role="search" aria-label="検索の条件" onSubmit={(event) => event.preventDefault()} className="mb-6">
        <label htmlFor="search-query" className="mr-2 text-sm">
          探す言葉
        </label>
        <input
          id="search-query"
          type="search"
          value={query}
          onChange={(event) => setQuery(event.currentTarget.value)}
          className="w-64 border border-gray-500 px-2 py-1"
        />
      </form>
      {!db && <Loading error={error} />}
      {results && (
        <>
          <p role="status" className="mb-4 text-sm text-gray-600">
            {results.length} 件
          </p>
          <ul className="space-y-4">
            {results.map((result) => (
              <li key={result.path}>
                <Link to={result.path}>{result.title}</Link>
                <span className="ml-2 text-sm text-gray-600">
                  {KIND_LABELS[result.kind]}
                  {result.date && ` ${result.date}`}
                </span>
                <p className="text-sm text-gray-700">
                  {result.snippet.map((part, i) => (part.match ? <mark key={i}>{part.text}</mark> : part.text))}
                </p>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

const SUGGESTIONS_ID = "header-search-suggestions";
const suggestionId = (index: number) => `header-search-suggestion-${index}`;

/**
 * ヘッダの検索ボックス（ADR 0041、0042）。送ると検索のページ（/search?q=…）に移る。
 * JS が動かなくても、フォームとして /search?q=… を開けば同じ結果になる。
 * 入力中は、候補を SUGGESTION_LIMIT 件まで出し、なければないことを出す（WAI-ARIA のコンボボックス）。
 * 矢印のキーで候補を選び、Enter でその記事に移る。候補を選んでいなければ、検索のページに移る。Escape で候補を閉じる。
 */
function HeaderSearch() {
  const { db } = useDb();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const suggestions = useMemo(() => (db ? suggest(db, query) : []), [db, query]);
  const trimmed = query.trim();
  const shown = open && db !== undefined && trimmed !== "";

  const go = (to: string) => {
    setQuery("");
    setOpen(false);
    setActive(-1);
    navigate(to);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setOpen(true);
      setActive(moveActive(active, event.key === "ArrowDown" ? 1 : -1, suggestions.length));
    } else if (event.key === "Enter" && shown && active >= 0) {
      event.preventDefault();
      go(suggestions[active].path);
    } else if (event.key === "Escape" && shown) {
      event.preventDefault();
      setOpen(false);
      setActive(-1);
    }
  };

  return (
    <form
      role="search"
      aria-label="サイトの中を検索"
      action={withBasePath("/search")}
      method="get"
      onSubmit={(event) => {
        event.preventDefault();
        go(searchPath(query));
      }}
      className="relative flex items-center gap-1"
    >
      <label htmlFor="header-search" className="sr-only">
        サイトの中を検索
      </label>
      <input
        id="header-search"
        type="search"
        name="q"
        role="combobox"
        autoComplete="off"
        aria-autocomplete="list"
        aria-expanded={shown && suggestions.length > 0}
        aria-controls={SUGGESTIONS_ID}
        aria-activedescendant={shown && active >= 0 ? suggestionId(active) : undefined}
        value={query}
        onChange={(event) => {
          setQuery(event.currentTarget.value);
          setOpen(true);
          setActive(-1);
        }}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onKeyDown={onKeyDown}
        className="w-36 border border-gray-500 px-1"
      />
      <button type="submit" className="border border-gray-500 bg-gray-100 px-2">
        検索
      </button>
      {/* 候補を押しても入力欄から焦点を外さない（外すと、押す前に候補が閉じる）。 */}
      <div
        hidden={!shown}
        onMouseDown={(event) => event.preventDefault()}
        className="absolute top-full left-0 z-10 mt-1 w-72 max-w-[calc(100vw-2rem)] border border-gray-500 bg-white sm:right-0 sm:left-auto"
      >
        <ul id={SUGGESTIONS_ID} role="listbox" aria-label="候補" hidden={suggestions.length === 0}>
          {suggestions.map((result, index) => (
            <li
              key={result.path}
              id={suggestionId(index)}
              role="option"
              aria-selected={index === active}
              onClick={() => go(result.path)}
              onMouseEnter={() => setActive(index)}
              className={`cursor-pointer px-2 py-1 ${index === active ? "bg-gray-200" : ""}`}
            >
              <span className="block text-blue-800 underline">{result.title}</span>
              <span className="block text-xs text-gray-600">
                {KIND_LABELS[result.kind]}
                {result.date && ` ${result.date}`}
              </span>
            </li>
          ))}
        </ul>
        {suggestions.length === 0 && <p className="px-2 py-1">「{trimmed}」に一致する記事はありません。</p>}
      </div>
      {/* 候補の数を読み上げる。 */}
      <p role="status" className="sr-only">
        {shown ? (suggestions.length > 0 ? `候補が ${suggestions.length} 件あります` : "一致する記事はありません") : ""}
      </p>
    </form>
  );
}

/** 種別をまたいだ統合一覧（ADR 0033）。post と article を、年ごとに新しい順に並べる。 */
function ArchivePage() {
  const { db, error } = useDb();
  useDocumentMeta(db, "すべての記事");
  if (!db) return <Loading error={error} />;

  const entries = listAll(db);
  const years = [...new Set(entries.map((entry) => entry.publishedAt.slice(0, 4)))];
  return (
    <div>
      <h1 className="mb-4 text-xl font-bold">すべての記事</h1>
      {entries.length === 0 && <p>まだ記事がありません。</p>}
      {years.map((year) => (
        <section key={year} className="mb-8">
          <h2 className="mb-3 border-b border-gray-300 pb-1 text-lg font-bold">{year}</h2>
          <ul className="space-y-2">
            {entries
              .filter((entry) => entry.publishedAt.startsWith(year))
              .map((entry) => (
                <li key={`${entry.kind}/${entry.slug}`}>
                  <span className="mr-2 text-sm text-gray-600">{entry.publishedAt}</span>
                  <Link to={entry.kind === "post" ? `/posts/${entry.slug}` : `/articles/${entry.slug}`}>{entry.title}</Link>
                  <span className="ml-2 text-sm text-gray-600">{entry.kind === "post" ? "ブログ" : "記事"}</span>
                  {entry.description && <p className="text-sm text-gray-700">{entry.description}</p>}
                </li>
              ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

/** CLI が index.html に入れた RSS のフィードの案内（ADR 0033）。なければ null。 */
function feedHref(): string | null {
  if (typeof document === "undefined") return null;
  return document.querySelector('link[rel="alternate"][type="application/rss+xml"]')?.getAttribute("href") ?? null;
}

const FEED_HREF = feedHref();

/**
 * CLI が index.html に入れた、ヘッダを content/header.md で作る目印（ADR 0043）。
 * あれば、DB を読むまでヘッダも本文も描かず、読み込み中の表示だけを出す。
 * 既定のヘッダを先に描くと、DB を読んだときに差し替わる。空のヘッダを先に描くと、ヘッダの中身が入ったときに本文が押し下げられる。
 */
const CUSTOM_HEADER =
  typeof document !== "undefined" &&
  document.querySelector('meta[name="sqlite-cms-header"]')?.getAttribute("content") === "custom";

const ROUTES: [string, (params: Record<string, string>) => ReactNode][] = [
  ["/", () => <Home />],
  ["/posts/:slug", ({ slug }) => <PostPage slug={slug} />],
  ["/articles/:slug", ({ slug }) => <ArticlePage slug={slug} />],
  ["/about", () => <AboutPage />],
  ["/search", () => <SearchPage />],
  ["/archive", () => <ArchivePage />],
];

function CurrentPage() {
  const path = usePath();
  for (const [pattern, render] of ROUTES) {
    const params = matchPath(pattern, path);
    if (params) return render(params);
  }
  return <NotFound />;
}

export default function App() {
  const { db, error } = useDb();
  const site = db ? getSite(db) : null;
  const path = usePath();
  const customHeader = site ? site.headerMd !== null : CUSTOM_HEADER;
  const headerPartials = useMemo(() => ({ search: <HeaderSearch /> }), []);
  useEffect(() => {
    window.scrollTo(0, 0);
  }, [path]);

  if (CUSTOM_HEADER && !db) {
    // index.html の読み込み中の表示（loading-screen）と同じ位置に出す。
    // key を本文の側と変え、DB を読んだときに同じ要素を使い回さずに置き換える（使い回すと、要素が動いたと数えられる）。
    return (
      <div key="loading" className="loading-screen">
        <Loading error={error} />
      </div>
    );
  }

  return (
    <div key="app" className="mx-auto max-w-2xl px-4 py-8">
      {customHeader ? (
        // 記事リポジトリの content/header.md で作ったヘッダ（ADR 0043）。{{> search}} の場所に検索ボックスを置く。
        <header className="mb-8 border-b border-black pb-2">
          {site?.headerMd && (
            <MarkdownBody
              source={site.headerMd}
              baseClassName="site-header-body"
              className=""
              linkCardImages={db ? getLinkCardImages(db) : undefined}
              partials={headerPartials}
            />
          )}
        </header>
      ) : (
        // 狭い画面ではサイト名と案内を別の行にし、サイト名が後から入っても案内が折り返さない（高さが変わらない）ようにする。
        <header className="mb-8 flex flex-col gap-1 border-b border-black pb-2 sm:flex-row sm:items-baseline sm:justify-between">
          <Link to="/" className="site-title text-xl font-bold">
            {site?.title ?? "\u00a0"}
          </Link>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
            <nav className="space-x-4">
              <Link to="/">トップ</Link>
              <Link to="/archive">一覧</Link>
              <Link to="/about">自己紹介</Link>
              <Link to="/search">検索</Link>
            </nav>
            <HeaderSearch />
          </div>
        </header>
      )}
      <main>
        <CurrentPage />
      </main>
      {/* フッターは DB を読んだ後に描く。先に RSS の案内だけを描くと、本文が入ったときに押し下げられて、レイアウトがずれる。 */}
      {site && (site.author || site.license || FEED_HREF) && (
        <footer className="mt-16 border-t border-gray-400 pt-2 text-sm text-gray-600">
          {site.author && <p>{site.author}</p>}
          {site.license && (
            <p>
              ライセンス:{" "}
              {site.license.url ? <a href={site.license.url}>{site.license.name}</a> : site.license.name}
            </p>
          )}
          {FEED_HREF && (
            <p>
              <a href={FEED_HREF}>RSS</a>
            </p>
          )}
        </footer>
      )}
    </div>
  );
}
