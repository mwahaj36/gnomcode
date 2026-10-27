import type { Metadata } from "next";
import LandingClient from "./LandingClient";

export const metadata: Metadata = {
  title: "spawnt — See what your coding agents are doing.",
  description: "Live status, cost and limits for your agents. Local-first. Nothing leaves your machine.",
};

export default function LandingPage() {
  return <LandingClient />;
}
