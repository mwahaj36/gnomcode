"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { supabase } from "../../lib/supabase";

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
        localStorage.setItem("fleethud_auth_callback", cb);
      }
    }

    supabase.auth.getSession().then(({ data: { session } }) => {
      if (session) {
        const cb = localStorage.getItem("fleethud_auth_callback");
        if (cb) {
          localStorage.removeItem("fleethud_auth_callback");
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
          setMessage("Confirmation link sent to your email! Please check your inbox to confirm your account.");
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
        else setMessage("Password reset instructions have been sent to your email address.");
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="sp-console-shell max-w-md mx-auto my-16 px-4 font-mono">
      <div className="bg-card border border-border rounded-xl p-8 shadow-2xl">
        <div className="flex items-center gap-2 mb-2">
          <h1 className="text-xl font-bold tracking-tight text-foreground font-sans">
            spawnt / account
          </h1>
        </div>
        <p className="text-xs text-muted mb-6 font-sans">
          {mode === "signin" && "Sign in to access your cross-device fleet, live quota metrics, and multi-plan ROI."}
          {mode === "signup" && "Create your spawnt account to sync across multiple machines and manage team fleets."}
          {mode === "forgot" && "Enter your registered email address to receive password reset instructions."}
        </p>

        {/* Tab Switcher */}
        <div className="flex border-b border-border mb-6">
          <button
            onClick={() => { setMode("signin"); setError(""); setMessage(""); }}
            className={`pb-2 px-3 text-xs font-semibold transition-colors border-b-2 -mb-px ${
              mode === "signin"
                ? "border-accent text-accent"
                : "border-transparent text-muted hover:text-foreground"
            }`}
          >
            Sign In
          </button>
          <button
            onClick={() => { setMode("signup"); setError(""); setMessage(""); }}
            className={`pb-2 px-3 text-xs font-semibold transition-colors border-b-2 -mb-px ${
              mode === "signup"
                ? "border-accent text-accent"
                : "border-transparent text-muted hover:text-foreground"
            }`}
          >
            Create Account
          </button>
          <button
            onClick={() => { setMode("forgot"); setError(""); setMessage(""); }}
            className={`pb-2 px-3 text-xs font-semibold transition-colors border-b-2 -mb-px ${
              mode === "forgot"
                ? "border-accent text-accent"
                : "border-transparent text-muted hover:text-foreground"
            }`}
          >
            Reset
          </button>
        </div>

        {error && (
          <div className="mb-4 text-xs bg-red/15 text-red border border-red/30 rounded p-3">
            {error}
          </div>
        )}
        {message && (
          <div className="mb-4 text-xs bg-green/15 text-green border border-green/30 rounded p-3">
            {message}
          </div>
        )}

        {mode !== "forgot" && (
          <>
            <button
              onClick={() => handleOAuth("github")}
              className="sp-auth-oauth w-full min-h-11 mb-3 flex items-center justify-center gap-2 px-4 rounded font-semibold text-xs transition-colors"
            >
              <svg className="w-4 h-4 fill-current" viewBox="0 0 24 24">
                <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
              </svg>
              Continue with GitHub
            </button>

            <div className="flex items-center my-4 text-[10px] text-muted">
              <div className="flex-1 border-t border-border"></div>
              <span className="px-3 uppercase">or email & password</span>
              <div className="flex-1 border-t border-border"></div>
            </div>
          </>
        )}

        <form onSubmit={handleEmailAuth} className="space-y-3.5">
          <div>
            <label className="block text-[11px] text-muted mb-1">Email address</label>
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="w-full bg-bg border border-border rounded px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-accent"
              placeholder="developer@example.com"
            />
          </div>

          {mode !== "forgot" && (
            <div>
              <div className="flex justify-between items-center mb-1">
                <label className="text-[11px] text-muted">Password</label>
                {mode === "signin" && (
                  <button
                    type="button"
                    onClick={() => setMode("forgot")}
                    className="text-[10px] text-accent hover:underline"
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
                className="w-full bg-bg border border-border rounded px-3 py-1.5 text-xs text-foreground focus:outline-none focus:border-accent"
                placeholder="********"
              />
            </div>
          )}

          <button
            type="submit"
            disabled={loading}
            className="w-full py-2 bg-accent text-bg font-semibold rounded text-xs hover:opacity-90 transition-opacity disabled:opacity-50 mt-1"
          >
            {loading
              ? "Processing..."
              : mode === "signup"
              ? "Create spawnt Account"
              : mode === "signin"
              ? "Sign In to Console"
              : "Send Reset Link"}
          </button>
        </form>

        <div className="mt-6 pt-4 border-t border-border text-center text-xs flex justify-between items-center text-[11px]">
          <Link href="/console" className="text-muted hover:text-foreground">
            &larr; Skip to Local Console
          </Link>
          <Link href="/downloads" className="text-green hover:underline">
            Download App &darr;
          </Link>
        </div>
      </div>
    </div>
  );
}
