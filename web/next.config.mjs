/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  devIndicators: false,
  distDir: process.env.GNOMCODE_NEXT_DIST_DIR || '.next',
  async redirects() {
    return [
      { source: "/pricing", destination: "/docs/getting-started", permanent: true },
      // The pre-1.2 launcher was replaced by the console; keep old bookmarks and installed shortcuts working.
      { source: "/app", destination: "/console", permanent: true },
    ];
  },
  async rewrites() {
    return [
      {
        source: "/win",
        destination: "/downloads/install.ps1",
      },
      {
        source: "/download",
        destination: "/downloads/Gnomcode-Setup.exe",
      },
      {
        source: "/sh",
        destination: "/downloads/install.sh",
      },
      {
        source: "/install.sh",
        destination: "/downloads/install.sh",
      },
    ];
  },
};

export default nextConfig;
