import { useEffect, useMemo, useState, type ReactNode } from "react";
import type { Database } from "sql.js";
import { getArticle, getLinkCardImages, getPage, getPost, getSite, listArticles, listPosts, loadDb } from "./db";
import { pageDescription, setMetaDescription } from "./documentMeta";
import { MarkdownBody } from "./MarkdownBody";
import { withBasePath } from "./base";
import { Link, matchPath, usePath } from "./router";
import { search, type SearchKind } from "./search";

let dbPromise: Promise<Database> | null = null;
function getDb(): Promise<Database> {
  dbPromise ??= loadDb();
  return dbPromise;
}

function useDb(): { db?: Database; error?: string } {
  const [state, setState] = useState<{ db?: Database; error?: string }>({});
  useEffect(() => {
    let mounted = true;
    getDb().then(
      (db) => mounted && setState({ db }),
      (e: unknown) => mounted && setState({ error: e instanceof Error ? e.message : String(e) }),
    );
    return () => {
      mounted = false;
    };
  }, []);
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

function initialQuery(): string {
  return typeof window === "undefined" ? "" : (new URLSearchParams(window.location.search).get("q") ?? "");
}

/** 全文検索（ADR 0031）。入力に合わせて結果を出し、言葉を URL（?q=）に残す。 */
function SearchPage() {
  const { db, error } = useDb();
  const [query, setQuery] = useState(initialQuery);
  useDocumentMeta(db, "検索");
  const results = useMemo(() => (db && query.trim() ? search(db, query) : null), [db, query]);

  useEffect(() => {
    const url = withBasePath("/search") + (query.trim() ? `?q=${encodeURIComponent(query.trim())}` : "");
    window.history.replaceState(null, "", url);
  }, [query]);

  return (
    <div>
      <h1 className="mb-4 text-xl font-bold">検索</h1>
      <form role="search" onSubmit={(event) => event.preventDefault()} className="mb-6">
        <label htmlFor="search-query" className="mr-2 text-sm">
          探す言葉
        </label>
        <input
          id="search-query"
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
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

const ROUTES: [string, (params: Record<string, string>) => ReactNode][] = [
  ["/", () => <Home />],
  ["/posts/:slug", ({ slug }) => <PostPage slug={slug} />],
  ["/articles/:slug", ({ slug }) => <ArticlePage slug={slug} />],
  ["/about", () => <AboutPage />],
  ["/search", () => <SearchPage />],
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
  const { db } = useDb();
  const site = db ? getSite(db) : null;
  const path = usePath();
  useEffect(() => {
    window.scrollTo(0, 0);
  }, [path]);

  return (
    <div className="mx-auto max-w-2xl px-4 py-8">
      <header className="mb-8 flex items-baseline justify-between border-b border-black pb-2">
        <Link to="/" className="site-title text-xl font-bold">
          {site?.title ?? "\u00a0"}
        </Link>
        <nav className="space-x-4 text-sm">
          <Link to="/">トップ</Link>
          <Link to="/about">自己紹介</Link>
          <Link to="/search">検索</Link>
        </nav>
      </header>
      <main>
        <CurrentPage />
      </main>
      {(site?.author || site?.license) && (
        <footer className="mt-16 border-t border-gray-400 pt-2 text-sm text-gray-600">
          {site.author && <p>{site.author}</p>}
          {site.license && (
            <p>
              ライセンス:{" "}
              {site.license.url ? <a href={site.license.url}>{site.license.name}</a> : site.license.name}
            </p>
          )}
        </footer>
      )}
    </div>
  );
}
