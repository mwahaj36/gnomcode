"use client";

import { usePathname } from "next/navigation";
import MarketingNavbar from "./MarketingNavbar";

export default function Navbar() {
  const pathname = usePathname();
  if (pathname === "/" || pathname.startsWith("/console") || pathname.startsWith("/settings") || pathname.startsWith("/remote")) return null;
  return <MarketingNavbar showTheme={pathname === "/auth"} />;
}
