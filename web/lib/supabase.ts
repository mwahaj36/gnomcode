import { createClient } from "@supabase/supabase-js";

const supabaseUrl =
  process.env.NEXT_PUBLIC_SUPABASE_URL ||
  "https://jjnuxngvhuvxkxrfmnqx.supabase.co";

const supabasePublishableKey =
  process.env.NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY ||
  "sb_publishable_o2ZiGak7RHq_cmL81BOeZQ_g7JJeyCI";

export const supabase = createClient(supabaseUrl, supabasePublishableKey);
