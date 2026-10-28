import branding from "../../branding.json";
import type { Metadata } from "next";
import Link from "next/link";
import packageInfo from "../../package.json";
import { PlatformDownloads } from "./PlatformDownloads";

export const metadata: Metadata = {
  title: "Downloads — spawnt",
  description: "Spawnt downloads are being prepared.",
  alternates: { canonical: `${branding.baseUrl}/downloads` },
};

export default function DownloadsPage() {
  return (
    <div className="sp-console-shell sp-downloads-page mx-auto max-w-7xl px-6 py-16 font-sans">
      <header className="sp-downloads-header">
        <div>
          <h1>Download spawnt</h1>
          <p>Spawnt desktop and CLI builds are being verified. Your agent logs stay on your machine.</p>
        </div>
        <span className="sp-download-release">NEXT RELEASE · v{packageInfo.version}</span>
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
