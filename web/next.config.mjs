/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  devIndicators: false,
  distDir: process.env.GNOMCODE_NEXT_DIST_DIR || '.next',
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
