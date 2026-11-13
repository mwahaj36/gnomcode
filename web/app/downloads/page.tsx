import { pageMetadata } from "../../lib/seo";
import Link from "next/link";
import packageInfo from "../../package.json";
import { PlatformDownloads } from "./PlatformDownloads";

export const metadata = pageMetadata("/downloads", `Download Gnomcode v${packageInfo.version} for Windows`, "Windows x64 release candidate for Claude Code, Codex and Antigravity. Check release notes and SHA-256 checksums before installing. macOS and Linux downloads are not available yet.");

export default function DownloadsPage() {
  return (
    <div className="sp-console-shell sp-downloads-page mx-auto max-w-7xl px-6 py-16 font-sans">
      <header className="sp-downloads-header">
        <div>
          <h1>Download gnomcode</h1>
          <p>Launch and switch supported AI coding agents in one local workspace. This release candidate targets Windows x64; macOS and Linux downloads are not yet available.</p>
        </div>
        <span className="sp-download-release">GNOMCODE · v{packageInfo.version}</span>
      </header>

      <PlatformDownloads />
      <p className="sp-download-terms">By downloading, you agree to the <Link href="/terms">Terms of Service</Link>. Read our <Link href="/privacy">Privacy notice</Link>.</p>

      <section className="sp-download-help" aria-label="Release verification">
        <div>
          <h2>Verify the release</h2>
          <p>Check the versioned SHA-256 manifest and release notes before installing. Windows installers are unsigned; update signatures are a separate authenticity check.</p>
        </div>
        <Link href="/docs/getting-started">Read installation guide <span aria-hidden="true">→</span></Link>
      </section>
    </div>
  );
}
