"use client";

import { useEffect, useState } from "react";
import { CHECK_ICON_SVG, COPY_ICON_SVG, copyTextToClipboard } from "../../lib/console-presentation";

export function CopyCommand({ command }: { command: string }) {
  const [feedback, setFeedback] = useState("");
  useEffect(() => {
    if (feedback !== "Copied") return;
    const timer = window.setTimeout(() => setFeedback(""), 2000);
    return () => window.clearTimeout(timer);
  }, [feedback]);

  async function copyCommand() {
    const ok = await copyTextToClipboard(command);
    setFeedback(ok ? "Copied" : "Copy failed. Select the command and copy it manually.");
  }

  return (
    <div className={`sp-copy-command${feedback.startsWith("Copy failed") ? " is-error" : ""}`}>
      <code>{command}</code>
      <button type="button" onClick={copyCommand} aria-label={feedback === "Copied" ? "Command copied" : `Copy command: ${command}`} title={feedback === "Copied" ? "Copied" : "Copy command"}>
        <span className="sp-copy-icon" dangerouslySetInnerHTML={{ __html: feedback === "Copied" ? CHECK_ICON_SVG : COPY_ICON_SVG }} />
      </button>
      <span className="sp-copy-command-status" role="status" aria-live="polite">{feedback}</span>
    </div>
  );
}
