import type { Metadata } from "next";
import Link from "next/link";
import { BRAND } from "@/lib/brand";

export const metadata: Metadata = {
  title: "Download spawnt — Native Installers & Binaries",
  description:
    "Download spawnt for Windows 10/11, macOS, Linux, and npm. Zero-configuration local monitoring for AI coding agents with zero content storage.",
  alternates: {
    canonical: "https://spawnt.xyz/downloads",
  },
};

export default function DownloadsPage() {
  return (
    <div className="max-w-7xl mx-auto px-6 py-14 font-mono text-xs">
      <div className="flex justify-between items-center pb-6 border-b border-border mb-10 flex-wrap gap-4">
        <div>
          <div className="inline-flex items-center gap-2 px-3 py-1 rounded-[4px] bg-surface border border-border text-muted uppercase tracking-wider text-[11px] mb-2">
            <span className="w-1.5 h-1.5 rounded-[1px] bg-primary"></span>
            <span>PRODUCTION RELEASE // v{BRAND.version}</span>
          </div>
          <h1 className="text-3xl font-bold tracking-tight text-foreground font-mono">
            Download {BRAND.name}
          </h1>
          <p className="text-sm text-muted mt-1 font-sans">
            Native installers, background desktop tray services, and CLI packages for Windows, macOS, and Linux
          </p>
        </div>
        <Link
          href="/console"
          className="h-9 px-4 border border-border bg-surface-elevated rounded-[4px] text-xs text-foreground hover:border-primary transition-colors flex items-center gap-2"
        >
          Launch Console &rarr;
        </Link>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
        {/* Windows Card */}
        <div className="bg-surface border border-primary rounded-[6px] p-6 flex flex-col justify-between">
          <div>
            <div className="text-[11px] text-primary uppercase tracking-wider mb-2 font-bold">
              [WINDOWS 10 / 11]
            </div>
            <h2 className="text-lg font-bold text-foreground mb-2">{BRAND.name} for Windows</h2>
            <p className="text-xs text-muted font-sans leading-relaxed mb-6">
              Standalone native installer with silent background tray daemon, autostart configuration, and zero black command prompt windows.
            </p>
          </div>
          <div className="space-y-3">
            <a
              href="/downloads/FleetHUD-Setup.exe"
              download="spawnt-setup.exe"
              className="w-full block text-center py-2.5 px-4 bg-primary text-white font-bold rounded-[4px] text-xs hover:brightness-110 transition-all"
            >
              Download Windows (.exe) &darr;
            </a>
            <div className="flex justify-between items-center text-[11px] text-muted px-1">
              <span>Standalone Setup</span>
              <a href="/downloads/FleetHUD-Setup.exe" download="spawnt-setup.exe" className="text-primary hover:underline">
                Direct (.exe)
              </a>
            </div>
            <div className="text-[11px] text-muted text-center pt-2 border-t border-border">
              PowerShell: <code className="text-foreground select-all">irm https://spawnt.xyz/win | iex</code>
            </div>
          </div>
        </div>

        {/* macOS Card */}
        <div className="bg-surface border border-border rounded-[6px] p-6 flex flex-col justify-between">
          <div>
            <div className="text-[11px] text-muted uppercase tracking-wider mb-2 font-bold">
              [MACOS 12+]
            </div>
            <h2 className="text-lg font-bold text-foreground mb-2">{BRAND.name} for Mac</h2>
            <p className="text-xs text-muted font-sans leading-relaxed mb-6">
              Universal binary (Apple Silicon & Intel) with menubar status indicator, local SQLite database, and launchd background daemon.
            </p>
          </div>
          <div>
            <div className="bg-surface-elevated border border-border rounded-[4px] p-3 text-xs text-foreground select-all mb-3 overflow-x-auto">
              curl -fsSL https://spawnt.xyz/install.sh | sh
            </div>
            <div className="text-[11px] text-muted text-center">Homebrew formula & tarball available</div>
          </div>
        </div>

        {/* Linux Card */}
        <div className="bg-surface border border-border rounded-[6px] p-6 flex flex-col justify-between">
          <div>
            <div className="text-[11px] text-muted uppercase tracking-wider mb-2 font-bold">
              [LINUX & WSL2]
            </div>
            <h2 className="text-lg font-bold text-foreground mb-2">{BRAND.name} for Linux</h2>
            <p className="text-xs text-muted font-sans leading-relaxed mb-6">
              POSIX compatible binary for Ubuntu, Debian, Fedora, Arch, and WSL2. Includes systemd user service unit.
            </p>
          </div>
          <div>
            <div className="bg-surface-elevated border border-border rounded-[4px] p-3 text-xs text-foreground select-all mb-3 overflow-x-auto">
              curl -fsSL https://spawnt.xyz/install.sh | sh
            </div>
            <div className="text-[11px] text-muted text-center">Debian .deb, RPM & AUR packages</div>
          </div>
        </div>
      </div>

      {/* CLI Quickstart */}
      <div className="bg-surface border border-border rounded-[6px] p-6">
        <div className="flex justify-between items-center mb-3 flex-wrap gap-2">
          <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
            INSTALL GLOBALLY VIA NPM //
          </h3>
          <span className="text-[10px] text-primary font-bold">[CROSS-PLATFORM]</span>
        </div>
        <p className="text-xs text-muted font-sans mb-3">
          For environments with Node.js installed, run or install {BRAND.name} immediately:
        </p>
        <div className="bg-surface-elevated border border-border rounded-[4px] p-3 text-xs text-foreground select-all">
          npm install -g spawnt
        </div>
      </div>
    </div>
  );
}
