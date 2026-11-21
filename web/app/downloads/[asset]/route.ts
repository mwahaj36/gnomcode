import packageInfo from '../../../package.json';
import branding from '../../../branding.json';
const { version } = packageInfo;
const assets = new Set([
  'Gnomcode-Setup.exe', 'gnomcode.exe', 'LICENSE', 'THIRD_PARTY_NOTICES.html',
  `Gnomcode-v${version}-windows-x64.zip`, `Gnomcode-v${version}-windows-arm64-cli.zip`,
  `Gnomcode-darwin-arm64-${version}.zip`, `Gnomcode-darwin-x64-${version}.zip`,
  `Gnomcode-linux-x64-${version}.zip`, `gnomcode-desktop_${version}_amd64.deb`,
  `gnomcode-v${version}-darwin-aarch64.tar.gz`, `gnomcode-v${version}-darwin-x86_64.tar.gz`,
  `gnomcode-v${version}-linux-aarch64.tar.gz`, `gnomcode-v${version}-linux-x86_64.tar.gz`,
  `gnomcode-v${version}-linux-x86_64-musl.tar.gz`, `SHA256SUMS-v${version}.txt`, 'SHA256SUMS',
]);
export async function GET(_request: Request, { params }: { params: Promise<{ asset: string }> }) {
  const { asset } = await params;
  // Manual release artifacts are available; never advertise an unsigned automatic update.
  if (asset === 'latest.json') return Response.json({ message: 'Automatic update unavailable. Download the current installer from /downloads.' }, { status: 503, headers: { 'Cache-Control': 'no-store' } });
  if (!assets.has(asset)) return new Response('Release asset not found.', { status: 404 });
  return Response.redirect(`${branding.releasesRepository}/releases/download/v${version}/${asset}`, 302);
}
