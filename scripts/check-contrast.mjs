// Verifies WCAG contrast for spawnt design tokens. Usage: node scripts/check-contrast.mjs
// Exits non-zero if any required pair fails. Keep values in sync with web/app/tokens.css.
const lum = (hex) => {
  const c = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
};
const ratio = (a, b) => {
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
};

const themes = {
  light: { bg: "#f6f7f8", surface: "#ffffff", surface2: "#edf0f3", text: "#0c1119", text2: "#444f5f", text3: "#5f6a7a", primaryText: "#d4102e", link: "#252e99", warn: "#8a5a0b" },
  dark: { bg: "#0c1119", surface: "#121926", surface2: "#19212f", text: "#f6f7f8", text2: "#a9b4c2", text3: "#8591a3", primaryText: "#ff4d66", link: "#becbd8", warn: "#e0a84a" },
};
const PRIMARY = "#ff2544", TERTIARY = "#252e99", DARK = "#0c1119";

const checks = []; // [label, fg, bg, min]
for (const [name, t] of Object.entries(themes)) {
  for (const bgKey of ["bg", "surface", "surface2"]) {
    for (const fg of ["text", "text2", "text3", "primaryText", "link", "warn"]) {
      checks.push([`${name}: ${fg} on ${bgKey}`, t[fg], t[bgKey], 4.5]);
    }
  }
}
checks.push(["button: on-primary (dark) on primary", DARK, PRIMARY, 4.5]);
checks.push(["button: white on tertiary", "#ffffff", TERTIARY, 4.5]);
checks.push(["dark: primary graphic on bg", PRIMARY, themes.dark.bg, 3]);
checks.push(["light: primary graphic on bg", PRIMARY, themes.light.bg, 3]);
checks.push(["info: primary text on light (large only, expected < 4.5)", PRIMARY, themes.light.bg, 3]);

let fail = 0;
for (const [label, fg, bg, min] of checks) {
  const r = ratio(fg, bg);
  const ok = r >= min;
  if (!ok) fail++;
  console.log(`${ok ? "PASS" : "FAIL"}  ${r.toFixed(2).padStart(5)}  (min ${min})  ${label}`);
}
console.log(`tertiary on dark bg (never use as text): ${ratio(TERTIARY, themes.dark.bg).toFixed(2)}`);
process.exit(fail ? 1 : 0);
