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
  const [oauthLoading, setOauthLoading] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      if (session) {
        router.push("/console");
      }
    });
  }, [router]);

  const handleOAuth = async (provider: "github" | "google") => {
    setError("");
    setMessage("");
    setOauthLoading(true);
    try {
      const { error } = await supabase.auth.signInWithOAuth({
        provider,
        options: {
          redirectTo: typeof window !== "undefined" ? `${window.location.origin}/console` : undefined,
        },
      });
      if (error) setError(error.message);
    } catch {
      setError("GitHub sign-in could not start. Check your connection and try again.");
    } finally {
      setOauthLoading(false);
    }
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
    <main className="sp-auth-page">
      <section className="sp-auth-panel">
        <header className="sp-auth-heading">
          <h1>spawnt / account</h1>
          <p>
          {mode === "signin" && "Sign in to access your synced account and cloud console."}
          {mode === "signup" && "Create a spawnt account to sync your account settings across devices."}
          {mode === "forgot" && "Enter your registered email address to receive password reset instructions."}
          </p>
        </header>

        <div className="sp-auth-tabs" role="group" aria-label="Account action">
          <button
            type="button" aria-pressed={mode === "signin"}
            onClick={() => { setMode("signin"); setError(""); setMessage(""); }}
            className={mode === "signin" ? "selected" : ""}
          >
            Sign in
          </button>
          <button
            type="button" aria-pressed={mode === "signup"}
            onClick={() => { setMode("signup"); setError(""); setMessage(""); }}
            className={mode === "signup" ? "selected" : ""}
          >
            Create account
          </button>
          <button
            type="button" aria-pressed={mode === "forgot"}
            onClick={() => { setMode("forgot"); setError(""); setMessage(""); }}
            className={mode === "forgot" ? "selected" : ""}
          >
            Reset password
          </button>
        </div>

        <div className="sp-auth-content" aria-live="polite">
        {error && (
          <div className="sp-auth-feedback sp-auth-error" role="alert" aria-live="assertive">
            {error}
          </div>
        )}
        {message && (
          <div className="sp-auth-feedback" role="status" aria-live="polite">
            {message}
          </div>
        )}

        {mode !== "forgot" && (
          <>
            <button
              onClick={() => handleOAuth("github")}
              disabled={oauthLoading || loading}
              className="sp-auth-oauth"
            >
              <svg className="w-4 h-4 fill-current" viewBox="0 0 24 24">
                <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
              </svg>
              {oauthLoading ? "Opening GitHub..." : "Continue with GitHub"}
            </button>

            <div className="sp-auth-divider"><span>Or use email and password</span></div>
          </>
        )}

        <form onSubmit={handleEmailAuth} className="sp-auth-form">
          <div>
            <label htmlFor="auth-email">Email address</label>
            <input
              id="auth-email"
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="sp-auth-input"
              placeholder="developer@example.com"
            />
          </div>

          {mode !== "forgot" && (
            <div>
              <div className="sp-auth-password-label">
                <label htmlFor="auth-password">Password</label>
                {mode === "signin" && (
                  <button
                    type="button"
                    onClick={() => setMode("forgot")}
                    className="sp-auth-forgot"
                  >
                    Forgot password?
                  </button>
                )}
              </div>
              <input
                id="auth-password"
                type="password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="sp-auth-input"
                placeholder="********"
              />
            </div>
          )}

          <button
            type="submit"
            disabled={loading || oauthLoading}
            className="sp-auth-submit"
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
        </div>

        <nav className="sp-auth-links" aria-label="More options">
          <Link href="/console">
            ← Local console
          </Link>
          <Link href="/downloads">
            Download app ↓
          </Link>
        </nav>
      </section>
    </main>
  );
}
