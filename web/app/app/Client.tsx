"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import AppNavbar from "../../components/AppNavbar";
import {
  getNotificationPermission,
  requestNotificationPermission,
  triggerServerPushTest,
  isStandalonePWA,
  type NotificationStatus
} from "../../lib/notifications";

export default function AppLauncherClient() {
  const [notifStatus, setNotifStatus] = useState<NotificationStatus>("default");
  const [isStandalone, setIsStandalone] = useState(false);
  const [testSent, setTestSent] = useState(false);
  const [notificationMessage,setNotificationMessage]=useState('');
  const [installPrompt, setInstallPrompt] = useState<any>(null);
  const [installed, setInstalled] = useState(false);

  useEffect(() => {
    setNotifStatus(getNotificationPermission());
    setIsStandalone(isStandalonePWA());

    const handleBeforeInstall = (e: Event) => {
      e.preventDefault();
      setInstallPrompt(e);
    };

    window.addEventListener("beforeinstallprompt", handleBeforeInstall);
    window.addEventListener("appinstalled", () => {
      setInstalled(true);
      setInstallPrompt(null);
    });

    return () => {
      window.removeEventListener("beforeinstallprompt", handleBeforeInstall);
    };
  }, []);

  const handleEnableNotifs = async () => {
    const granted = await requestNotificationPermission();
    setNotifStatus(granted ? "granted" : getNotificationPermission());
    if(granted){const result=await triggerServerPushTest();setNotificationMessage(result.message);}
  };

  const handleTestNotif = async () => {
    setTestSent(true);
    const result=await triggerServerPushTest();
    setNotificationMessage(result.message);
    setTestSent(false);
  };

  const handleInstallClick = async () => {
    if (!installPrompt) return;
    installPrompt.prompt();
    const { outcome } = await installPrompt.userChoice;
    if (outcome === "accepted") {
      setInstalled(true);
    }
    setInstallPrompt(null);
  };

  return (
    <div className="sp-app-workspace min-h-screen flex flex-col bg-bg text-[var(--sp-text)] font-sans">
      <AppNavbar />

      <main className="flex-1 max-w-4xl w-full mx-auto p-4 sm:p-6 md:p-8 space-y-6">
        <header className="border-b border-[var(--sp-border)] pb-4 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="text-xs font-mono tracking-wider uppercase px-2 py-0.5 border border-[var(--sp-border)] rounded text-[var(--sp-muted)]">
                {isStandalone ? "[PWA INSTALLED]" : "[WEB APP]"}
              </span>
              <span className="text-xs font-mono text-[var(--sp-muted)]">v0.9.0</span>
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-[var(--sp-text)]">Gnomcode Workspace</h1>
            <p className="text-sm text-[var(--sp-muted)] mt-1">
              Choose your interface: interactive mobile companion or desktop telemetry console.
            </p>
          </div>

          {installPrompt && !installed && (
            <button
              onClick={handleInstallClick}
              className="inline-flex items-center justify-center px-4 py-2 text-xs font-mono font-semibold uppercase tracking-wider bg-[var(--sp-text)] text-bg rounded hover:opacity-90 transition-opacity"
            >
              Install App on Device
            </button>
          )}
        </header>

        {/* Primary Interface Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Remote Companion Card */}
          <div className="border border-[var(--sp-border)] rounded-lg p-5 bg-[var(--sp-card)] flex flex-col justify-between hover:border-[var(--sp-accent)] transition-colors">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-mono uppercase tracking-wider text-[var(--sp-accent)] font-semibold">
                  [RECOMMENDED FOR MOBILE]
                </span>
                <span className="text-xs font-mono text-[var(--sp-muted)]">E2E Encrypted</span>
              </div>
              <h2 className="text-lg font-semibold text-[var(--sp-text)]">Remote Companion</h2>
              <p className="text-sm text-[var(--sp-muted)] leading-relaxed">
                Connect your phone or browser to active coding sessions. View conversation steps, respond to tool approval requests, and prompt Claude Code, Codex, or Antigravity from anywhere.
              </p>
            </div>
            <div className="mt-6 pt-4 border-t border-[var(--sp-border)] flex items-center justify-between">
              <span className="text-xs font-mono text-[var(--sp-muted)]">Zero telemetry</span>
              <Link
                href="/remote"
                className="px-4 py-2 text-xs font-mono uppercase tracking-wider font-semibold rounded bg-[var(--sp-accent)] text-white hover:opacity-90 transition-opacity"
              >
                Launch Remote
              </Link>
            </div>
          </div>

          {/* Fleet Console Card */}
          <div className="border border-[var(--sp-border)] rounded-lg p-5 bg-[var(--sp-card)] flex flex-col justify-between hover:border-[var(--sp-border-hover)] transition-colors">
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-mono uppercase tracking-wider text-[var(--sp-muted)]">
                  [TELEMETRY & FLEET]
                </span>
                <span className="text-xs font-mono text-[var(--sp-muted)]">Declared Trust</span>
              </div>
              <h2 className="text-lg font-semibold text-[var(--sp-text)]">Fleet Console</h2>
              <p className="text-sm text-[var(--sp-muted)] leading-relaxed">
                Review recorded token usage, estimated API spend across providers, quota pools, and active sessions running on your local machine.
              </p>
            </div>
            <div className="mt-6 pt-4 border-t border-[var(--sp-border)] flex items-center justify-between">
              <span className="text-xs font-mono text-[var(--sp-muted)]">Local-first SQLite</span>
              <Link
                href="/console"
                className="px-4 py-2 text-xs font-mono uppercase tracking-wider font-semibold rounded border border-[var(--sp-border)] text-[var(--sp-text)] hover:bg-[var(--sp-card-hover)] transition-colors"
              >
                Open Console
              </Link>
            </div>
          </div>
        </div>

        {notificationMessage && <p role="status">{notificationMessage}</p>}
        {/* Notifications & Device Companion Settings */}
        <div className="border border-[var(--sp-border)] rounded-lg p-5 bg-[var(--sp-card)] space-y-4">
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 border-b border-[var(--sp-border)] pb-3">
            <div>
              <h3 className="text-sm font-semibold text-[var(--sp-text)]">PWA & Desktop Notifications</h3>
              <p className="text-xs text-[var(--sp-muted)] mt-0.5">
                Receive instant alerts when an agent completes a task, halts for permission, or hits a quota reset.
              </p>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-xs font-mono px-2 py-0.5 border border-[var(--sp-border)] rounded text-[var(--sp-muted)] uppercase">
                Status: {notifStatus}
              </span>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3">
            {notifStatus !== "granted" ? (
              <button
                onClick={handleEnableNotifs}
                className="px-3 py-1.5 text-xs font-mono font-semibold uppercase tracking-wider rounded bg-[var(--sp-text)] text-bg hover:opacity-90 transition-opacity"
              >
                Enable Notifications
              </button>
            ) : (
              <span className="text-xs font-mono text-green-500 font-semibold">[NOTIFICATIONS ACTIVE]</span>
            )}

            <button
              onClick={handleTestNotif}
              className="px-3 py-1.5 text-xs font-mono font-semibold uppercase tracking-wider rounded border border-[var(--sp-border)] text-[var(--sp-text)] hover:bg-[var(--sp-card-hover)] transition-colors"
            >
              {testSent ? "Alert Triggered" : "Test Local Alert"}
            </button>

            <Link
              href="/settings"
              className="px-3 py-1.5 text-xs font-mono uppercase tracking-wider rounded text-[var(--sp-muted)] hover:text-[var(--sp-text)] transition-colors"
            >
              Manage Preferences
            </Link>
          </div>
        </div>

        {/* Web Push Configuration Note */}
        <div className="border border-[var(--sp-border)] rounded-lg p-4 bg-[var(--sp-card)] text-xs text-[var(--sp-muted)] space-y-2">
          <div className="font-mono text-[var(--sp-text)] font-semibold uppercase tracking-wider text-[11px]">
            [NOTIFICATION ARCHITECTURE]
          </div>
          <p>
            Enable and test background alerts on this device while signed in. Your laptop must stay awake with account Remote access enabled. On iPhone, open Gnomcode from the Home Screen.
          </p>
        </div>
      </main>
    </div>
  );
}
