import { useEffect, useState } from "react";
import { Link, Route, Routes, useLocation, useParams } from "react-router";
import type { Database } from "sql.js";
import { getArticle, getPage, getPost, listArticles, listPosts, loadDb } from "./db";
import { MarkdownBody } from "./MarkdownBody";

const SITE_TITLE = "tett23の記事置き場";

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

function useDocumentTitle(title: string | null) {
  useEffect(() => {
    document.title = title ? `${title} - ${SITE_TITLE}` : SITE_TITLE;
    return () => {
      document.title = SITE_TITLE;
    };
  }, [title]);
}

function Loading({ error }: { error?: string }) {
  return error ? <p>読み込みに失敗しました: {error}</p> : <p>読み込み中…</p>;
}

function Home() {
  const { db, error } = useDb();
  useDocumentTitle(null);
  if (!db) return <Loading error={error} />;

  const articles = listArticles(db);
  const posts = listPosts(db);

  return (
    <div>
      <p className="mb-8">
        tett23 の記事置き場。長めの読み物は記事に、日々の雑多なものはブログに置いている。
      </p>

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

function PostPage() {
  const { slug } = useParams<{ slug: string }>();
  const { db, error } = useDb();
  const post = db && slug ? getPost(db, slug) : null;
  useDocumentTitle(post?.title ?? null);
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

function ArticlePage() {
  const { slug } = useParams<{ slug: string }>();
  const { db, error } = useDb();
  const article = db && slug ? getArticle(db, slug) : null;
  useDocumentTitle(article?.title ?? null);
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
  useDocumentTitle(page?.title ?? null);
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
  return <p>見つかりません。</p>;
}

export default function App() {
  const location = useLocation();
  useEffect(() => {
    window.scrollTo(0, 0);
  }, [location.pathname]);

  return (
    <div className="mx-auto max-w-2xl px-4 py-8">
      <header className="mb-8 flex items-baseline justify-between border-b border-black pb-2">
        <Link to="/" className="site-title text-xl font-bold">
          {SITE_TITLE}
        </Link>
        <nav className="space-x-4 text-sm">
          <Link to="/">トップ</Link>
          <Link to="/about">自己紹介</Link>
        </nav>
      </header>
      <main>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/posts/:slug" element={<PostPage />} />
          <Route path="/articles/:slug" element={<ArticlePage />} />
          <Route path="/about" element={<AboutPage />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </main>
      <footer className="mt-16 border-t border-gray-400 pt-2 text-sm text-gray-600">
        <p>tett23</p>
      </footer>
    </div>
  );
}
