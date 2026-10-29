"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { supabase } from "../lib/supabase";

export default function AccountLink({className = "", arrow = false, onClick}: {className?:string; arrow?:boolean; onClick?:()=>void}) {
  const [signedIn, setSignedIn] = useState<boolean | null>(null);
  useEffect(() => {
    const {data:{subscription}} = supabase.auth.onAuthStateChange((_event, session) => setSignedIn(Boolean(session?.user)));
    return () => subscription.unsubscribe();
  }, []);
  return <Link className={className} href={signedIn === false ? "/auth" : "/console"} onClick={onClick}>{signedIn === false ? "Log in" : "Console"}{arrow && <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M5 19 19 5M5 5h14v14" /></svg>}</Link>;
}
