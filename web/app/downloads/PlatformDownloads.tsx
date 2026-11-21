"use client";

import { useEffect, useRef, useState } from "react";
import { CopyCommand } from "../components/CopyCommand";
import packageInfo from "../../package.json";
const { version } = packageInfo;

type DownloadAction = { label: string; href: string; detail: string };

const platforms = [
  {
    id: "windows", name: "Windows", availability: "Available", detail: "Windows · x64",
    body: "Launch Claude Code, Codex and Antigravity, continue conversations and keep a separate workspace terminal close at hand.",
    actions: [
      {label: "Download Windows installer", href: "/downloads/Gnomcode-Setup.exe", detail: "Windows x64"},
      {label: "Download ZIP", href: `/downloads/Gnomcode-v${version}-windows-x64.zip`, detail: "CLI and installer archive"},
      {label: "Download Windows ARM64 CLI", href: `/downloads/Gnomcode-v${version}-windows-arm64-cli.zip`, detail: "CLI only; no desktop installer"},
    ] as DownloadAction[], command: null,
    note: "Unsigned Windows installer. Review the release notes and SHA-256 checksums before installing.",
  },
  {
    id: "macos", name: "macOS", availability: "Available", detail: "macOS · Apple Silicon & Intel",
    body: "Bring your agent conversations, recorded usage and model-switch workflow into one local workspace.",
    actions: [
      {label: "Download desktop (Apple Silicon)", href: `/downloads/Gnomcode-darwin-arm64-${version}.zip`, detail: "macOS arm64 desktop app"},
      {label: "Download desktop (Intel)", href: `/downloads/Gnomcode-darwin-x64-${version}.zip`, detail: "macOS x64 desktop app"},
      {label: "Download macOS CLI (Apple Silicon)", href: `/downloads/gnomcode-v${version}-darwin-aarch64.tar.gz`, detail: "macOS arm64 archive"},
      {label: "Download macOS CLI (Intel)", href: `/downloads/gnomcode-v${version}-darwin-x86_64.tar.gz`, detail: "macOS x86_64 archive"},
    ] as DownloadAction[],
    command: "curl -fsSL https://gnomcode.vercel.app/downloads/install.sh | bash",
    note: "Unsigned, unnotarized macOS packages verified in CI. Review the release notes and SHA-256 checksums before installing.",
  },
  {
    id: "linux", name: "Linux", availability: "Available", detail: "Linux · x86_64 & arm64",
    body: "Use the standalone CLI and local console to orchestrate agents and inspect usage on your own machine.",
    actions: [
      {label: "Download desktop (.deb)", href: `/downloads/gnomcode-desktop_${version}_amd64.deb`, detail: "Debian / Ubuntu x64"},
      {label: "Download desktop ZIP", href: `/downloads/Gnomcode-linux-x64-${version}.zip`, detail: "Linux x64 portable desktop"},
      {label: "Download Linux CLI (x86_64)", href: `/downloads/gnomcode-v${version}-linux-x86_64.tar.gz`, detail: "Linux x86_64 archive"},
      {label: "Download Linux CLI (arm64)", href: `/downloads/gnomcode-v${version}-linux-aarch64.tar.gz`, detail: "Linux aarch64 archive"},
    ] as DownloadAction[],
    command: "curl -fsSL https://gnomcode.vercel.app/downloads/install.sh | bash",
    note: "Linux desktop and CLI packages verified in CI. ARM64 and musl are CLI-only distributions.",
  },
] as const;

function detectPlatform(): (typeof platforms)[number]["id"] {
  const platform = `${navigator.userAgent} ${navigator.platform}`.toLowerCase();
  if (platform.includes("mac") || platform.includes("iphone") || platform.includes("ipad")) return "macos";
  if (platform.includes("linux") || platform.includes("android")) return "linux";
  return "windows";
}

export function PlatformDownloads() {
  const [selected, setSelected] = useState<(typeof platforms)[number]["id"]>("windows");
  const tabs = useRef<Array<HTMLButtonElement | null>>([]);

  useEffect(() => setSelected(detectPlatform()), []);

  const activeIndex = platforms.findIndex(({ id }) => id === selected);
  const active = platforms[activeIndex];

  function handleKeyDown(event: React.KeyboardEvent<HTMLButtonElement>) {
    if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const next = event.key === "Home" ? 0 : event.key === "End" ? platforms.length - 1
      : (activeIndex + (event.key === "ArrowRight" ? 1 : platforms.length - 1)) % platforms.length;
    setSelected(platforms[next].id);
    tabs.current[next]?.focus();
  }

  return (
    <section className="sp-platform-picker" aria-label="Choose your platform">
      <div className="sp-platform-tabs" role="tablist" aria-label="Operating system">
        {platforms.map((platform, index) => (
          <button
            key={platform.id}
            ref={(node) => { tabs.current[index] = node; }}
            id={`platform-tab-${platform.id}`}
            type="button"
            role="tab"
            aria-selected={selected === platform.id}
            aria-controls="platform-panel"
            tabIndex={selected === platform.id ? 0 : -1}
            onClick={() => setSelected(platform.id)}
            onKeyDown={handleKeyDown}
          >
            {platform.name}
          </button>
        ))}
      </div>
      <article className="sp-platform-panel" id="platform-panel" role="tabpanel" aria-labelledby={`platform-tab-${active.id}`}>
        <header>
          <h2>{active.name}</h2>
          <span className={active.actions.length ? "sp-platform-state" : "sp-platform-state sp-platform-pending"}>{active.availability}</span>
        </header>
        <p className="sp-platform-detail">{active.detail}</p>
        <p className="sp-platform-body">{active.body}</p>
        {active.command && <div className="sp-install-command"><span>QUICK INSTALL</span><CopyCommand command={active.command} /></div>}
        {active.actions.length ? (
          <div className="sp-download-actions">
            {active.actions.map((action, index) => (
              <a className={index === 0 ? "sp-download-action sp-download-primary" : "sp-download-action"} href={action.href} key={action.href}>
                <span>{action.label}</span><small>{action.detail}</small>
              </a>
            ))}
          </div>
        ) : <p className="sp-download-unavailable">Choose your installation path in the <a href="/docs/getting-started">getting started guide</a>.</p>}
        <p className="sp-platform-note">{active.note} <a href={`/downloads/SHA256SUMS-v${version}.txt`}>SHA-256 checksums</a></p>
      </article>
    </section>
  );
}
