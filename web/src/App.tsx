import { useEffect, useState, type ReactNode } from "react";
import type { Database } from "sql.js";
import { getArticle, getPage, getPost, getSite, listArticles, listPosts, loadDb } from "./db";
import { MarkdownBody } from "./MarkdownBody";
import { Link, matchPath, usePath } from "./router";

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

function useDocumentTitle(db: Database | undefined, title: string | null) {
  useEffect(() => {
    if (!db) return;
    const siteTitle = getSite(db).title;
    document.title = title ? `${title} - ${siteTitle}` : siteTitle;
  }, [db, title]);
}

function Loading({ error }: { error?: string }) {
  return error ? <p>読み込みに失敗しました: {error}</p> : <p>読み込み中…</p>;
}

function Home() {
  const { db, error } = useDb();
  useDocumentTitle(db, null);
  if (!db) return <Loading error={error} />;

  const { homeMd } = getSite(db);
  const articles = listArticles(db);
  const posts = listPosts(db);

  return (
    <div>
      {homeMd && <MarkdownBody source={homeMd} className="mb-8" />}

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
  useDocumentTitle(db, post?.title ?? null);
  if (!db) return <Loading error={error} />;
  if (!post) return <NotFound />;

  return (
    <article>
      <h1 className="text-2xl font-bold">{post.title}</h1>
      <p className="mt-1 text-gray-600">
        <time>{post.publishedAt}</time>
      </p>
      <MarkdownBody source={post.bodyMd} />
    </article>
  );
}

function ArticlePage({ slug }: { slug: string }) {
  const { db, error } = useDb();
  const article = db ? getArticle(db, slug) : null;
  useDocumentTitle(db, article?.title ?? null);
  if (!db) return <Loading error={error} />;
  if (!article) return <NotFound />;

  return (
    <article>
      <h1 className="text-2xl font-bold">{article.title}</h1>
      <p className="mt-1 text-gray-600">
        <time>{article.publishedAt}</time>
        {article.updatedAt && <span className="ml-3">（{article.updatedAt} 改稿）</span>}
      </p>
      <MarkdownBody source={article.bodyMd} />
    </article>
  );
}

function AboutPage() {
  const { db, error } = useDb();
  const page = db ? getPage(db, "about") : null;
  useDocumentTitle(db, page?.title ?? null);
  if (!db) return <Loading error={error} />;
  if (!page) return <NotFound />;

  return (
    <article>
      <h1 className="text-2xl font-bold">{page.title}</h1>
      <MarkdownBody source={page.bodyMd} />
    </article>
  );
}

function NotFound() {
  const { db } = useDb();
  useDocumentTitle(db, "見つかりません");
  return <p>見つかりません。</p>;
}

const ROUTES: [string, (params: Record<string, string>) => ReactNode][] = [
  ["/", () => <Home />],
  ["/posts/:slug", ({ slug }) => <PostPage slug={slug} />],
  ["/articles/:slug", ({ slug }) => <ArticlePage slug={slug} />],
  ["/about", () => <AboutPage />],
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
