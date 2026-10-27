"use client";

import { useEffect, useState } from "react";

export function CopyCommand({ command }: { command: string }) {
  const [feedback, setFeedback] = useState("");
  useEffect(() => {
    if (feedback !== "Copied") return;
    const timer = window.setTimeout(() => setFeedback(""), 2000);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  async function copyCommand() {
    try {
      await navigator.clipboard.writeText(command);
      setFeedback("Copied");
    } catch {
      setFeedback("Copy failed. Select the command and copy it manually.");
    }
  }

  return (
    <div className={`sp-copy-command${feedback.startsWith("Copy failed") ? " is-error" : ""}`}>
      <code>{command}</code>
      <button type="button" onClick={copyCommand} aria-label={feedback === "Copied" ? "Command copied" : `Copy command: ${command}`} title={feedback === "Copied" ? "Copied" : "Copy command"}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          {feedback === "Copied" ? <path d="m5 12 4 4L19 6" /> : <><path d="M9 5H6a1 1 0 0 0-1 1v14h14V6a1 1 0 0 0-1-1h-3" /><rect x="9" y="3" width="6" height="4" rx="1" /></>}
        </svg>
      </button>
      <span className="sp-copy-command-status" role="status" aria-live="polite">{feedback}</span>
    </div>
  );
}
