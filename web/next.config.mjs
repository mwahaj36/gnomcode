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
