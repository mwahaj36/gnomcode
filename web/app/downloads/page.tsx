import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Download FleetHUD — Native Installers & Binaries",
  description:
    "Download FleetHUD for Windows 10/11, macOS (Apple Silicon & Intel), Linux, and npm. Features silent system tray mode, zero black console windows, and autostart.",
  alternates: {
    canonical: "https://fleethud.vercel.app/downloads",
  },
};

export default function DownloadsPage() {
  return (
    <div className="max-w-7xl mx-auto px-6 py-14 font-sans">
      <div className="flex justify-between items-center pb-6 border-b border-border mb-10 flex-wrap gap-4">
        <div>
          <div className="inline-flex items-center gap-2 px-3 py-0.5 rounded-full bg-green/15 border border-green/30 text-green font-mono text-xs mb-2">
            <span>Production Release &bull; v1.1.1</span>
          </div>
          <h1 className="text-3xl font-extrabold tracking-tight text-foreground">
            Download FleetHUD
          </h1>
          <p className="text-sm text-muted mt-1">
            Native installers, background services, and CLI packages for Windows, macOS, and Linux
          </p>
        </div>
        <Link
          href="/console"
          className="px-4 py-2 border border-border rounded text-xs font-mono text-foreground hover:border-accent transition-colors"
        >
          Launch Console &rarr;
        </Link>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mb-14 font-mono">
        {/* Windows Card */}
        <div className="bg-card border-2 border-accent/40 rounded-xl p-6 flex flex-col justify-between shadow-lg shadow-accent/5">
          <div>
            <div className="text-xs text-accent uppercase tracking-wider mb-2 font-semibold">
              [WINDOWS 10 / 11]
            </div>
            <h2 className="text-xl font-bold text-foreground mb-2">FleetHUD for Windows</h2>
            <p className="text-xs text-muted leading-relaxed mb-6">
              Standalone native <code className="text-foreground">Installer.exe</code> with System Tray daemon, autostart registry configuration, and zero black command prompt windows.
            </p>
          </div>
          <div>
            <a
              href="https://github.com/mwahaj36/fleethud/releases/latest/download/FleetHUD-v1.1.1-windows-x64.zip"
              className="w-full block text-center py-3 px-4 bg-accent text-[#0d1117] font-bold rounded-lg text-xs hover:opacity-90 transition-opacity shadow-sm shadow-accent/20"
            >
              Download Windows Zip (.exe) &darr;
            </a>
            <div className="text-[11px] text-muted text-center mt-2.5">
              PowerShell 1-Liner: <code className="text-accent select-all">irm fleethud.vercel.app/win | iex</code>
            </div>
          </div>
        </div>

        {/* macOS Card */}
        <div className="bg-card border border-border rounded-xl p-6 flex flex-col justify-between">
          <div>
            <div className="text-xs text-muted uppercase tracking-wider mb-2 font-semibold">
              [MACOS 12+]
            </div>
            <h2 className="text-xl font-bold text-foreground mb-2">FleetHUD for Mac</h2>
            <p className="text-xs text-muted leading-relaxed mb-6">
              Universal binary (Apple Silicon M1/M2/M3/M4 & Intel) with Menubar status tray, local SQLite flight recorder, and launchd daemon.
            </p>
          </div>
          <div>
            <div className="bg-bg border border-border rounded-lg p-3 text-xs text-muted select-all mb-3 overflow-x-auto">
              curl -fsSL https://raw.githubusercontent.com/mwahaj36/fleethud/main/scripts/install.sh | bash
            </div>
            <div className="text-[11px] text-muted text-center">Homebrew formula & tarball available</div>
          </div>
        </div>

        {/* Linux Card */}
        <div className="bg-card border border-border rounded-xl p-6 flex flex-col justify-between">
          <div>
            <div className="text-xs text-muted uppercase tracking-wider mb-2 font-semibold">
              [LINUX & WSL2]
            </div>
            <h2 className="text-xl font-bold text-foreground mb-2">FleetHUD for Linux</h2>
            <p className="text-xs text-muted leading-relaxed mb-6">
              POSIX compatible binary for Ubuntu, Debian, Fedora, Arch, and WSL2. Includes systemd user daemon unit.
            </p>
          </div>
          <div>
            <div className="bg-bg border border-border rounded-lg p-3 text-xs text-muted select-all mb-3 overflow-x-auto">
              curl -fsSL https://raw.githubusercontent.com/mwahaj36/fleethud/main/scripts/install.sh | bash
            </div>
            <div className="text-[11px] text-muted text-center">Debian .deb, RPM & AUR packages</div>
          </div>
        </div>
      </div>

      {/* CLI Quickstart */}
      <div className="bg-card border border-border rounded-xl p-6 font-mono">
        <div className="flex justify-between items-center mb-4 flex-wrap gap-2">
          <h3 className="text-sm font-semibold uppercase tracking-wider text-foreground">
            Install Globally via npm
          </h3>
          <span className="text-xs text-green font-semibold">[CROSS-PLATFORM]</span>
        </div>
        <p className="text-xs text-muted mb-4">
          For environments with Node.js installed, download and manage FleetHUD via npm:
        </p>
        <div className="bg-bg border border-border rounded-lg p-3.5 text-xs text-accent select-all">
          npm install -g fleethud
        </div>
      </div>
    </div>
  );
}
