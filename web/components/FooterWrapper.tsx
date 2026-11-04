"use client";

import { usePathname } from "next/navigation";
import MarketingFooter from "./MarketingFooter";

export default function FooterWrapper() {
  const pathname = usePathname();
  if (pathname === "/") return null;
  const isAppRoute = pathname.startsWith("/console") || pathname.startsWith("/settings") || pathname.startsWith("/auth") || pathname.startsWith("/remote") || pathname.startsWith("/app");

  if (isAppRoute) {
    return null;
  }

  return <MarketingFooter />;
}
