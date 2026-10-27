"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { supabase } from "../lib/supabase";
import ThemeControl from "./ThemeControl";

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
    <header className="sp-app-header border-b border-border sticky top-0 z-50 font-mono text-xs">
      <div className="max-w-7xl mx-auto px-6 h-13 flex items-center justify-between">
        {/* Left: Cockpit Brand & Tabs */}
        <div className="flex items-center gap-6">
          <Link href="/console" className="flex items-center gap-2.5">
            <span className="font-bold text-sm tracking-tight text-foreground font-sans">
              spawnt
            </span>
          </Link>

          <div className="h-4 w-px bg-border"></div>

          <nav className="flex items-center gap-1">
            <Link
              href="/console"
              className={`px-3 py-1 rounded transition-colors ${
                pathname === "/console"
                  ? "bg-card text-foreground border border-border font-semibold"
                  : "text-muted hover:text-foreground"
              }`}
            >
              Console
            </Link>
            <Link
              href="/settings"
              className={`px-3 py-1 rounded transition-colors ${
                pathname === "/settings"
                  ? "bg-card text-foreground border border-border font-semibold"
                  : "text-muted hover:text-foreground"
              }`}
            >
              Settings
            </Link>
            <Link
              href="/docs"
              className="px-3 py-1 rounded text-muted hover:text-foreground transition-colors"
            >
              Docs &rarr;
            </Link>
          </nav>
        </div>

        {/* Right: Live Sync Status Beacon & Account */}
        <div className="flex items-center gap-4">
          <div className="hidden sm:flex items-center gap-2 text-[11px]">
            <span
              className={`w-2 h-2 ${
                daemonLive ? "bg-green animate-pulse" : "bg-yellow"
              }`}
            ></span>
            <span className="text-muted">
              {daemonLive ? "127.0.0.1:41738 [LIVE]" : "Daemon Standby"}
            </span>
          </div>

          <ThemeControl className="sp-app-theme" />

          {user ? (
            <div className="flex items-center gap-3">
              <Link
                href="/settings"
                className="text-muted hover:text-foreground font-semibold text-[11px]"
              >
                {user.email?.split("@")[0] || "Account"}
              </Link>
              <button
                onClick={handleSignOut}
                className="text-red hover:underline text-[11px]"
              >
                Sign Out
              </button>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <Link
                href="/auth"
                className="px-2.5 py-1 rounded bg-accent/20 text-accent border border-accent/40 hover:bg-accent/30 font-semibold text-[11px]"
              >
                [Connect Cloud Sync]
              </Link>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
