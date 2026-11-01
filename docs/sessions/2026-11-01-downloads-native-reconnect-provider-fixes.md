# Downloads, Windows startup, reconnect and provider switching

## Goals

Fix the missing Windows download action and broken inline links, add a useful website 404, diagnose the Windows `[Win32]` tray exception and chat reentry connection failure, detect installed Antigravity CLI, and exercise a real Codex/Antigravity switch with simulated credit exhaustion.

## Root causes and changes

- Windows PowerShell loaded Windows Forms but did not reference it when compiling the tray C# types. The real Add-Type block now declares its compiler references and fails immediately if compilation fails. The tray regression executes that actual block instead of providing a substitute Win32 class.
- Every download action array was empty. Restore the approved Windows development installer/ZIP and keep unpublished platforms unavailable. Normal inline legal/guide text replaces flex layouts that split sentence fragments. Missing routes return a themed HTTP 404 with recovery links.
- A failed ten-second directory lookup during chat restoration ended reconnect immediately. Retry transient discovery failures within the existing thirty-second connection budget. Revocation, identity mismatch and authentication validation remain enforced. Public-relay browser coverage leaves/reopens a chat and injects a timed-out lookup.
- Installed `agy.exe` was outside the desktop process PATH. The shared Rust executable resolver now checks the official Antigravity CLI installation directory as a fallback for discovery, background metadata and PTY launches. Its bounded native `agy models` catalog supplies validated model choices with configured status.
- Antigravity CLI 1.4.12 writes compatible transcripts under `antigravity-cli/brain`, separate from the IDE. Reuse the adapter, retain CLI surface, and look up CLI/IDE workspaces read-only. A real CLI fixture was anonymized with the repository script and snapshotted. No vendor logs or credentials are modified by ingestion.
- Hosted sessions in Git subdirectories could fail to bind when their native log omitted the Git remote. Match the exact workspace identity as well as the resolved Git project, retaining unique ownership and time checks; ambiguous native sessions remain unbound.
- Provider errors were incorrectly treated as active generation and blocked a handoff. Explicit switching now accepts stopped/error states while preserving generation/tool/permission guards.
- Live return-to-Codex testing found large pasted context could leave Enter inside its Windows paste-burst window. Input acknowledgment now follows the PTY write, and an End key flushes Codex's paste burst before Enter. This preserves text and never replays a prompt automatically.

## Verification

- Regression failures were observed before fixes for tray compilation, missing download actions, timed-out directory restoration, missing CLI path/root/workspace lookup, CLI surface, subdirectory binding, provider-error handoff and PTY acknowledgment/paste boundaries.
- Responsive browser checks cover downloads and real HTTP 404 responses in both themes at 320, 390 and 1440 pixels; screenshots were visually reviewed. Native console browser checks and web tests passed.
- Public-relay browser checks passed with encrypted enrollment, saved trust, leave/return, discovery timeout recovery, stalled socket recovery and account-wide access. This uses browser automation, not a physical phone.
- Real no-tool prompts in a dedicated test workspace exercised Codex GPT-6.1-Sol -> Antigravity Gemini 3.8 Flash (Low) -> Codex. Both recalled the synthetic marker; the returned Codex reply also recalled the acknowledgment added by Antigravity. Conversation history includes both providers. Credit exhaustion was simulated as an error state only in an isolated Spawnt database, without changing account balances or native history. The old handoff rejected it; the corrected handoff succeeded. Unit coverage also proves thinking/tool/approval states remain blocked.
- Live Codex multiline calibration reproduced swallowed Enter and verified the End-key boundary submits the same text. Automated host-input coverage verifies write acknowledgment and separate paste/boundary/submit frames.
- Full Rust quality gate, production web build and release packaging are required for the delivered artifact. Final publication/artifact evidence follows below.

## Boundaries

No database schema change or new migration is required. This remains the owner-approved unsigned 0.9.0 development rollout. Independent security review, physical-phone acceptance and macOS/Linux runtime acceptance remain open. No release tag, payment approval, account quota modification or update-manifest publication is included.
