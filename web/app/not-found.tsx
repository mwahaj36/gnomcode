import Link from "next/link";

export const metadata = {title: "Page not found", robots: {index: false, follow: false}};

export default function NotFound() {
  return <section className="sp-not-found">
    <h1>404 — Page not found</h1>
    <p>This page may have moved, or the link may be incorrect. Head home or find what you need below.</p>
    <nav aria-label="Page recovery">
      <Link className="sp-download-action sp-download-primary" href="/">Go to homepage</Link>
      <Link href="/downloads">Downloads</Link>
      <Link href="/docs">Documentation</Link>
    </nav>
  </section>;
}
