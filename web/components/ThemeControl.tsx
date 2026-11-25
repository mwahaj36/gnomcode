"use client";

import { type KeyboardEvent, useEffect, useState } from "react";
import { applyThemeMode, readThemeMode, type ThemeMode } from "../lib/theme";

const MODES: { mode: ThemeMode; label: string }[] = [
  { mode: "system", label: "System" },
  { mode: "light", label: "Light" },
  { mode: "dark", label: "Dark" },
];

/** Theme as a three-way segmented control: every option is visible, the current one is checked. */
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

  function choose(mode: ThemeMode) {
    applyThemeMode(mode);
    setTheme(mode);
  }

  // Radio group keyboard model: arrows move and select, focus follows the checked option.
  function onKey(event: KeyboardEvent<HTMLDivElement>) {
    const step = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    const index = (MODES.findIndex((item) => item.mode === theme) + step + MODES.length) % MODES.length;
    choose(MODES[index].mode);
    event.currentTarget.querySelectorAll<HTMLButtonElement>("button")[index]?.focus();
  }

  return (
    <div className={`sp-theme-control ${className}`} role="radiogroup" aria-label="Theme" onKeyDown={onKey}>
      {MODES.map(({ mode, label }) => (
        <button key={mode} type="button" role="radio" aria-checked={theme === mode} tabIndex={theme === mode ? 0 : -1} onClick={() => choose(mode)}>
          {label}
        </button>
      ))}
    </div>
  );
}
