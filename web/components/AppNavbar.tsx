"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { supabase } from "../lib/supabase";

export default function AppNavbar() {
  const pathname = usePathname();
  const [user, setUser] = useState<any>(null);
  const [daemonLive, setDaemonLive] = useState<boolean | null>(null);

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setUser(session?.user ?? null);
    });

    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
    });

    const checkDaemon = async () => {
      try {
        const res = await fetch("http://127.0.0.1:41738/api/v1/live?page=1&limit=1", {
          signal: AbortSignal.timeout(1500),
        });
        setDaemonLive(res.ok);
      } catch {
        setDaemonLive(false);
      }
    };

    checkDaemon();
    const interval = setInterval(checkDaemon, 5000);

    return () => {
      subscription.unsubscribe();
      clearInterval(interval);
    };
  }, []);

  const handleSignOut = async () => {
    await supabase.auth.signOut();
    setUser(null);
  };

  return (
    <header className="border-b border-line bg-surface sticky top-0 z-50 font-mono text-xs">
      <div className="max-w-7xl mx-auto px-6 h-13 flex items-center justify-between">
        {/* Left: Brand & Tabs */}
        <div className="flex items-center gap-6">
          <Link href="/console" className="flex items-center gap-2.5">
            <svg
              className="w-4 h-4 text-ink"
              viewBox="0 0 2048 2048"
            >
              <polyline
                points="290.67 1756.53 290.67 291.47 1755.73 291.47"
                fill="currentColor"
              />
              <rect
                x="799.39"
                y="789.88"
                width="957.95"
                height="957.95"
                fill="currentColor"
              />
            </svg>
            <span className="font-bold text-sm tracking-tight text-ink lowercase">
              spawnt
            </span>
            <span className="text-[10px] px-1.5 py-0.5 rounded-sm bg-surface-2 text-ink-2 border border-line font-semibold">
              CONSOLE
            </span>
          </Link>

          <div className="h-4 w-px bg-line"></div>

          <nav className="flex items-center gap-1">
            <Link
              href="/console"
              className={`px-3 py-1 rounded-sm transition-colors ${
                pathname === "/console"
                  ? "bg-surface-2 text-ink border border-line font-semibold"
                  : "text-ink-2 hover:text-ink"
              }`}
            >
              Console
            </Link>
            <Link
              href="/settings"
              className={`px-3 py-1 rounded-sm transition-colors ${
                pathname === "/settings"
                  ? "bg-surface-2 text-ink border border-line font-semibold"
                  : "text-ink-2 hover:text-ink"
              }`}
            >
              Settings
            </Link>
            <Link
              href="/docs"
              className="px-3 py-1 rounded-sm text-ink-2 hover:text-ink transition-colors"
            >
              Docs &rarr;
            </Link>
          </nav>
        </div>

        {/* Right: Live Sync Status Beacon & Account */}
        <div className="flex items-center gap-4">
          <div className="hidden sm:flex items-center gap-2 px-2.5 py-1 rounded-sm bg-surface-2 border border-line text-[11px]">
            <span
              className={`w-2 h-2 rounded-full ${
                daemonLive ? "bg-primary animate-pulse" : "bg-ink-3"
              }`}
            ></span>
            <span className="text-ink-2">
              {daemonLive ? "127.0.0.1:41738 [LIVE]" : "Daemon Standby"}
            </span>
          </div>

          {user ? (
            <div className="flex items-center gap-3">
              <Link
                href="/settings"
                className="px-2.5 py-1 rounded-sm bg-surface-2 text-ink border border-line font-semibold text-[11px]"
              >
                {user.email?.split("@")[0] || "User"}
              </Link>
              <button
                onClick={handleSignOut}
                className="text-primary hover:underline text-[11px]"
              >
                Sign Out
              </button>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <Link
                href="/auth"
                className="px-2.5 py-1 rounded-sm bg-primary text-dark font-semibold text-[11px]"
              >
                Sign In
              </Link>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
