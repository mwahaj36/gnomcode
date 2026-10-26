"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { supabase } from "../lib/supabase";

export default function MarketingNavbar() {
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

  const navLinks = [
    { name: "Overview", href: "/" },
    { name: "Documentation", href: "/docs" },
    { name: "Pricing", href: "/pricing" },
    { name: "Downloads", href: "/downloads" },
  ];

  return (
    <header className="border-b border-border bg-[#0d1117]/80 backdrop-blur-md sticky top-0 z-50">
      <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
        <div className="flex items-center gap-8">
          <Link href="/" className="flex items-center gap-3 group">
            <div className="w-8 h-8 rounded-lg bg-[#0d1117] p-1 flex items-center justify-center shadow-sm group-hover:scale-105 transition-transform">
              <img src="/fleethud.svg" alt="FleetHUD Logo" className="w-full h-full object-contain" />
            </div>
            <div className="flex flex-col">
              <span className="font-bold text-base tracking-tight text-foreground font-sans">
                FleetHUD
              </span>
              <span className="text-[10px] font-mono text-muted -mt-1">
                Mission Control
              </span>
            </div>
          </Link>

          <nav className="hidden md:flex items-center gap-1 font-mono text-xs">
            {navLinks.map((link) => {
              const isActive = pathname === link.href;
              return (
                <Link
                  key={link.href}
                  href={link.href}
                  className={`px-3.5 py-1.5 rounded-md transition-all ${
                    isActive
                      ? "bg-card text-foreground border border-border font-semibold shadow-sm"
                      : "text-muted hover:text-foreground hover:bg-card/50"
                  }`}
                >
                  {link.name}
                </Link>
              );
            })}
          </nav>
        </div>

        <div className="flex items-center gap-3 font-mono text-xs">
          <Link
            href="/downloads"
            className="hidden sm:inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-border text-muted hover:text-foreground hover:border-accent/50 transition-colors"
          >
            <span>Download</span>
            <span className="text-[10px] px-1.5 py-0.2 rounded bg-card text-accent border border-border">v1.1.1</span>
          </Link>

          {user ? (
            <div className="flex items-center gap-2">
              <Link
                href="/console"
                className="px-3.5 py-1.5 bg-accent text-[#0d1117] font-semibold rounded-md hover:opacity-90 transition-opacity flex items-center gap-1.5"
              >
                <span>Console</span>
                <span className="text-[10px]">&rarr;</span>
              </Link>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <Link
                href="/auth"
                className="px-3 py-1.5 text-muted hover:text-foreground transition-colors"
              >
                Sign In
              </Link>
              <Link
                href="/console"
                className="px-3.5 py-1.5 bg-accent text-[#0d1117] font-semibold rounded-md hover:opacity-90 transition-opacity shadow-sm shadow-accent/20 flex items-center gap-1.5"
              >
                <span>Launch Console</span>
                <span className="text-[10px]">&rarr;</span>
              </Link>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
