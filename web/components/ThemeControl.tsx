"use client";

import { useEffect, useState } from "react";
import { applyThemeMode, nextThemeMode, readThemeMode, type ThemeMode } from "../lib/theme";

export default function ThemeControl({ className = "" }: { className?: string }) {
  const [theme, setTheme] = useState<ThemeMode>("system");

  useEffect(() => setTheme(readThemeMode()), []);

  function cycleTheme() {
    const next = nextThemeMode(theme);
    applyThemeMode(next);
    setTheme(next);
  }

  return <button className={`sp-theme-control ${className}`} type="button" onClick={cycleTheme} aria-label={`Theme: ${theme}. Switch mode`}>
    Theme / {theme}
  </button>;
}
