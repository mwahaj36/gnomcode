# Test the Agent IDE on Windows

This is an unsigned development build. The installer is
`apps/desktop/out/make/squirrel.windows/x64/Gnomcode-0.9.0 Setup.exe`.
The portable ZIP is in `apps/desktop/out/make/zip/win32/x64/`.

## Upgrade and restart

1. Choose **Quit** from the old Gnomcode tray menu. Closing its window only hides it.
2. Stop the old daemon before opening the new build. In PowerShell, run:

   ```powershell
   $gnomcodeDataRoot = if ($env:GNOMCODE_HOME) { $env:GNOMCODE_HOME } else { Join-Path $env:USERPROFILE '.gnomcode' }
   $gnomcodeDaemonFile = Join-Path $gnomcodeDataRoot 'daemon.json'
   if (Test-Path -LiteralPath $gnomcodeDaemonFile) {
       $gnomcodeDaemon = Get-Content -LiteralPath $gnomcodeDaemonFile -Raw | ConvertFrom-Json
       $gnomcodeDaemonProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $($gnomcodeDaemon.pid)"
       if ($gnomcodeDaemonProcess.Name -eq 'gnomcode.exe' -and $gnomcodeDaemonProcess.CommandLine -match '\bdaemon\b') {
           Stop-Process -Id $gnomcodeDaemon.pid
       }
   }
   ```

   This targets the recorded daemon, preserving independently hosted agents.
   Do not stop every process named `gnomcode`: PTY owners also use that executable.

3. Run the new installer, then open Gnomcode. Alternatively, extract the ZIP into
   a new folder and run `gnomcode-desktop.exe` there.
   Both development builds may report version 0.9.0; confirm the new Sessions
   sidebar and New Session launcher rather than relying on the version alone.

4. Start a **new** hosted session for interactive tests below. PTY owners already
   running from an older build retain their older input protocol.

## Acceptance checks

| Test | Steps | Expected result |
| :--- | :--- | :--- |
| Past conversations | Open **Sessions**, browse pages, search the current page, then select an old conversation. | Sidebar stays beside the conversation; user instructions and assistant text appear without Antigravity metadata or raw tool output. |
| Terminal shell | Choose **+ New Session**, select **Terminal Shell**, enter an existing absolute workspace path and launch. Click the terminal to type, or send `Write-Output 'GNOMCODE_TEST_OK'` from the prompt box. | PowerShell opens and prints `GNOMCODE_TEST_OK`. Arrow keys, Enter and Control+C work in the terminal. |
| Missing CLI | Select an agent that is not installed. | The launcher explains the missing PATH entry and shows install/login commands. Use Terminal Shell to install or sign in. Reopen New Session to check again; restart the daemon if PATH itself changed. |
| Agent conversation | Launch an installed, signed-in Codex, Claude Code or Gemini CLI in a test repository. Complete its workspace trust/login prompts in **Terminal stream**. Send `Reply with READY. Do not modify files or run commands.` | The official CLI receives the prompt. Its available structured conversation history refreshes in chat; terminal output remains accessible separately. |
| Multiline instructions | Send two short lines to a ready hosted agent. | The agent receives a single multiline instruction. The input clears only after delivery is acknowledged. |
| Model handoff | In an agent conversation, select a model available to your provider account and choose **Switch**. | Gnomcode launches a fresh compatible session with a redacted handoff. The source session remains running. Verify the new CLI accepts the model and receives the handoff. |
| Phone pairing | With a Pro account on the desktop, open **Settings**, choose **Start pairing**, and open its link on your phone. Compare the codes and confirm on both devices. | The phone connects through the public encrypted relay without a local-network requirement. |
| Phone chat and prompt | Select a native agent session on the phone. Wait until it is idle, then send a short instruction. | Clean history appears and subsequent responses refresh. Prompting is enabled only for an idle agent with a unique Gnomcode-hosted PTY binding. |
| Phone launch | In the paired companion, enter an existing workspace path **on the desktop** and launch an installed agent. Refresh sessions after startup. | The desktop starts that agent; the phone lists its session. Remote shell launch and terminal keystrokes are unavailable. |
| Session survival | Keep a newly hosted agent or shell open, choose Quit in Gnomcode, restart the daemon as above, then reopen Gnomcode. | The hosted process survives and can be reattached through Terminal stream. |

The model handoff creates a new session; it does not change the model inside the
original CLI process. Provider model availability and authentication are controlled
by the official CLIs. Physical-phone acceptance and independent Remote security
review remain manual release checks.

If a check fails, report the step, agent/CLI version, displayed error and whether
the problem happens in chat, terminal or the phone companion. Do not share pairing
links, bearer tokens or provider credentials.
