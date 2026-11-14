export const faqGroups = [
  {
    "title": "Getting started",
    "questions": [
      {
        "question": "Do I need a Gnomcode account?",
        "answer": "No account is needed for local features. Optional account metadata sync and Remote discovery use a Gnomcode account; your agents keep separate vendor accounts."
      },
      {
        "question": "Does Gnomcode work offline?",
        "answer": "Local reporting, inspection and orchestration do not require Gnomcode cloud services. Official agent inference may still require the vendor network; disable update checks and leave optional services off to avoid Gnomcode network traffic."
      },
      {
        "question": "Which operating systems are supported?",
        "answer": "Downloads are Windows x64 only. On Windows, Gnomcode also discovers sessions inside WSL distros and can launch agents there. macOS and Linux builds are not available."
      },
      {
        "question": "Do I install agent CLIs separately?",
        "answer": "Yes. Install and authenticate the official CLI you want to launch, then let Gnomcode discover it on PATH. Gnomcode does not supply provider subscriptions or credentials."
      }
    ]
  },
  {
    "title": "Supported agents",
    "questions": [
      {
        "question": "Which agents does Gnomcode observe?",
        "answer": "Gnomcode 1.0 supports exactly Claude Code, Codex and Antigravity. Available token, model and quota fields vary by source."
      },
      {
        "question": "Does Gnomcode support Gemini CLI, Cursor, Aider, Cline or Teams?",
        "answer": "No. Gemini CLI, Cursor, Aider and Cline are not supported, and there is no Teams product. Antigravity uses Google's Gemini models and is supported. History from removed agents stays on disk and is not shown as a supported agent."
      },
      {
        "question": "Will more agents be added?",
        "answer": "New support needs a documented source contract, anonymized fixtures and capability tests. No future agent or release date is promised by the current support list."
      }
    ]
  },
  {
    "title": "Model switching",
    "questions": [
      {
        "question": "What is a zero-token handoff?",
        "answer": "A compact briefing assembled locally without an AI summarization call. The receiving model still consumes input tokens and can charge for them."
      },
      {
        "question": "Does the new agent receive my full history?",
        "answer": "It receives redacted objectives, changed-file paths and recent activity rather than a full replay. The original conversation remains available locally; provider-internal state is not transferred."
      },
      {
        "question": "What happens to the old session?",
        "answer": "Its history is preserved while the handoff creates a fresh target session. Inspect lifecycle state before allowing both agents to edit the same workspace."
      },
      {
        "question": "Can I switch back?",
        "answer": "You can select an eligible supported target again or return to the original history. A fresh handoff does not merge provider-internal sessions or guarantee native resume across every CLI."
      }
    ]
  },
  {
    "title": "Remote access",
    "questions": [
      {
        "question": "How do I pair my phone?",
        "answer": "Confirm account access when linking the native app. Every device signed in to that account can then open its available sessions automatically. No phone pairing or per-desktop approval is required."
      },
      {
        "question": "Is Remote independently security approved?",
        "answer": "The E2EE implementation and public-relay browser tests exist. Independent review and explicit production approval remain launch gates."
      },
      {
        "question": "What can I do from my phone?",
        "answer": "Read available chat history, receive replies, launch supported CLIs, choose a workspace, send eligible initial/follow-up prompts, interrupt a turn, browse the session's changes, files and artifacts read-only, and attach files to a prompt. Imported history stays read-only until you choose Continue here, which reopens it through the agent's own resume command."
      },
      {
        "question": "Can I approve permission prompts from my phone?",
        "answer": "Yes, for all three agents. Claude Code: the optional hook from gnomcode integrate claude forwards a one-line summary of the prompt. Codex and Antigravity: turn on protocol mode first with gnomcode protocol-approvals enable codex (or antigravity); it is off by default and applies to new launches. Timeouts and errors leave the decision to the agent's own dialog, and nothing is ever allowed by default."
      },
      {
        "question": "What can I not do remotely?",
        "answer": "Run an arbitrary shell, escalate permissions or bypass native approval checks. Remote access also depends on an online desktop and uniquely owned live agent session."
      },
      {
        "question": "Does pairing survive a reload?",
        "answer": "Native consent survives updates, reloads and inactivity. Devices can disable a native app for all signed-in devices. Native sign-out clears its consent; browser sign-out clears local connection credentials."
      }
    ]
  },
  {
    "title": "Pricing and privacy",
    "questions": [
      {
        "question": "What is free and what is paid?",
        "answer": "All local features are free without a local license. Remote is a paid category; commercial prices and terms are not finalized. There is no Teams product."
      },
      {
        "question": "Does Gnomcode charge per token?",
        "answer": "Gnomcode reports estimated API-equivalent usage rather than reselling inference. Your agent provider determines its own subscription or usage charges; final paid Gnomcode service terms require launch approval."
      },
      {
        "question": "Does Gnomcode read my agent API keys?",
        "answer": "It does not read official agent subscription tokens for launch or model discovery. Optional Gnomcode account authentication uses its own separate credentials; avoid typing secrets into conversations because redaction is not perfect."
      },
      {
        "question": "What data leaves my machine?",
        "answer": "Optional account sync sends a non-E2EE metadata snapshot, currently including labels and automation metadata. Paired Remote deliberately carries conversation and commands as ciphertext to your trusted device; chat events are excluded from metadata sync."
      },
      {
        "question": "Can I delete local history?",
        "answer": "Session deletion removes Gnomcode chat rows without modifying agent logs. A complete store reset requires stopping Gnomcode and handling state.db, sidecars and backups; preserved agent logs can be ingested again."
      }
    ]
  },
  {
    "title": "Troubleshooting",
    "questions": [
      {
        "question": "Why is my agent not detected?",
        "answer": "Start the official agent and run gnomcode doctor. Check its supported log location, home overrides and PATH, then restart Gnomcode after a CLI installation."
      },
      {
        "question": "Why is my quota unknown?",
        "answer": "Not every agent records a reliable quota window. Gnomcode leaves missing data unknown; inspect your official CLI or provider usage page rather than treating unknown as zero."
      },
      {
        "question": "Why will the companion not connect?",
        "answer": "Check desktop connectivity, current builds and the same account on both devices. Generate a fresh offer if the discovery code expired; if remembered trust was revoked, pair again."
      }
    ]
  }
];
