import Link from "next/link";

export default function DownloadsPage() {
  return (
    <div className="max-w-4xl mx-auto py-8">
      <div className="flex justify-between items-center pb-6 border-b border-border mb-8">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">FleetHUD Downloads</h1>
          <p className="text-sm text-muted mt-1">Native installers and binaries for Windows, macOS, and Linux</p>
        </div>
        <Link href="/" className="px-4 py-2 border border-border rounded text-sm font-mono hover:bg-card">
          &larr; Back to Mission Control
        </Link>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
        {/* Windows Card */}
        <div className="bg-card border border-border rounded-lg p-6 flex flex-col justify-between">
          <div>
            <div className="text-xs font-mono text-accent uppercase tracking-wider mb-2">[WINDOWS 10/11]</div>
            <h2 className="text-lg font-semibold mb-2">FleetHUD for Windows</h2>
            <p className="text-xs text-muted leading-relaxed mb-6">
              Full desktop installer with System Tray icon, Desktop shortcut, and background autostart.
            </p>
          </div>
          <div>
            <a
              href="https://github.com/mwahaj36/fleethud/releases/latest/download/FleetHUD-v1.1.0-windows-x64.zip"
              className="w-full block text-center py-2.5 px-4 bg-green text-bg font-semibold font-mono rounded text-sm hover:opacity-90 transition-opacity"
            >
              Download Windows (.exe / .zip)
            </a>
            <div className="text-[11px] font-mono text-muted text-center mt-2">64-bit | Standalone + Installer</div>
          </div>
        </div>

        {/* macOS Card */}
        <div className="bg-card border border-border rounded-lg p-6 flex flex-col justify-between">
          <div>
            <div className="text-xs font-mono text-accent uppercase tracking-wider mb-2">[MACOS 12+]</div>
            <h2 className="text-lg font-semibold mb-2">FleetHUD for Mac</h2>
            <p className="text-xs text-muted leading-relaxed mb-6">
              Universal binary (Apple Silicon M1/M2/M3 & Intel) with menubar HUD and background service.
            </p>
          </div>
          <div>
            <div className="bg-bg border border-border rounded p-2 text-xs font-mono text-muted select-all mb-2">
              curl -fsSL https://raw.githubusercontent.com/mwahaj36/fleethud/main/scripts/install.sh | sh
            </div>
            <div className="text-[11px] font-mono text-muted text-center">Terminal one-line install</div>
          </div>
        </div>

        {/* Linux Card */}
        <div className="bg-card border border-border rounded-lg p-6 flex flex-col justify-between">
          <div>
            <div className="text-xs font-mono text-accent uppercase tracking-wider mb-2">[LINUX]</div>
            <h2 className="text-lg font-semibold mb-2">FleetHUD for Linux</h2>
            <p className="text-xs text-muted leading-relaxed mb-6">
              POSIX compatible installer for Ubuntu, Debian, Fedora, Arch, and WSL2.
            </p>
          </div>
          <div>
            <div className="bg-bg border border-border rounded p-2 text-xs font-mono text-muted select-all mb-2">
              curl -fsSL https://raw.githubusercontent.com/mwahaj36/fleethud/main/scripts/install.sh | sh
            </div>
            <div className="text-[11px] font-mono text-muted text-center">Includes systemd user daemon</div>
          </div>
        </div>
      </div>

      {/* CLI Quickstart */}
      <div className="bg-card border border-border rounded-lg p-6">
        <h3 className="text-sm font-semibold uppercase tracking-wider text-muted font-mono mb-3">Install via npm / npx</h3>
        <p className="text-xs text-muted mb-4">Run FleetHUD anywhere with Node.js installed:</p>
        <div className="bg-bg border border-border rounded p-3 text-xs font-mono text-accent select-all">
          npm install -g fleethud
        </div>
      </div>
    </div>
  );
}
