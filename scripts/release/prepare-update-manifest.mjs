#!/usr/bin/env node
// Prepare a local, unsigned manifest from reviewed Windows packaging output.
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, renameSync } from 'node:fs';
import path from 'node:path';

try {
  const [assetsDirectory, output] = process.argv.slice(2);
  if (!assetsDirectory || !output || process.argv.length !== 4) throw new Error('Usage: node scripts/release/prepare-update-manifest.mjs <assets-directory> <output-manifest.json>');
  const { version } = JSON.parse(readFileSync(new URL('../../web/package.json', import.meta.url), 'utf8'));
  const { releasesRepository, remoteRelayUrl } = JSON.parse(readFileSync(new URL('../../web/branding.json', import.meta.url), 'utf8'));
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('A stable package version is required.');
  // Unsigned field (same trust tier as notes_url): lets the relay move again later by
  // republishing this manifest alone, without a new release. See SPEC's Remote relay notes.
  if (!/^wss:\/\/[^?#&]+\/relay$/.test(remoteRelayUrl)) throw new Error('branding.json remoteRelayUrl must be a clean wss://.../relay URL.');
  const repository = new URL(releasesRepository);
  if (repository.protocol !== 'https:' || repository.hostname !== 'github.com' || repository.username || repository.password || repository.search || repository.hash) throw new Error('Expected the approved HTTPS GitHub release repository.');
  const name = 'Gnomcode-Setup.exe';
  const bytes = readFileSync(path.join(assetsDirectory, name));
  if (bytes.length < 2 || bytes.subarray(0, 2).toString() !== 'MZ') throw new Error('Expected a packaged Windows installer.');
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  const checksums = readFileSync(path.join(assetsDirectory, `SHA256SUMS-v${version}.txt`), 'utf8');
  if (!checksums.split(/\r?\n/).some(line => line === `${sha256}  ${name}`)) throw new Error('Installer does not match the versioned packaging checksums.');
  const manifest = { version, remote_relay_url: remoteRelayUrl, platforms: { 'win32-x64': {
    url: `${releasesRepository.replace(/\/$/, '')}/releases/download/v${version}/${name}`,
    sha256,
    file_name: name,
  } } };
  const temporary = `${output}.prepared-part`;
  writeFileSync(temporary, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  renameSync(temporary, output);
  console.log(`Prepared unsigned v${version} Windows x64 manifest. Sign and review before publication.`);
} catch (error) {
  console.error(error instanceof Error ? error.message : 'Manifest preparation failed.');
  process.exitCode = 1;
}
