import type { Metadata } from "next";
import Link from "next/link";
import { PlatformDownloads } from "./PlatformDownloads";

export const metadata: Metadata = {
  title: "Downloads — spawnt",
  description: "Download the verified Windows or Linux release of spawnt.",
  alternates: { canonical: "https://fleethud.vercel.app/downloads" },
};

export default function DownloadsPage() {
  return (
    <div className="sp-console-shell sp-downloads-page mx-auto max-w-7xl px-6 py-16 font-sans">
      <header className="sp-downloads-header">
        <div>
          <h1>Download spawnt</h1>
          <p>Verified local releases for Windows and Linux. Your agent logs stay on your machine.</p>
        </div>
        <span className="sp-download-release">CURRENT RELEASE · v1.3.0</span>
      </header>

      <PlatformDownloads />

      <section className="sp-download-help" aria-label="Release verification">
        <div>
          <h2>Verify the release</h2>
          <p>Release archives include a SHA-256 manifest. The Windows updater and Linux install script verify downloads before replacing an existing install.</p>
        </div>
        <Link href="/docs#quickstart">Read installation guide <span aria-hidden="true">→</span></Link>
      </section>
    </div>
  );
}
