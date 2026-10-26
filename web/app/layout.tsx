import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "FleetHUD — Mission Control",
  description: "Live mission control and flight recorder for AI coding agents",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="bg-bg text-[#c9d1d9] antialiased p-6 min-h-screen">
        {children}
      </body>
    </html>
  );
}
