"use client";

import { useEffect, useRef, useState } from "react";
import { CopyCommand } from "../components/CopyCommand";
import packageInfo from "../../package.json";
const { version } = packageInfo;

type DownloadAction = { label: string; href: string; detail: string };

const platforms = [
  {
    id: "windows", name: "Windows", availability: "DEVELOPMENT BUILD", detail: "Windows 10 / 11 · x64",
    body: "Launch official coding agents, continue conversations and keep a separate workspace terminal close at hand.",
    actions: [
      {label: "Download Windows installer", href: "/downloads/Gnomcode-Setup.exe", detail: "Windows x64"},
      {label: "Download ZIP", href: `/downloads/Gnomcode-v${version}-windows-x64.zip`, detail: "Portable archive"},
    ] as DownloadAction[], command: null,
    note: "Unsigned development build. Check the SHA-256 manifest before installing.",
  },
  {
    id: "macos", name: "macOS", availability: "NOT AVAILABLE YET", detail: "Apple Silicon / Intel",
    body: "Bring your agent conversations, recorded usage and model-switch workflow into one local workspace.",
    actions: [] as DownloadAction[], command: null, note: "Read the installation guide for platform requirements and release channels.",
  },
  {
    id: "linux", name: "Linux", availability: "NOT AVAILABLE YET", detail: "x86_64 / ARM64 · glibc 2.35+",
    body: "Use the standalone CLI and local console to orchestrate agents and inspect usage on your own machine.",
    actions: [] as DownloadAction[], command: null,
    note: "Read the installation guide for current release channels and verification steps.",
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
        <p className="sp-platform-note">{active.note} {active.id==='windows' && <a href={`/downloads/SHA256SUMS-v${version}.txt`}>SHA-256 checksums</a>}</p>
      </article>
    </section>
  );
}
