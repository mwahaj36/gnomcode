export type ThemeMode = "system" | "light" | "dark";

export function readThemeMode(): ThemeMode {
  try {
    const stored = localStorage.getItem("spawnt-theme");
    return stored === "light" || stored === "dark" ? stored : "system";
  } catch {
    return "system";
  }
}

export function applyThemeMode(mode: ThemeMode) {
  if (mode === "system") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = mode;

  try {
    if (mode === "system") localStorage.removeItem("spawnt-theme");
    else localStorage.setItem("spawnt-theme", mode);
  } catch {
    // Keep the current page theme even if browser storage is unavailable.
  }
}

export function nextThemeMode(mode: ThemeMode): ThemeMode {
  return mode === "system" ? "light" : mode === "light" ? "dark" : "system";
}
