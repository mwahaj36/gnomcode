import type { Metadata } from "next";
import branding from "../branding.json";

export function pageMetadata(path: string, title: string, description: string, index = true): Metadata {
  const url = `${branding.baseUrl}${path}`;
  const images = [{ url: `${branding.baseUrl}/og-image.png`, width: 1200, height: 630 }];
  return {
    title: { absolute: title }, description,
    alternates: { canonical: url },
    openGraph: { title, description, url, siteName: "Gnomcode", type: "website", images },
    twitter: { card: "summary_large_image", title, description, images: images.map(image => image.url) },
    robots: { index, follow: index },
  };
}
