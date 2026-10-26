"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { supabase } from "../lib/supabase";

export default function MarketingNavbar() {
  const pathname = usePathname();
  const [user, setUser] = useState<any>(null);
  const [isScrolled, setIsScrolled] = useState(false);
  const [theme, setTheme] = useState("dark");

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setUser(session?.user ?? null);
    });

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setUser(session?.user ?? null);
    });

    // Theme detection
    const savedTheme = localStorage.getItem("spawnt_theme") || "dark";
    setTheme(savedTheme);
    document.documentElement.setAttribute("data-theme", savedTheme);

    // Daytona Hysteresis on-scroll floating island handler
    let ticking = false;
    const handleScroll = () => {
      if (!ticking) {
        window.requestAnimationFrame(() => {
          const scrollY = window.scrollY;
          if (scrollY > 45 && !isScrolled) {
            setIsScrolled(true);
          } else if (scrollY < 15 && isScrolled) {
            setIsScrolled(false);
          }
          ticking = false;
        });
        ticking = true;
      }
    };

    window.addEventListener("scroll", handleScroll, { passive: true });
    handleScroll();

    return () => {
      subscription.unsubscribe();
      window.removeEventListener("scroll", handleScroll);
    };
  }, [isScrolled]);

  const toggleTheme = () => {
    const nextTheme = theme === "dark" ? "light" : "dark";
    setTheme(nextTheme);
    document.documentElement.setAttribute("data-theme", nextTheme);
    localStorage.setItem("spawnt_theme", nextTheme);
  };

  const navLinks = [
    { name: "PRODUCT", href: "/" },
    { name: "PRICING", href: "/pricing" },
    { name: "DOCS", href: "/docs" },
    { name: "DOWNLOADS", href: "/downloads" },
  ];

  return (
    <div
      className={`fixed top-0 left-0 right-0 z-50 flex justify-center pointer-events-none transition-[padding] duration-[320ms] ease-[cubic-bezier(0.16,1,0.3,1)] ${
        isScrolled ? "px-4 pt-3 pb-0" : "p-0"
      }`}
    >
      <header
        className={`pointer-events-auto h-[60px] w-full flex items-center justify-between px-6 transition-all duration-[320ms] ease-[cubic-bezier(0.16,1,0.3,1)] ${
          isScrolled
            ? "max-w-[1240px] rounded-md border border-line-strong bg-surface/90 backdrop-blur-xl shadow-2xl shadow-black/60 shadow-[0_16px_36px_-10px_rgba(0,0,0,0.85),0_0_25px_rgba(255,37,68,0.06)]"
            : "max-w-full rounded-none border-b border-line bg-surface/80 backdrop-blur-md"
        }`}
      >
        {/* Brand Lockup */}
        <div className="flex items-center gap-6">
          <Link href="/" className="flex items-center gap-2.5 group">
            <svg
              className="w-5 h-5 text-ink transition-transform group-hover:scale-105"
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
            <span className="font-bold text-base tracking-tight text-ink lowercase">
              spawnt
            </span>
          </Link>

          <span className="text-ink-3 font-mono text-sm">/</span>

          {/* Navigation Links */}
          <nav className="hidden md:flex items-center gap-6 font-mono text-[11px] font-semibold tracking-wider">
            {navLinks.map((link) => {
              const isActive = pathname === link.href;
              return (
                <Link
                  key={link.href}
                  href={link.href}
                  className={`transition-colors py-1 ${
                    isActive
                      ? "text-primary font-bold border-b-2 border-primary"
                      : "text-ink-2 hover:text-primary"
                  }`}
                >
                  {link.name}
                </Link>
              );
            })}
          </nav>
        </div>

        {/* Right CTA Actions */}
        <div className="flex items-center gap-3 font-mono text-xs">
          <button
            onClick={toggleTheme}
            className="w-8 h-8 rounded-sm border border-line bg-surface-2 text-ink-2 hover:text-ink hover:border-line-strong flex items-center justify-center transition-colors"
            title="Toggle theme"
          >
            {theme === "dark" ? (
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              >
                <circle cx="12" cy="12" r="5" />
                <line x1="12" y1="1" x2="12" y2="3" />
                <line x1="12" y1="21" x2="12" y2="23" />
                <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
                <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
                <line x1="1" y1="12" x2="3" y2="12" />
                <line x1="21" y1="12" x2="23" y2="12" />
                <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
                <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
              </svg>
            ) : (
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              >
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
              </svg>
            )}
          </button>

          {user ? (
            <Link
              href="/console"
              className="px-3 py-1.5 rounded-sm border border-line bg-surface-2 text-ink hover:border-primary transition-colors text-[11px] font-semibold"
            >
              CONSOLE &rarr;
            </Link>
          ) : (
            <Link
              href="/auth"
              className="hidden sm:inline-flex px-3 py-1.5 rounded-sm border border-line bg-surface-2 text-ink-2 hover:text-ink hover:border-line-strong transition-colors text-[11px] font-semibold"
            >
              SIGN IN
            </Link>
          )}

          <Link
            href="/downloads"
            className="px-3.5 py-1.5 rounded-sm bg-primary text-dark font-semibold hover:opacity-90 transition-opacity text-[11px] tracking-wide"
          >
            DOWNLOAD
          </Link>
        </div>
      </header>
    </div>
  );
}
