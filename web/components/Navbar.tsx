"use client";

import { usePathname } from "next/navigation";
import MarketingNavbar from "./MarketingNavbar";
import AppNavbar from "./AppNavbar";

export default function Navbar() {
  const pathname = usePathname();
  const isAppRoute = pathname.startsWith("/console") || pathname.startsWith("/settings");

  if (isAppRoute) {
    return <AppNavbar />;
  }

  return <MarketingNavbar />;
}
