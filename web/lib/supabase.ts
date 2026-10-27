import { createClient } from "@supabase/supabase-js";

export const supabaseUrl =
  process.env.NEXT_PUBLIC_SUPABASE_URL ||
  "https://jjnuxngvhuvxkxrfmnqx.supabase.co";

export const supabasePublishableKey =
  process.env.NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY ||
  "sb_publishable_o2ZiGak7RHq_cmL81BOeZQ_g7JJeyCI";

export const supabase = createClient(supabaseUrl, supabasePublishableKey, {
  auth: {
    // Device pairing owns its independent OAuth callback and storage.
    detectSessionInUrl: typeof window === "undefined" || (window.location.pathname !== "/pair" && !sessionStorage.getItem("fleethud_pair_request")),
  },
});
