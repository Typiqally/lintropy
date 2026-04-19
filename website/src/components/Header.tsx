import { useEffect, useMemo, useState } from "react";
import { Link, NavLink, useLocation } from "react-router-dom";
import { GitFork, Menu, Search, Star, X } from "lucide-react";
import { primaryNav, nav } from "../nav";
import SearchDialog from "./SearchDialog";

function isMacLike(): boolean {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent || "";
  const platform = (navigator as unknown as { platform?: string }).platform || "";
  return /Mac|iPhone|iPod|iPad/i.test(platform + " " + ua);
}

function GithubMark({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0 0 16 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}

type RepoStats = { stars: number; forks: number };

const STATS_CACHE_KEY = "lintropy:gh-stats";
const STATS_TTL_MS = 10 * 60 * 1000;

function formatCount(n: number): string {
  if (n < 1000) return String(n);
  return (n / 1000).toFixed(1).replace(/\.0$/, "") + "k";
}

function readCachedStats(): RepoStats | null {
  try {
    const raw = sessionStorage.getItem(STATS_CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as { at: number; stats: RepoStats };
    if (Date.now() - parsed.at > STATS_TTL_MS) return null;
    return parsed.stats;
  } catch {
    return null;
  }
}

function writeCachedStats(stats: RepoStats): void {
  try {
    sessionStorage.setItem(
      STATS_CACHE_KEY,
      JSON.stringify({ at: Date.now(), stats }),
    );
  } catch {
    // ignore quota / disabled storage
  }
}

function isActiveSection(to: string, path: string): boolean {
  if (to === "/") return path === "/";
  // Match the first path segment so that, e.g., /configuration counts as
  // Reference active even though the link itself points at /configuration.
  const seg = (p: string) => p.split("/").filter(Boolean)[0] ?? "";
  const toSeg = seg(to);
  if (toSeg === "integrations") return path.startsWith("/integrations");
  if (toSeg === "overview" || toSeg === "getting-started")
    return ["overview", "getting-started"].includes(seg(path));
  if (toSeg === "configuration" || toSeg === "rule-language" || toSeg === "cli")
    return ["configuration", "rule-language", "cli"].includes(seg(path));
  if (toSeg === "troubleshooting") return seg(path) === "troubleshooting";
  return false;
}

export default function Header() {
  const { pathname } = useLocation();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [repoStats, setRepoStats] = useState<RepoStats | null>(() =>
    readCachedStats(),
  );
  const kbdHint = useMemo(() => (isMacLike() ? "⌘K" : "Ctrl K"), []);

  useEffect(() => setDrawerOpen(false), [pathname]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setSearchOpen(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (repoStats) return;
    let cancelled = false;
    fetch("https://api.github.com/repos/Typiqally/lintropy", {
      headers: { Accept: "application/vnd.github+json" },
    })
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => {
        if (cancelled || !data) return;
        const stats: RepoStats = {
          stars: data.stargazers_count ?? 0,
          forks: data.forks_count ?? 0,
        };
        setRepoStats(stats);
        writeCachedStats(stats);
      })
      .catch(() => {
        // network/rate-limit failure: silently skip counts
      });
    return () => {
      cancelled = true;
    };
  }, [repoStats]);

  return (
    <>
      <header className="sticky top-0 z-40 border-b border-[color:var(--color-border)] backdrop-blur-xl backdrop-saturate-150 bg-[rgba(8,8,8,0.55)]">
        <div className="mx-auto flex h-14 max-w-6xl items-center gap-3 px-4 sm:px-6">
          <Link
            to="/"
            className="flex items-center gap-2 text-[15px] font-bold tracking-tight text-[color:var(--color-fg)] transition hover:text-[color:var(--color-accent)]"
          >
            Lintropy
            <span
              className="rounded-full border border-[color:var(--color-border)] bg-white/[0.02] px-1.5 py-0.5 text-[10px] font-medium tracking-normal text-[color:var(--color-fg-subtle)]"
              aria-label={`version ${__LINTROPY_VERSION__}`}
            >
              v{__LINTROPY_VERSION__}
            </span>
          </Link>

          <nav className="hidden items-center gap-1 border-l border-[color:var(--color-border)] pl-3 md:flex">
            {primaryNav.map((item) => {
              const active = isActiveSection(item.to, pathname);
              return (
                <Link
                  key={item.to}
                  to={item.to}
                  className={`rounded-full px-3 py-1.5 text-[13px] font-medium transition ${
                    active
                      ? "bg-[rgba(141,225,180,0.08)] text-[color:var(--color-accent)]"
                      : "text-[color:var(--color-fg-muted)] hover:bg-white/5 hover:text-[color:var(--color-fg)]"
                  }`}
                >
                  {item.title}
                </Link>
              );
            })}
          </nav>

          <div className="flex-1" />

          <button
            type="button"
            onClick={() => setSearchOpen(true)}
            className="hidden h-8 w-56 items-center gap-2 rounded-full border border-[color:var(--color-border)] bg-white/[0.02] px-3 text-xs text-[color:var(--color-fg-subtle)] transition hover:border-[color:var(--color-border-strong)] hover:text-[color:var(--color-fg)] sm:flex"
            aria-label="Search"
          >
            <Search size={14} />
            <span>Search</span>
            <kbd className="ml-auto rounded border border-[color:var(--color-border)] px-1.5 py-0.5 text-[10px] text-[color:var(--color-fg-faint)]">
              {kbdHint}
            </kbd>
          </button>

          <a
            href="https://github.com/Typiqally/lintropy"
            target="_blank"
            rel="noopener noreferrer"
            className="hidden h-8 items-center gap-2 rounded-full border border-[color:var(--color-border)] bg-white/[0.02] px-3 text-xs font-semibold text-[color:var(--color-fg-muted)] transition hover:border-[color:var(--color-border-strong)] hover:text-[color:var(--color-fg)] sm:inline-flex"
          >
            <GithubMark size={14} />
            <span>GitHub</span>
            {repoStats && (
              <>
                <span
                  className="flex items-center gap-1 border-l border-[color:var(--color-border)] pl-2 text-[color:var(--color-fg-subtle)]"
                  aria-label={`${repoStats.stars} stars`}
                >
                  <Star size={12} />
                  {formatCount(repoStats.stars)}
                </span>
                <span
                  className="flex items-center gap-1 text-[color:var(--color-fg-subtle)]"
                  aria-label={`${repoStats.forks} forks`}
                >
                  <GitFork size={12} />
                  {formatCount(repoStats.forks)}
                </span>
              </>
            )}
          </a>

          <button
            type="button"
            onClick={() => setDrawerOpen((v) => !v)}
            className="inline-flex h-9 w-9 items-center justify-center rounded-md text-[color:var(--color-fg-muted)] transition hover:bg-white/5 md:hidden"
            aria-label="Open menu"
          >
            {drawerOpen ? <X size={18} /> : <Menu size={18} />}
          </button>
        </div>

        {drawerOpen && (
          <div className="border-t border-[color:var(--color-border)] md:hidden">
            <div className="mx-auto max-w-6xl px-4 py-3 sm:px-6">
              <div className="flex flex-col gap-1">
                {primaryNav.map((item) => {
                  const active = isActiveSection(item.to, pathname);
                  return (
                    <Link
                      key={item.to}
                      to={item.to}
                      className={`rounded-md px-3 py-2 text-sm transition ${
                        active
                          ? "bg-[rgba(141,225,180,0.08)] text-[color:var(--color-accent)]"
                          : "text-[color:var(--color-fg-muted)] hover:bg-white/5"
                      }`}
                    >
                      {item.title}
                    </Link>
                  );
                })}
              </div>
              <div className="mt-3 border-t border-[color:var(--color-border)] pt-3">
                {nav.map((group) => (
                  <div key={group.title} className="mb-3">
                    <div className="mb-1 text-[10px] font-bold uppercase tracking-[0.16em] text-[color:var(--color-fg-subtle)]">
                      {group.title}
                    </div>
                    {group.items.map((leaf) => (
                      <NavLink
                        key={leaf.slug}
                        to={`/${leaf.slug}`}
                        className={({ isActive }) =>
                          `block rounded-md px-2 py-1.5 text-sm transition ${
                            isActive
                              ? "text-[color:var(--color-accent)]"
                              : "text-[color:var(--color-fg-muted)] hover:text-[color:var(--color-fg)]"
                          }`
                        }
                      >
                        {leaf.title}
                      </NavLink>
                    ))}
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}
      </header>

      <SearchDialog open={searchOpen} onOpenChange={setSearchOpen} />
    </>
  );
}
