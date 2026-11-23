import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { api, Analytics, AuthResult, ShortUrl, shortUrl } from "./api";

type Session = AuthResult;

function App() {
  const [session, setSession] = useState<Session | null>(() => {
    const token = localStorage.getItem("rusturl_token");
    const email = localStorage.getItem("rusturl_email");
    return token && email ? { token, email } : null;
  });

  const signOut = useCallback(() => {
    localStorage.removeItem("rusturl_token");
    localStorage.removeItem("rusturl_email");
    setSession(null);
  }, []);

  const signIn = useCallback((result: AuthResult) => {
    localStorage.setItem("rusturl_token", result.token);
    localStorage.setItem("rusturl_email", result.email);
    setSession(result);
  }, []);

  return session ? (
    <Dashboard session={session} onSignOut={signOut} />
  ) : (
    <AuthScreen onAuthenticated={signIn} />
  );
}

function AuthScreen({
  onAuthenticated,
}: {
  onAuthenticated: (session: AuthResult) => void;
}) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      onAuthenticated(await api.authenticate(mode === "login" ? "login" : "register", email, password));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Something went wrong.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="auth-page">
      <div className="auth-orbit orbit-one" />
      <div className="auth-orbit orbit-two" />
      <section className="auth-card">
        <Brand />
        <p className="eyebrow">YOUR LINKS, IN GOOD HANDS</p>
        <h1>{mode === "login" ? "Welcome back." : "Make every link count."}</h1>
        <p className="auth-subtitle">
          {mode === "login"
            ? "Sign in to pick up where you left off."
            : "Create an account and start sharing smarter."}
        </p>
        <form className="auth-form" onSubmit={submit}>
          <label>
            Email address
            <input
              autoComplete="email"
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder="you@company.com"
              required
            />
          </label>
          <label>
            Password
            <input
              autoComplete={mode === "login" ? "current-password" : "new-password"}
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder="At least 8 characters"
              minLength={8}
              maxLength={128}
              required
            />
          </label>
          {error && <p className="alert error-alert">{error}</p>}
          <button className="button primary wide" disabled={busy}>
            {busy ? "Please wait…" : mode === "login" ? "Sign in" : "Create account"}
            {!busy && <span aria-hidden="true">→</span>}
          </button>
        </form>
        <p className="auth-switch">
          {mode === "login" ? "New to Linkly?" : "Already have an account?"}{" "}
          <button
            className="text-button"
            onClick={() => {
              setMode(mode === "login" ? "register" : "login");
              setError("");
            }}
          >
            {mode === "login" ? "Create an account" : "Sign in"}
          </button>
        </p>
        <p className="auth-footnote">Simple links. Useful insights. No noise.</p>
      </section>
    </main>
  );
}

function Dashboard({
  session,
  onSignOut,
}: {
  session: Session;
  onSignOut: () => void;
}) {
  const [urls, setUrls] = useState<ShortUrl[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [query, setQuery] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [selectedCode, setSelectedCode] = useState<string | null>(null);

  const loadUrls = useCallback(async () => {
    try {
      const result = await api.listUrls(session.token);
      setUrls(result);
      setError("");
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : "Unable to load links.";
      if (message.includes("token")) onSignOut();
      else setError(message);
    } finally {
      setLoading(false);
    }
  }, [session.token, onSignOut]);

  useEffect(() => {
    void loadUrls();
  }, [loadUrls]);

  const filteredUrls = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return urls;
    return urls.filter(
      (url) =>
        url.short_code.toLowerCase().includes(needle) ||
        url.original_url.toLowerCase().includes(needle),
    );
  }, [query, urls]);

  const totalClicks = urls.reduce((sum, url) => sum + url.clicks, 0);
  const bestUrl = [...urls].sort((a, b) => b.clicks - a.clicks)[0];

  async function removeUrl(url: ShortUrl) {
    if (!window.confirm(`Delete ${url.short_code}? Its short link will stop working.`)) return;
    try {
      await api.deleteUrl(session.token, url.short_code);
      setUrls((current) => current.filter((item) => item.id !== url.id));
      setNotice("Link deleted.");
      if (selectedCode === url.short_code) setSelectedCode(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Unable to delete this link.");
    }
  }

  async function copyLink(code: string) {
    try {
      await navigator.clipboard.writeText(shortUrl(code));
      setNotice("Short link copied to clipboard.");
    } catch {
      setError("Clipboard access is unavailable in this browser.");
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <Brand />
        <div className="workspace-label">WORKSPACE</div>
        <button className="nav-item active">
          <span className="nav-icon">↗</span> Links
          <span className="nav-count">{urls.length}</span>
        </button>
        <div className="sidebar-bottom">
          <div className="sidebar-tip">
            <span className="tip-icon">✳</span>
            <strong>Short & sweet</strong>
            <p>Every great campaign starts with a link that’s easy to share.</p>
          </div>
          <div className="profile">
            <div className="avatar">{session.email.slice(0, 1).toUpperCase()}</div>
            <div className="profile-email" title={session.email}>{session.email}</div>
            <button className="icon-button signout" onClick={onSignOut} title="Sign out" aria-label="Sign out">↪</button>
          </div>
        </div>
      </aside>

      <main className="main-content">
        <header className="topbar">
          <div className="breadcrumbs"><span>Workspace</span><span className="crumb-slash">/</span><strong>Links</strong></div>
          <div className="topbar-right"><span className="status-dot" /> All systems operational</div>
        </header>

        <div className="page-content">
          <section className="page-heading">
            <div>
              <p className="eyebrow">OVERVIEW</p>
              <h1>Your links, at a glance.</h1>
              <p className="page-subtitle">Create, organize, and track your short links.</p>
            </div>
            <button className="button primary" onClick={() => setShowCreate(true)}>
              <span className="plus">+</span> Create a link
            </button>
          </section>

          {error && <p className="alert error-alert page-alert">{error}</p>}
          {notice && <p className="alert success-alert page-alert">{notice}<button onClick={() => setNotice("")}>×</button></p>}

          <section className="stats-grid" aria-label="Link statistics">
            <StatCard label="Total links" value={urls.length.toLocaleString()} icon="↗" tone="violet" detail="Across your workspace" />
            <StatCard label="Total clicks" value={totalClicks.toLocaleString()} icon="⌁" tone="blue" detail="All-time link visits" />
            <StatCard label="Top link" value={bestUrl ? `/${bestUrl.short_code}` : "—"} icon="★" tone="amber" detail={bestUrl ? `${bestUrl.clicks.toLocaleString()} clicks` : "Your most visited link"} />
          </section>

          <section className="links-panel">
            <div className="panel-heading">
              <div>
                <h2>All links <span className="count-pill">{urls.length}</span></h2>
                <p>Your shortened URLs and their performance.</p>
              </div>
              <label className="search-field">
                <span aria-hidden="true">⌕</span>
                <input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search links…" />
                <kbd>⌘ K</kbd>
              </label>
            </div>
            {loading ? (
              <div className="empty-state"><div className="loader" />Loading your links…</div>
            ) : filteredUrls.length === 0 ? (
              <div className="empty-state">
                <div className="empty-icon">↗</div>
                <strong>{query ? "No links match your search" : "Your first link is waiting"}</strong>
                <p>{query ? "Try another URL or short code." : "Turn a long URL into something worth sharing."}</p>
                {!query && <button className="button secondary" onClick={() => setShowCreate(true)}>Create your first link</button>}
              </div>
            ) : (
              <div className="table-scroll">
                <table>
                  <thead><tr><th>SHORT LINK</th><th>DESTINATION</th><th>CLICKS</th><th>CREATED</th><th><span className="sr-only">Actions</span></th></tr></thead>
                  <tbody>
                    {filteredUrls.map((url) => (
                      <tr key={url.id}>
                        <td>
                          <div className="short-link-cell">
                            <span className="link-mark">↗</span>
                            <button className="short-code" onClick={() => setSelectedCode(url.short_code)}>{url.short_code}</button>
                            <button className="copy-button" onClick={() => void copyLink(url.short_code)} title="Copy short link" aria-label={`Copy ${url.short_code}`}>▢</button>
                          </div>
                        </td>
                        <td><a className="destination" href={url.original_url} target="_blank" rel="noreferrer">{url.original_url}</a></td>
                        <td><span className="clicks">{url.clicks.toLocaleString()}</span></td>
                        <td className="date-cell">{new Date(url.created_at).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })}</td>
                        <td>
                          <div className="row-actions">
                            <button className="icon-button" onClick={() => setSelectedCode(url.short_code)} title="View analytics" aria-label={`View analytics for ${url.short_code}`}>▥</button>
                            <button className="icon-button danger-hover" onClick={() => void removeUrl(url)} title="Delete link" aria-label={`Delete ${url.short_code}`}>×</button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
            <footer className="panel-footer">
              <span>Showing <strong>{filteredUrls.length}</strong> of <strong>{urls.length}</strong> links</span>
              <span>Clicks are counted without collecting visitor information.</span>
            </footer>
          </section>
          <footer className="page-footer"><span>© {new Date().getFullYear()} Linkly</span><span>Built for links that go places.</span></footer>
        </div>
      </main>

      {showCreate && (
        <CreateLinkModal
          token={session.token}
          onClose={() => setShowCreate(false)}
          onCreated={(created) => {
            const newUrl: ShortUrl = { ...created, is_active: true, clicks: 0 };
            setUrls((current) => [newUrl, ...current]);
            setShowCreate(false);
            setNotice("Your short link is ready.");
            void navigator.clipboard.writeText(created.short_url).catch(() => undefined);
          }}
        />
      )}
      {selectedCode && (
        <AnalyticsModal
          code={selectedCode}
          token={session.token}
          onClose={() => setSelectedCode(null)}
        />
      )}
    </div>
  );
}

function CreateLinkModal({
  token,
  onClose,
  onCreated,
}: {
  token: string;
  onClose: () => void;
  onCreated: (url: Awaited<ReturnType<typeof api.createUrl>>) => void;
}) {
  const [originalUrl, setOriginalUrl] = useState("");
  const [alias, setAlias] = useState("");
  const [expiresAt, setExpiresAt] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function submit(event: FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      const values: { original_url: string; custom_alias?: string; expires_at?: string } = {
        original_url: originalUrl,
      };
      if (alias.trim()) values.custom_alias = alias.trim();
      if (expiresAt) values.expires_at = new Date(expiresAt).toISOString();
      onCreated(await api.createUrl(token, values));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Unable to create link.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal title="Create a short link" onClose={onClose}>
      <p className="modal-intro">Give your long URL a shorter, more memorable home.</p>
      <form className="modal-form" onSubmit={submit}>
        <label>Destination URL<input type="url" required placeholder="https://example.com/your/very/long/link" value={originalUrl} onChange={(event) => setOriginalUrl(event.target.value)} /></label>
        <label>Custom alias <span className="optional">OPTIONAL</span><div className="alias-input"><span>linkly/</span><input maxLength={32} placeholder="summer-sale" value={alias} onChange={(event) => setAlias(event.target.value)} /></div></label>
        <label>Expiration date <span className="optional">OPTIONAL</span><input type="datetime-local" min={new Date(Date.now() + 60_000).toISOString().slice(0, 16)} value={expiresAt} onChange={(event) => setExpiresAt(event.target.value)} /></label>
        {error && <p className="alert error-alert">{error}</p>}
        <div className="modal-actions"><button type="button" className="button secondary" onClick={onClose}>Cancel</button><button className="button primary" disabled={busy}>{busy ? "Creating…" : "Create link"} <span aria-hidden="true">→</span></button></div>
      </form>
    </Modal>
  );
}

function AnalyticsModal({
  code,
  token,
  onClose,
}: {
  code: string;
  token: string;
  onClose: () => void;
}) {
  const [analytics, setAnalytics] = useState<Analytics | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    api.analytics(token, code)
      .then((result) => { if (active) setAnalytics(result); })
      .catch((reason: unknown) => {
        if (active) setError(reason instanceof Error ? reason.message : "Unable to load analytics.");
      });
    return () => { active = false; };
  }, [code, token]);

  return (
    <Modal title={`Analytics for /${code}`} onClose={onClose}>
      {error ? <p className="alert error-alert">{error}</p> : analytics ? (
        <div className="analytics-content">
          <div className="analytics-total"><span>Total clicks</span><strong>{analytics.total_clicks.toLocaleString()}</strong></div>
          <h3>Daily clicks <span className="muted-caption">LAST 30 DAYS</span></h3>
          {analytics.daily_clicks.length === 0 ? <p className="muted-text">No clicks recorded yet. Share your link to get started.</p> : (
            <div className="daily-list">
              {analytics.daily_clicks.map((day) => (
                <div className="daily-row" key={day.day}>
                  <span>{new Date(`${day.day}T00:00:00`).toLocaleDateString(undefined, { month: "short", day: "numeric" })}</span>
                  <div className="bar-track"><div className="bar-fill" style={{ width: `${Math.max(4, (day.clicks / Math.max(...analytics.daily_clicks.map((item) => item.clicks))) * 100)}%` }} /></div>
                  <strong>{day.clicks}</strong>
                </div>
              ))}
            </div>
          )}
          <p className="privacy-note">Only timestamps are recorded. No visitor IP addresses or personal details are stored.</p>
        </div>
      ) : <div className="empty-state compact"><div className="loader" />Loading analytics…</div>}
    </Modal>
  );
}

function Modal({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: React.ReactNode;
}) {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="modal-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="modal-card" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <div className="modal-heading"><div><p className="eyebrow">RUSTURL</p><h2 id="modal-title">{title}</h2></div><button className="icon-button modal-close" onClick={onClose} aria-label="Close dialog">×</button></div>
        {children}
      </section>
    </div>
  );
}

function StatCard({
  label,
  value,
  icon,
  tone,
  detail,
}: {
  label: string;
  value: string;
  icon: string;
  tone: string;
  detail: string;
}) {
  return (
    <article className="stat-card">
      <div className={`stat-icon ${tone}`}>{icon}</div>
      <p>{label}</p>
      <strong title={value}>{value}</strong>
      <span>{detail}</span>
    </article>
  );
}

function Brand() {
  return <div className="brand"><div className="brand-mark">l<span>.</span></div><span>link<span className="brand-light">ly</span></span></div>;
}

export default App;
