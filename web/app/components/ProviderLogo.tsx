import { PROVIDER_LOGOS } from "../../lib/provider-logos.js";

/** The provider mark the console uses (web/lib/provider-logos.js), as an inline 24x24 SVG. */
export function ProviderLogo({ provider, size = 20 }: { provider: "claude-code" | "codex" | "antigravity"; size?: number }) {
  return <svg className="sp-provider-logo" viewBox="0 0 24 24" width={size} height={size} fill="currentColor" aria-hidden="true" focusable="false"><path d={PROVIDER_LOGOS[provider]} /></svg>;
}
