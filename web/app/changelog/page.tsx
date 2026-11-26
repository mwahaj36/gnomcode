import { readFileSync } from "node:fs";
import path from "node:path";
import Link from "next/link";
import branding from "../../branding.json";
import { pageMetadata } from "../../lib/seo";

export const metadata = pageMetadata(
  "/changelog",
  "Gnomcode Changelog: What's New in Each Release",
  "Release notes for Gnomcode, the free local runtime for Claude Code, Codex and Antigravity: new features, fixes and platform changes in every version.",
);

type Release = { version: string; date: string; groups: Array<{ title: string; items: string[] }> };

// Read at build time from the repository CHANGELOG so the page never drifts from release notes.
function parseChangelog(source: string): Release[] {
  const releases: Release[] = [];
  for (const line of source.split(/\r?\n/)) {
    const release = line.match(/^## (\d+\.\d+\.\d+)(?:\s+-\s+(\S+))?/);
    if (release) { releases.push({ version: release[1], date: release[2] ?? "", groups: [{ title: "", items: [] }] }); continue; }
    const current = releases[releases.length - 1];
    if (!current) continue;
    const group = line.match(/^###\s+(.+)/);
    if (group) { current.groups.push({ title: group[1].trim(), items: [] }); continue; }
    const item = line.match(/^\s*[-*]\s+(.+)/);
    if (item) current.groups[current.groups.length - 1].items.push(item[1].replace(/`([^`]+)`/g, "$1").replace(/\*\*([^*]+)\*\*/g, "$1").replace(/\[([^\]]+)\]\([^)]+\)/g, "$1"));
  }
  return releases.map(r => ({ ...r, groups: r.groups.filter(g => g.items.length) })).filter(r => r.groups.length);
}

function loadReleases(): Release[] {
  try { return parseChangelog(readFileSync(path.join(process.cwd(), "..", "CHANGELOG.md"), "utf8")).slice(0, 12); }
  catch { return []; }
}

export default function ChangelogPage() {
  const releases = loadReleases();
  const schema = {
    "@context": "https://schema.org",
    "@type": "ItemList",
    "name": "Gnomcode release notes",
    "itemListElement": releases.map((r, index) => ({
      "@type": "ListItem",
      "position": index + 1,
      "name": `Gnomcode ${r.version}`,
      "url": `${branding.baseUrl}/changelog#v${r.version}`,
    })),
  };
  return (
    <div className="sp-console-shell sp-docs-page sp-guide-page mx-auto px-6 font-sans">
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(schema).replace(/</g, "\\u003c") }} />
      <article className="sp-docs-article">
        <header className="sp-docs-header">
          <h1>Changelog</h1>
          <p>What changed in each Gnomcode release. Downloads and checksums for the current version are on the <Link href="/downloads">downloads page</Link>; every build is also on <a href="https://github.com/mwahaj36/gnomcode/releases">GitHub Releases</a>.</p>
        </header>
        {releases.length === 0 && <p>Release notes are on <a href="https://github.com/mwahaj36/gnomcode/releases">GitHub Releases</a>.</p>}
        {releases.map(r => (
          <section key={r.version} id={`v${r.version}`}>
            <h2>{r.version}{r.date && <> <time dateTime={r.date}>{r.date}</time></>}</h2>
            {r.groups.map((g, i) => (
              <div key={i}>
                {g.title && <h3>{g.title}</h3>}
                <ul>{g.items.map((item, j) => <li key={j}>{item}</li>)}</ul>
              </div>
            ))}
          </section>
        ))}
      </article>
    </div>
  );
}
