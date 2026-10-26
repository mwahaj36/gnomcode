"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { supabase } from "../../lib/supabase";
import { BRAND } from "@/lib/brand";

type AuthMode = "signin" | "signup" | "forgot";

export default function AuthPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [mode, setMode] = useState<AuthMode>("signin");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  useEffect(() => {
    if (typeof window !== "undefined") {
      const params = new URLSearchParams(window.location.search);
      const cb = params.get("callback");
      if (cb) {
        localStorage.setItem("spawnt_auth_callback", cb);
      }
    }

    supabase.auth.getSession().then(({ data: { session } }) => {
      if (session) {
        const cb = localStorage.getItem("spawnt_auth_callback");
        if (cb) {
          localStorage.removeItem("spawnt_auth_callback");
          window.location.href = `${cb}?session=${encodeURIComponent(JSON.stringify(session))}`;
          return;
        }
        router.push("/console");
      }
    });
  }, [router]);

  const handleOAuth = async (provider: "github" | "google") => {
    setError("");
    setMessage("");
    const { error } = await supabase.auth.signInWithOAuth({
      provider,
      options: {
        redirectTo: typeof window !== "undefined" ? `${window.location.origin}/console` : undefined,
      },
    });
    if (error) setError(error.message);
  };

  const handleEmailAuth = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setMessage("");
    setLoading(true);

    try {
      if (mode === "signup") {
        const { error, data } = await supabase.auth.signUp({
          email,
          password,
          options: {
            emailRedirectTo: typeof window !== "undefined" ? `${window.location.origin}/console` : undefined,
          },
        });
        if (error) {
          setError(error.message);
        } else if (data.session) {
          router.push("/console");
        } else {
          setMessage("Confirmation link sent to your email. Check your inbox to confirm.");
        }
      } else if (mode === "signin") {
        const { error } = await supabase.auth.signInWithPassword({
          email,
          password,
        });
        if (error) setError(error.message);
        else router.push("/console");
      } else if (mode === "forgot") {
        const { error } = await supabase.auth.resetPasswordForEmail(email, {
          redirectTo: typeof window !== "undefined" ? `${window.location.origin}/settings` : undefined,
        });
        if (error) setError(error.message);
        else setMessage("Password reset instructions have been sent to your email.");
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="max-w-4xl mx-auto my-16 px-6 font-mono text-xs">
      <div className="grid grid-cols-1 md:grid-cols-2 border border-border rounded-[6px] bg-surface overflow-hidden">
        {/* Left column: Daytona Privacy Guarantee & Architecture */}
        <div className="p-8 border-b md:border-b-0 md:border-r border-border bg-surface-elevated/40 flex flex-col justify-between space-y-6">
          <div>
            <div className="inline-flex items-center gap-2 px-2.5 py-1 rounded-[4px] bg-surface border border-border text-[10px] text-muted mb-4 uppercase">
              <span className="w-1.5 h-1.5 rounded-[1px] bg-primary"></span>
              <span>ACCOUNT & SYNC //</span>
            </div>
            <h1 className="text-2xl font-bold tracking-tight text-foreground font-mono mb-3">
              {BRAND.name} cloud
            </h1>
            <p className="text-muted font-sans text-xs leading-relaxed mb-6">
              Connect an account to sync quota and budget rollups across multiple machines. Local monitoring remains 100% active even without cloud sync.
            </p>

            <div className="space-y-4 pt-4 border-t border-border">
              <div>
                <div className="text-primary font-bold text-[11px] mb-1">01 // ZERO CONTENT STORED</div>
                <p className="text-muted text-[11px] font-sans">
                  Prompts, source files, and tool outputs never leave your machine. Only aggregate token counters sync.
                </p>
              </div>

              <div>
                <div className="text-primary font-bold text-[11px] mb-1">02 // HMAC-SHA256 MASKING</div>
                <p className="text-muted text-[11px] font-sans">
                  Repository and directory paths are irreversibly hashed before reaching the cloud.
                </p>
              </div>

              <div>
                <div className="text-primary font-bold text-[11px] mb-1">03 // FAIL-OPEN LOCAL DB</div>
                <p className="text-muted text-[11px] font-sans">
                  Local SQLite retains full telemetry history even when completely offline.
                </p>
              </div>
            </div>
          </div>

          <div className="pt-4 border-t border-border flex items-center justify-between text-[11px] text-muted">
            <Link href="/console" className="hover:text-foreground">
              &larr; Open Local Console
            </Link>
            <Link href="/downloads" className="text-primary hover:underline font-bold">
              Download App &darr;
            </Link>
          </div>
        </div>

        {/* Right column: Auth Forms */}
        <div className="p-8 bg-surface">
          {/* Tab Switcher */}
          <div className="flex border-b border-border mb-6">
            <button
              onClick={() => { setMode("signin"); setError(""); setMessage(""); }}
              className={`pb-2.5 px-3 text-xs font-bold transition-colors border-b-2 -mb-px ${
                mode === "signin"
                  ? "border-primary text-primary"
                  : "border-transparent text-muted hover:text-foreground"
              }`}
            >
              [SIGN IN]
            </button>
            <button
              onClick={() => { setMode("signup"); setError(""); setMessage(""); }}
              className={`pb-2.5 px-3 text-xs font-bold transition-colors border-b-2 -mb-px ${
                mode === "signup"
                  ? "border-primary text-primary"
                  : "border-transparent text-muted hover:text-foreground"
              }`}
            >
              [CREATE ACCOUNT]
            </button>
            <button
              onClick={() => { setMode("forgot"); setError(""); setMessage(""); }}
              className={`pb-2.5 px-3 text-xs font-bold transition-colors border-b-2 -mb-px ${
                mode === "forgot"
                  ? "border-primary text-primary"
                  : "border-transparent text-muted hover:text-foreground"
              }`}
            >
              [RESET]
            </button>
          </div>

          {error && (
            <div className="mb-4 text-xs bg-primary/10 text-primary border border-primary/30 rounded-[4px] p-3">
              {error}
            </div>
          )}
          {message && (
            <div className="mb-4 text-xs bg-surface-elevated text-foreground border border-border rounded-[4px] p-3">
              {message}
            </div>
          )}

          {mode !== "forgot" && (
            <>
              <button
                onClick={() => handleOAuth("github")}
                className="w-full mb-4 flex items-center justify-center gap-2 py-2 px-4 rounded-[4px] bg-surface-elevated border border-border text-foreground font-bold text-xs hover:border-primary transition-colors"
              >
                <svg className="w-4 h-4 fill-current" viewBox="0 0 24 24">
                  <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
                </svg>
                <span>Continue with GitHub</span>
              </button>

              <div className="flex items-center my-4 text-[10px] text-muted">
                <div className="flex-1 border-t border-border"></div>
                <span className="px-3 uppercase">or email & password</span>
                <div className="flex-1 border-t border-border"></div>
              </div>
            </>
          )}

          <form onSubmit={handleEmailAuth} className="space-y-4">
            <div>
              <label className="block text-[11px] text-muted mb-1">EMAIL ADDRESS // </label>
              <input
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full bg-surface-elevated border border-border rounded-[4px] px-3 py-2 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
                placeholder="developer@example.com"
              />
            </div>

            {mode !== "forgot" && (
              <div>
                <div className="flex justify-between items-center mb-1">
                  <label className="text-[11px] text-muted">PASSWORD // </label>
                  {mode === "signin" && (
                    <button
                      type="button"
                      onClick={() => setMode("forgot")}
                      className="text-[10px] text-primary hover:underline"
                    >
                      Forgot password?
                    </button>
                  )}
                </div>
                <input
                  type="password"
                  required
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="w-full bg-surface-elevated border border-border rounded-[4px] px-3 py-2 text-xs text-foreground focus:outline-none focus:border-primary font-mono"
                  placeholder="••••••••"
                />
              </div>
            )}

            <button
              type="submit"
              disabled={loading}
              className="w-full py-2.5 bg-primary text-white font-bold rounded-[4px] text-xs hover:brightness-110 transition-all disabled:opacity-50 mt-2"
            >
              {loading
                ? "Processing..."
                : mode === "signup"
                ? "Create Account &rarr;"
                : mode === "signin"
                ? "Sign In &rarr;"
                : "Send Reset Link &rarr;"}
            </button>
          </form>
        </div>
      </div>
    </div>
  );
}
