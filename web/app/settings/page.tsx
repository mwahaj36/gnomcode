"use client";
import { useEffect } from "react";
import Link from "next/link";
export default function SettingsRoute() {
  useEffect(() => { window.location.replace("/console?view=settings"); }, []);
  return <main className="sp-console-shell mx-auto px-6 py-10"><p>Opening settings…</p><Link href="/console?view=settings">Open settings</Link></main>;
}
