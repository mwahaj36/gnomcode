import DocPage from "../../../components/DocPage";
import { pageMetadata, buildHowToJsonLd } from "../../../lib/seo";
const howTo = buildHowToJsonLd("How to install Gnomcode", "Install the free Gnomcode runtime for Claude Code, Codex and Antigravity on Windows, macOS or Linux.", [
  { name: "Install Gnomcode", text: "On Windows run irm https://gnomcode.vercel.app/downloads/install.ps1 | iex in PowerShell. On macOS or Linux run curl -fsSL https://gnomcode.vercel.app/downloads/install.sh | bash. With Node.js on any platform run npm install -g gnomcode." },
  { name: "Install an agent CLI", text: "Install and sign in to Claude Code, OpenAI Codex or Antigravity with its official CLI." },
  { name: "Open the console", text: "Run gnomcode console, choose a workspace folder and an installed agent, then launch." },
  { name: "Check the setup", text: "Run gnomcode doctor if an agent does not appear." },
]);
import { CopyCommand } from "../../components/CopyCommand";
export const metadata = pageMetadata("/docs/getting-started", "Getting Started with Gnomcode | Gnomcode", "Install Gnomcode, launch your first official agent CLI and find its conversation, terminal, usage and quota in one workspace.");
export default function Page() { return <DocPage title={"Getting Started with Gnomcode"} answer={"Install Gnomcode with one command (PowerShell on Windows, curl on macOS and Linux, or npm anywhere), install and sign into an official agent CLI, then choose a workspace and launch it from Gnomcode. Local features require no Gnomcode account; each agent keeps its own provider account and permissions."}>
<script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(howTo).replace(/</g, "\\u003c") }} />
<section><h2>{"How do I install Gnomcode?"}</h2><p>{"Gnomcode is free and open source. Pick the command for your system, or download an installer or archive from the downloads page and compare it with the published SHA-256 checksums. Windows installers are not Authenticode-signed and macOS apps are not notarized, so your OS may ask you to confirm the first launch."}</p>
<h3>Windows (PowerShell)</h3><CopyCommand command={"irm https://gnomcode.vercel.app/downloads/install.ps1 | iex"} />
<h3>macOS and Linux</h3><CopyCommand command={"curl -fsSL https://gnomcode.vercel.app/downloads/install.sh | bash"} />
<h3>Any platform with Node.js</h3><CopyCommand command={"npm install -g gnomcode"} />
<p>{"Then run gnomcode --version to confirm the install and gnomcode console to open the workspace."}</p></section>
<section><h2>{"How do I open the first session?"}</h2><p>{"Run gnomcode console to start the local console. Select a folder and an installed Claude Code, Codex or Antigravity CLI, then launch. Complete vendor login, workspace trust and permission prompts in Terminal stream. Send normal instructions in the conversation composer; use Shift+Enter for a new line."}</p></section>
<section><h2>How do files and approvals work?</h2><p>Open Files for a workspace folder tree, Changes for Git diffs, or Artifacts for documents referenced by Claude Code, Codex or Antigravity. Click a local document link in a reply to preview it. Expand preview gives the document more room; Download saves the complete file when available.</p><p>On desktop, choose Trust this project to remember tool and folder approval for future sessions in that exact workspace. Revoke it from Workspace. Permission and sandbox changes still ask individually. Remote approvals remain approve-once.</p></section>
<section><h2>{"Which agents should I connect?"}</h2><p>{"Claude Code, OpenAI Codex and Antigravity have hosted CLI launch paths; these are the only three agents 1.0 supports. Existing logs appear after local discovery; imported history stays read-only until you choose Continue here."}</p></section>
<section><h2>{"What should I check if no agent appears?"}</h2><p>{"Run gnomcode doctor and start a real session in your official CLI. Check PATH and the supported log roots, including CLAUDE_CONFIG_DIR and CODEX_HOME overrides, and GEMINI_HOME for Antigravity. Restart Gnomcode after installing a CLI. Model and quota data can remain unknown until the agent records them."}</p></section>
<section><h2>{"How do I use the CLI?"}</h2><p>{"Use the commands below with your own workspace path. Separate worktrees avoid two agents editing the same files at once."}</p></section>
<section><h2>CLI examples</h2><CopyCommand command={"gnomcode console"} /><CopyCommand command={"gnomcode doctor"} /><CopyCommand command={"gnomcode launch codex --repo ./project-a"} /></section>
</DocPage>; }
