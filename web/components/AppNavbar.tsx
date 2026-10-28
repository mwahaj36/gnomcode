"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import ThemeControl from "./ThemeControl";
export default function AppNavbar() {
  const onSettings = usePathname().startsWith("/settings");
  return <header className="sp-app-header"><div className="sp-app-header-inner"><div className="sp-app-brand-nav"><Link href="/console" className="sp-app-brand" aria-label="spawnt console"><span className="sp-brand-mark" aria-hidden="true"/><span>spawnt</span></Link><nav className="sp-app-nav" aria-label="App navigation"><Link href="/settings" aria-current={onSettings ? "page" : undefined} className={onSettings ? "selected" : ""}>Settings</Link></nav></div><div className="sp-app-actions"><ThemeControl className="sp-app-theme"/></div></div></header>;
}
