"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { supabase } from "../lib/supabase";
import ThemeControl from "./ThemeControl";

export default function AppNavbar() {
  const pathname = usePathname();
  const [user, setUser] = useState<any>(null);

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setUser(session?.user ?? null);
    });

    const { data: { subscription } } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
    });

    return () => subscription.unsubscribe();
  }, []);

  const handleSignOut = async () => {
    await supabase.auth.signOut();
    setUser(null);
  };

  const onConsole = pathname.startsWith("/console");
  const onSettings = pathname.startsWith("/settings");

  return (
    <header className="sp-app-header">
      <div className="sp-app-header-inner">
        <div className="sp-app-brand-nav">
          <Link href="/console" className="sp-app-brand" aria-label="spawnt console">
            <span className="sp-brand-mark" aria-hidden="true" />
            <span>spawnt</span>
          </Link>
          <nav className="sp-app-nav" aria-label="App navigation">
            <Link
              href="/console"
              aria-current={onConsole ? "page" : undefined}
              className={onConsole ? "selected" : ""}
            >
              Console
            </Link>
            <Link
              href="/settings"
              aria-current={onSettings ? "page" : undefined}
              className={onSettings ? "selected" : ""}
            >
              Settings
            </Link>
            <Link href="/docs">Docs</Link>
          </nav>
        </div>

        <div className="sp-app-actions">
          <div className={`sp-app-daemon ${daemonLive ? "is-live" : ""}`} aria-live="polite">
            <span aria-hidden="true" />
            <span>{daemonLive ? "DAEMON / LIVE" : "DAEMON / STANDBY"}</span>
          </div>
          <ThemeControl className="sp-app-theme" />

          {user ? (
            <div className="sp-app-account">
              <Link href="/settings" title={user.email || "Account"}>
                {user.email?.split("@")[0] || "Account"}
              </Link>
              <button type="button" onClick={handleSignOut}>Sign out</button>
            </div>
          ) : null}
        </div>
      </div>
    </header>
  );
}
