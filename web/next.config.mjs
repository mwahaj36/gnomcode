/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  async rewrites() {
    return [
      {
        source: "/win",
        destination: "/downloads/install.ps1",
      },
      {
        source: "/download",
        destination: "/downloads/FleetHUD-Setup.exe",
      },
    ];
  },
};

export default nextConfig;
