/**
 * Central brand constants. URLs intentionally keep their current values until
 * the spawnt.xyz domain is purchased; swapping the domain is a one-line change here.
 */
export const BRAND_NAME = "spawnt";
export const TAGLINE = "See what your coding agents are doing.";
export const SUBLINE =
  "Live status, cost and limits for your agents. Local-first. Nothing leaves your machine.";
export const PRIVACY_LINE =
  "spawnt reads local logs and stores metadata only. Prompts, code and output are never saved or sent.";

export const SITE_URL = "https://fleethud.vercel.app";
export const DOWNLOAD_BASE = `${SITE_URL}/downloads`;

export const BRAND = {
  name: BRAND_NAME,
  tagline: TAGLINE,
  subline: SUBLINE,
  privacyInvariant: PRIVACY_LINE,
  version: "1.2.1",
  siteUrl: SITE_URL,
  downloadBase: DOWNLOAD_BASE,
};
