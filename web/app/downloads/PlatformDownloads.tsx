"use client";

import { useEffect, useRef, useState } from "react";
import { CopyCommand } from "../components/CopyCommand";

const platforms = [
  {
    id: "windows", name: "Windows", availability: "AVAILABLE", detail: "Windows 10 / 11 · x64",
    body: "Install the tray app and local daemon. The standalone installer and zip contain the same v1.3.0 release.",
    actions: [
      { label: "Download installer", href: "/downloads/FleetHUD-Setup.exe", detail: ".exe" },
      { label: "Download archive", href: "/downloads/FleetHUD-v1.3.0-windows-x64.zip", detail: ".zip" },
    ], command: "irm https://fleethud.vercel.app/win | iex",
    note: "Updates preserve the account and recorded sessions.",
  },
  {
    id: "macos", name: "macOS", availability: "NOT PUBLISHED", detail: "Apple Silicon / Intel",
    body: "A verified macOS release is not available yet. We will publish an installer after native builds and signing are ready.",
    actions: [], command: null, note: "No download path is enabled for macOS today.",
  },
  {
    id: "linux", name: "Linux", availability: "AVAILABLE", detail: "x86_64 / ARM64 · glibc 2.35+",
    body: "Install the CLI and local web daemon from the verified v1.3.0 tarball. WSL2 is supported.",
    actions: [
      { label: "Install script", href: "/sh", detail: "SHA-256 verified" },
      { label: "Release checksums", href: "/downloads/SHA256SUMS-v1.3.0.txt", detail: "SHA256SUMS" },
    ], command: "curl -fsSL https://fleethud.vercel.app/sh | bash",
    note: "Updates preserve local user data and account pairing.",
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
        ) : <p className="sp-download-unavailable">There is no verified installer to download yet.</p>}
        <p className="sp-platform-note">{active.note}</p>
      </article>
    </section>
  );
}
