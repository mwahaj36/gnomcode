import Link from "next/link";

export default function LandingPage() {
  const supportedAgents = [
    {
      name: "OpenAI Codex",
      tag: "CLI & Desktop App",
      desc: "Live token deltas, reasoning tokens, and exact 5h & 7d quota pools.",
    },
    {
      name: "Google Antigravity",
      tag: "IDE & Autonomous Agent",
      desc: "Full turn tracking, tool invocations, and dynamic model switches.",
    },
    {
      name: "Claude Code",
      tag: "CLI & Assistant",
      desc: "Multi-block deduplication, Opus 5 cache savings, and turn breakdowns.",
    },
    {
      name: "Google Gemini CLI",
      tag: "Terminal Assistant",
      desc: "Rewritten session history parsing and token spend calculations.",
    },
  ];

  const coreFeatures = [
    {
      title: "Zero Content Storage",
      desc: "No prompts, source code, or tool outputs are ever stored or transmitted. Activity summaries are scrubbed with local redactors before persistence.",
      badge: "PRIVACY FIRST",
    },
    {
      title: "Per-Model Quota Pools",
      desc: "Track independent 5-hour and 7-day rolling windows across all models. Always know your exact remaining capacity before hitting a hard limit.",
      badge: "EXACT LIMITS",
    },
    {
      title: "Subscription ROI Multiplier",
      desc: "Compare your actual API-equivalent token spend against your $20/mo or $100/mo flat fees to see your exact net savings in real time.",
      badge: "ROI TRACKING",
    },
    {
      title: "True Cross-Platform Desktop",
      desc: "Compiled native executable with zero terminal windows, system tray indicator, autostart on login, and continuous background tailing.",
      badge: "NATIVE DESKTOP",
    },
  ];

  return (
    <div className="max-w-7xl mx-auto px-6 py-16">
      {/* Hero Section */}
      <div className="text-center max-w-3xl mx-auto mb-16">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-accent/15 border border-accent/30 text-accent font-mono text-xs mb-6">
          <span className="w-2 h-2 rounded-full bg-accent animate-pulse"></span>
          <span>FleetHUD v1.3.0 Production Release</span>
        </div>
        <h1 className="text-4xl sm:text-5xl font-extrabold tracking-tight text-foreground mb-5 leading-tight">
          Live Mission Control for AI Coding Agents
        </h1>
        <p className="text-base sm:text-lg text-muted mb-8 font-sans leading-relaxed">
          Flight recorder, token spend attribution, and per-model quota monitoring for Claude Code, Codex, and Antigravity on Windows, macOS, and Linux.
        </p>
        <div className="flex flex-wrap justify-center gap-4 font-mono text-xs">
          <Link
            href="/console"
            className="px-6 py-3 rounded-md bg-accent text-bg font-semibold hover:opacity-90 transition-opacity flex items-center gap-2 shadow-lg shadow-accent/10"
          >
            Launch Web Console &rarr;
          </Link>
          <a
            href="/downloads/FleetHUD-Setup.exe"
            download="FleetHUD-Setup.exe"
            className="px-6 py-3 rounded-md bg-card border border-border text-foreground hover:border-accent transition-colors flex items-center gap-2"
          >
            Download Windows Installer (.exe) &darr;
          </a>
        </div>
      </div>

      {/* Terminal Preview Card */}
      <div className="max-w-4xl mx-auto mb-20 bg-card border border-border rounded-lg overflow-hidden shadow-2xl font-mono text-xs">
        <div className="bg-bg/80 border-b border-border px-4 py-2.5 flex items-center gap-2 text-muted">
          <div className="w-2.5 h-2.5 rounded-full bg-red/60"></div>
          <div className="w-2.5 h-2.5 rounded-full bg-accent/60"></div>
          <div className="w-2.5 h-2.5 rounded-full bg-green/60"></div>
          <span className="ml-2 text-[11px]">fleethud status &bull; loopback: 127.0.0.1:41738</span>
        </div>
        <div className="p-5 overflow-x-auto text-[#c9d1d9] space-y-2 leading-relaxed">
          <div className="text-muted">=== FLEETHUD ACTIVE SESSIONS ===</div>
          <div className="grid grid-cols-6 text-muted border-b border-border/60 pb-1 text-[11px]">
            <span>SESSION</span>
            <span>AGENT</span>
            <span>SURFACE</span>
            <span>MODEL</span>
            <span>STATUS</span>
            <span className="text-right">LAST ACTIVE</span>
          </div>
          <div className="grid grid-cols-6 text-foreground text-[11px] items-center">
            <span className="font-semibold">codex:01a17825</span>
            <span className="capitalize">Codex</span>
            <span>Desktop</span>
            <span>gpt-6.1-sol</span>
            <span className="text-green font-semibold">&bull; thinking</span>
            <span className="text-right text-muted">Just now</span>
          </div>
          <div className="grid grid-cols-6 text-foreground text-[11px] items-center">
            <span className="font-semibold">antigravity:b488</span>
            <span className="capitalize">Antigravity</span>
            <span>IDE</span>
            <span>Gemini 4 Argon</span>
            <span className="text-green font-semibold">&bull; tool</span>
            <span className="text-right text-muted">Just now</span>
          </div>
          <div className="grid grid-cols-6 text-muted text-[11px] items-center">
            <span>codex:01a176fa</span>
            <span className="capitalize">Codex</span>
            <span>Desktop</span>
            <span>gpt-6.1-sol</span>
            <span className="text-muted">&bull; idle</span>
            <span className="text-right text-muted">12m ago</span>
          </div>
        </div>
      </div>

      {/* Supported Agents */}
      <div className="mb-20">
        <div className="text-center mb-10">
          <h2 className="text-2xl font-bold tracking-tight text-foreground mb-2">
            Multi-Agent Support Out of the Box
          </h2>
          <p className="text-xs text-muted font-mono uppercase tracking-wider">
            Zero configuration required &bull; Auto-discovered local logs
          </p>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-5">
          {supportedAgents.map((ag) => (
            <div
              key={ag.name}
              className="bg-card border border-border rounded-lg p-5 hover:border-accent/50 transition-colors"
            >
              <div className="flex justify-between items-start mb-3">
                <h3 className="font-bold text-sm text-foreground">{ag.name}</h3>
                <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-bg border border-border text-muted">
                  {ag.tag}
                </span>
              </div>
              <p className="text-xs text-muted leading-relaxed">{ag.desc}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Core Features */}
      <div className="mb-20">
        <div className="text-center mb-10">
          <h2 className="text-2xl font-bold tracking-tight text-foreground mb-2">
            Engineered for High-Frequency Coding
          </h2>
          <p className="text-xs text-muted font-mono uppercase tracking-wider">
            Non-negotiable product invariants &bull; Strict offline resilience
          </p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
          {coreFeatures.map((feat) => (
            <div
              key={feat.title}
              className="bg-card border border-border rounded-lg p-6 hover:border-accent/40 transition-colors"
            >
              <div className="flex justify-between items-center mb-3">
                <h3 className="font-bold text-base text-foreground">{feat.title}</h3>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-accent/15 text-accent border border-accent/30 font-semibold">
                  {feat.badge}
                </span>
              </div>
              <p className="text-xs text-muted leading-relaxed font-sans">{feat.desc}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Final Call to Action */}
      <div className="bg-card border border-border rounded-xl p-8 sm:p-12 text-center max-w-3xl mx-auto">
        <h2 className="text-2xl sm:text-3xl font-bold text-foreground mb-4">
          Ready to Take Command of Your Fleet?
        </h2>
        <p className="text-xs sm:text-sm text-muted mb-8 max-w-xl mx-auto font-sans leading-relaxed">
          Open the web console or download the native Windows binary to see your live token spend and quota headroom right now.
        </p>
        <div className="flex flex-wrap justify-center gap-4 font-mono text-xs">
          <Link
            href="/console"
            className="px-6 py-2.5 bg-accent text-bg font-semibold rounded hover:opacity-90 transition-opacity"
          >
            Open Web Console &rarr;
          </Link>
          <Link
            href="/downloads"
            className="px-6 py-2.5 bg-bg border border-border text-foreground rounded hover:border-accent transition-colors"
          >
            Download Native App &darr;
          </Link>
        </div>
      </div>
    </div>
  );
}
