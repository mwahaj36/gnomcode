"use client";

import { usePathname } from "next/navigation";
import MarketingFooter from "./MarketingFooter";

export default function FooterWrapper() {
  const pathname = usePathname();
  const isAppRoute = pathname.startsWith("/console") || pathname.startsWith("/settings") || pathname.startsWith("/auth");

  if (isAppRoute) {
    return null;
  }

  return <MarketingFooter />;
}
