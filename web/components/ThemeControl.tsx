"use client";

import { useEffect, useState } from "react";
import { applyThemeMode, nextThemeMode, readThemeMode, type ThemeMode } from "../lib/theme";

export default function ThemeControl({ className = "" }: { className?: string }) {
  const [theme, setTheme] = useState<ThemeMode>("system");

  // Label, applied theme and stored preference stay one value: re-read and re-apply when another tab
  // changes the preference or this tab regains focus.
  useEffect(() => {
    const sync = () => {
      const mode = readThemeMode();
      applyThemeMode(mode);
      setTheme(mode);
    };
    sync();
    const onStorage = (event: StorageEvent) => {
      if (event.key === null || event.key === "gnomcode-theme") sync();
    };
    window.addEventListener("storage", onStorage);
    window.addEventListener("focus", sync);
    return () => {
      window.removeEventListener("storage", onStorage);
      window.removeEventListener("focus", sync);
    };
  }, []);

  function cycleTheme() {
    const next = nextThemeMode(theme);
    applyThemeMode(next);
    setTheme(next);
  }

  return <button className={`sp-theme-control ${className}`} type="button" onClick={cycleTheme} aria-label={`Theme: ${theme}. Switch mode`}>
    Theme / {theme}
  </button>;
}
