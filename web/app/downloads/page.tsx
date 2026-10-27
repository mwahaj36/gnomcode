import type { Metadata } from "next";
import Link from "next/link";
import { CopyCommand } from "../components/CopyCommand";

export const metadata: Metadata = {
  title: "Downloads — spawnt",
  description: "Download the verified Windows or Linux release of spawnt.",
  alternates: { canonical: "https://fleethud.vercel.app/downloads" },
};

const platforms = [
  {
    name: "Windows",
    availability: "AVAILABLE",
    detail: "Windows 10 / 11 · x64",
    body: "Install the tray app and local daemon. The standalone installer and zip contain the same v1.3.0 release.",
    actions: [
      { label: "Download installer", href: "/downloads/FleetHUD-Setup.exe", detail: ".exe" },
      { label: "Download archive", href: "/downloads/FleetHUD-v1.3.0-windows-x64.zip", detail: ".zip" },
    ],
    command: "irm https://fleethud.vercel.app/win | iex",
    note: "Updates preserve the account and recorded sessions.",
  },
  {
    name: "Linux",
    availability: "AVAILABLE",
    detail: "x86_64 / ARM64 · glibc 2.35+",
    body: "Install the CLI and local web daemon from the verified v1.3.0 tarball. WSL2 is supported.",
    actions: [
      { label: "Install script", href: "/sh", detail: "SHA-256 verified" },
      { label: "Release checksums", href: "/downloads/SHA256SUMS-v1.3.0.txt", detail: "SHA256SUMS" },
    ],
    command: "curl -fsSL https://fleethud.vercel.app/sh | bash",
    note: "Updates preserve local user data and account pairing.",
  },
  {
    name: "macOS",
    availability: "NOT PUBLISHED",
    detail: "Apple Silicon / Intel",
    body: "A verified macOS release is not available yet. We will publish an installer after native builds and signing are ready.",
    actions: [],
    command: null,
    note: "No download path is enabled for macOS today.",
  },
];

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

      <section className="sp-platform-list" aria-label="Platform downloads">
        {platforms.map((platform) => (
          <article className="sp-platform" key={platform.name}>
            <header>
              <h2>{platform.name}</h2>
              <span className={platform.actions.length ? "sp-platform-state" : "sp-platform-state sp-platform-pending"}>
                {platform.availability}
              </span>
            </header>
            <p className="sp-platform-detail">{platform.detail}</p>
            <p className="sp-platform-body">{platform.body}</p>
            {platform.command && (
              <div className="sp-install-command">
                <span>QUICK INSTALL</span>
                <CopyCommand command={platform.command} />
              </div>
            )}
            {platform.actions.length > 0 ? (
              <div className="sp-download-actions">
                {platform.actions.map((action, index) => (
                  <a className={index === 0 ? "sp-download-action sp-download-primary" : "sp-download-action"} href={action.href} key={action.href}>
                    <span>{action.label}</span><small>{action.detail}</small>
                  </a>
                ))}
              </div>
            ) : (
              <p className="sp-download-unavailable">There is no verified installer to download yet.</p>
            )}
            <p className="sp-platform-note">{platform.note}</p>
          </article>
        ))}
      </section>

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
