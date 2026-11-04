/**
 * VAPID Web Push configuration.
 * Environment variables configured in Vercel / host environment:
 * - NEXT_PUBLIC_VAPID_PUBLIC_KEY
 * - VAPID_PRIVATE_KEY
 * - VAPID_SUBJECT
 */

export const VAPID_PUBLIC_KEY = process.env.NEXT_PUBLIC_VAPID_PUBLIC_KEY || "";
export const VAPID_PRIVATE_KEY = process.env.VAPID_PRIVATE_KEY || "";
export const VAPID_SUBJECT = process.env.VAPID_SUBJECT || "mailto:support@gnomcode.com";

export function isVapidConfigured(): boolean {
  return Boolean(VAPID_PUBLIC_KEY && VAPID_PRIVATE_KEY);
}
