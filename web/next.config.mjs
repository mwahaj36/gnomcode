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
        destination: "/downloads/Spawnt-Setup.exe",
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
