"use client";

import { useState } from "react";

export function CopyCommand({ command }: { command: string }) {
  const [feedback, setFeedback] = useState("");

  async function copyCommand() {
    try {
      await navigator.clipboard.writeText(command);
      setFeedback("Copied");
    } catch {
      setFeedback("Copy failed. Select the command and copy it manually.");
    }
  }

  return (
    <div className="sp-copy-command">
      <code>{command}</code>
      <button type="button" onClick={copyCommand} aria-label={`Copy command: ${command}`}>
        Copy
      </button>
      <span className="sp-copy-command-status" role="status" aria-live="polite">{feedback}</span>
    </div>
  );
}
