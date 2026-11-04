"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import ThemeControl from "./ThemeControl";

export default function AppNavbar() {
  const pathname = usePathname();
  const onRemote = pathname.startsWith("/remote");
  const onConsole = pathname.startsWith("/console");
  const onSettings = pathname.startsWith("/settings");

  return (
    <header className="sp-app-header">
      <div className="sp-app-header-inner">
        <div className="sp-app-brand-nav">
          <Link href="/app" className="sp-app-brand" aria-label="gnomcode workspace">
            <span className="sp-brand-mark" aria-hidden="true" />
            <span>gnomcode</span>
          </Link>
          <nav className="sp-app-nav" aria-label="App navigation">
            <Link
              href="/remote"
              aria-current={onRemote ? "page" : undefined}
              className={onRemote ? "selected" : ""}
            >
              Remote
            </Link>
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
          </nav>
        </div>
        <div className="sp-app-actions">
          <ThemeControl className="sp-app-theme" />
        </div>
      </div>
    </header>
  );
}
