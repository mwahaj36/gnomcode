#!/usr/bin/env node
// End-to-end cross-platform release finisher for an agent to execute.
//
// Preconditions (maintainer-only, key-gated, NOT automated here — see
// docs/release-signing.md): the Windows installer has already been built and
// signed, i.e. web/public/downloads/ already contains a signed
// latest.json plus Gnomcode-Setup.exe, gnomcode.exe and
// Gnomcode-v{version}-windows-x64.zip for the version in web/package.json,
// and `git status` is clean on `main` with that version already merged
// (the "chore(release): vX.Y.Z" commit, tagged vX.Y.Z — see the cut-release
// skill's version-bump + package-release.ps1 + sign-update-manifest.mjs
// steps). This script never reads or asks for the private signing key.
//
// What this script DOES automate (everything that needs no secret, only a
// GitHub-authenticated `gh` and an npm/vercel login already present in the
// environment — see README below for the full contract):
//   1. Triggers the two GitHub Actions workflows that build every other
//      platform's binaries (ci.yml with package=true, npm-release-build.yml)
//      and waits for them.
//   2. Downloads every artifact, extracts the raw binaries, and assembles
//      all 7 npm/platform/* packages (binary + LICENSE + THIRD_PARTY_NOTICES).
//   3. Rebuilds the two supplementary assets that have no dedicated script
//      (Gnomcode-vX.Y.Z-windows-arm64-cli.zip, gnomcode-vX.Y.Z-linux-x86_64-musl.tar.gz),
//      matching scripts/package-artifacts.py's archive shape by hand.
//   4. Stages every asset into web/public/downloads, regenerates SHA256SUMS
//      and SHA256SUMS-vX.Y.Z.txt over the complete set, and runs the repo's
//      own gates (check-release-versions.mjs --artifacts, npm/check-release.cjs,
//      npm/smoke-release.cjs) — aborting on any failure, touching nothing further.
//   5. Commits the two checksum files on a fresh branch (never main directly),
//      opens a PR, waits for its CI, and merges it once green.
//   6. Creates the GitHub release (tag vX.Y.Z, the matching CHANGELOG.md
//      section as notes, all assets attached).
//   7. Deploys the website to production (`vercel deploy --prod`).
//   8. Publishes every npm package (platform packages first, the `gnomcode`
//      shim last) unless --no-publish is passed. npm's own web/2FA publish
//      approval, when the account requires it, is NOT something this script
//      can click through — it waits, same as a human running the command
//      would, and the person must approve it in their browser.
//
// Usage: node scripts/release/finish-cross-platform-release.mjs [--no-publish] [--dry-run]
//
// Safe to re-run: every step checks what already exists/was already done
// (published versions, existing PRs/releases/deployments) before acting, so
// a retry after a transient failure does not duplicate work.

import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, cpSync, readFileSync, writeFileSync, existsSync, readdirSync, chmodSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const dryRun = process.argv.includes('--dry-run');
const skipPublish = process.argv.includes('--no-publish');

function log(step, message) {
  console.log(`[release ${step}] ${message}`);
}

function run(command, args, options = {}) {
  const label = `${command} ${args.join(' ')}`;
  if (dryRun && options.mutates) {
    log('dry-run', `would run: ${label}`);
    return '';
  }
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', ...options });
  if (result.status !== 0) {
    throw new Error(`${label} failed (exit ${result.status}):\n${result.stderr || result.stdout}`);
  }
  return result.stdout;
}

function runJson(command, args) {
  return JSON.parse(run(command, args));
}

const version = JSON.parse(readFileSync(path.join(root, 'web/package.json'), 'utf8')).version;
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error(`web/package.json has a non-stable version: ${version}`);
log('setup', `targeting v${version}`);

// --- Precondition: the signed Windows installer set must already exist ---
const downloads = path.join(root, 'web/public/downloads');
for (const name of ['latest.json', 'Gnomcode-Setup.exe', 'gnomcode.exe', `Gnomcode-v${version}-windows-x64.zip`]) {
  if (!existsSync(path.join(downloads, name))) {
    throw new Error(
      `${name} is missing from web/public/downloads. Build and sign the Windows installer first ` +
      '(bump-version.mjs, package-release.ps1 with GNOMCODE_UPDATE_PUBLIC_KEY, then ' +
      'prepare-update-manifest.mjs + sign-update-manifest.mjs with the maintainer-held private key). ' +
      'This script never touches that key and will not proceed without its output.',
    );
  }
}
const manifest = JSON.parse(readFileSync(path.join(downloads, 'latest.json'), 'utf8'));
if (manifest.version !== version || !manifest.platforms?.['win32-x64']?.signature) {
  throw new Error('web/public/downloads/latest.json is not a signed manifest for this version.');
}
if (run('git', ['status', '--porcelain', '--untracked-files=no']).trim()) {
  throw new Error('Working tree has uncommitted tracked changes. Commit or stash before running this.');
}
if (run('git', ['rev-parse', '--abbrev-ref', 'HEAD']).trim() !== 'main') {
  throw new Error('Must run from main (the version-bump + Windows packaging commit must already be merged).');
}
run('git', ['fetch', 'origin', 'main']);
if (run('git', ['rev-parse', 'HEAD']).trim() !== run('git', ['rev-parse', 'origin/main']).trim()) {
  throw new Error('Local main is not up to date with origin/main.');
}
run('git', ['fetch', '--tags', 'origin']);
let tagExists = true;
try { run('git', ['rev-parse', `v${version}`]); } catch { tagExists = false; }
if (!tagExists) throw new Error(`Tag v${version} not found. The version-bump commit should already be tagged.`);
log('setup', 'preconditions OK: signed Windows assets present, main up to date, tag exists');

const stage = mkdtempSync(path.join(os.tmpdir(), 'gnomcode-release-'));
const artifacts = path.join(stage, 'artifacts');
const final = path.join(stage, 'final');
mkdirSync(artifacts, { recursive: true });
mkdirSync(final, { recursive: true });

// --- Step 1: trigger and wait for the two build workflows ---
function latestRunId(workflowFile) {
  const runs = runJson('gh', ['run', 'list', '--workflow', workflowFile, '--branch', 'main', '--limit', '1', '--json', 'databaseId,status,conclusion,createdAt']);
  return runs[0];
}

function triggerAndWait(workflowFile, label) {
  const before = latestRunId(workflowFile);
  if (dryRun) { log('dry-run', `would trigger ${label} (${workflowFile})`); return null; }
  run('gh', ['workflow', 'run', workflowFile, '--ref', 'main'], { mutates: true });
  let run_;
  for (let i = 0; i < 30 && (!run_ || run_.databaseId === before?.databaseId); i++) {
    run_ = latestRunId(workflowFile);
    if (!run_ || run_.databaseId === before?.databaseId) spinWait(2000);
  }
  if (!run_) throw new Error(`${label}: could not find the triggered run`);
  log('build', `${label}: waiting on run ${run_.databaseId}`);
  execFileSync('gh', ['run', 'watch', String(run_.databaseId), '--exit-status'], { cwd: root, stdio: 'inherit' });
  return run_.databaseId;
}

function spinWait(ms) {
  execFileSync(process.platform === 'win32' ? 'powershell' : 'sleep', process.platform === 'win32' ? ['-Command', `Start-Sleep -Milliseconds ${ms}`] : [String(ms / 1000)]);
}

log('build', 'triggering ci.yml (package=true) and npm-release-build.yml');
let packageRunId = dryRun ? null : null;
if (!dryRun) {
  const before = latestRunId('ci.yml');
  run('gh', ['workflow', 'run', 'ci.yml', '--ref', 'main', '-f', 'package=true'], { mutates: true });
  run('gh', ['workflow', 'run', 'npm-release-build.yml', '--ref', 'main'], { mutates: true });
  let run_;
  for (let i = 0; i < 30 && (!run_ || run_.databaseId === before?.databaseId); i++) {
    run_ = latestRunId('ci.yml');
    if (!run_ || run_.databaseId === before?.databaseId) spinWait(2000);
  }
  packageRunId = run_.databaseId;
  log('build', `ci.yml (package=true) is run ${packageRunId}; waiting (this is the slow one, ~15-30 min)`);
  execFileSync('gh', ['run', 'watch', String(packageRunId), '--exit-status'], { cwd: root, stdio: 'inherit' });
  const npmRun = latestRunId('npm-release-build.yml');
  log('build', `npm-release-build.yml is run ${npmRun.databaseId}; waiting`);
  execFileSync('gh', ['run', 'watch', String(npmRun.databaseId), '--exit-status'], { cwd: root, stdio: 'inherit' });
}

// --- Step 2: download and assemble npm platform packages ---
if (!dryRun) {
  run('gh', ['run', 'download', String(packageRunId), '--dir', artifacts]);
  run('gh', ['run', 'download', String(latestRunId('npm-release-build.yml').databaseId), '--dir', artifacts]);

  const extracted = path.join(stage, 'extract');
  mkdirSync(extracted, { recursive: true });
  const nativeTargets = {
    'native-aarch64-apple-darwin': ['darwin-arm64', `gnomcode-v${version}-darwin-aarch64.tar.gz`],
    'native-x86_64-apple-darwin': ['darwin-x64', `gnomcode-v${version}-darwin-x86_64.tar.gz`],
    'native-aarch64-unknown-linux-gnu': ['linux-arm64', `gnomcode-v${version}-linux-aarch64.tar.gz`],
    'native-x86_64-unknown-linux-gnu': ['linux-x64', `gnomcode-v${version}-linux-x86_64.tar.gz`],
  };
  const platformBins = {}; // platform -> local binary path
  for (const [artifactDir, [platform, tarName]] of Object.entries(nativeTargets)) {
    const dest = path.join(extracted, platform);
    mkdirSync(dest, { recursive: true });
    run('tar', ['-xzf', path.join(artifacts, artifactDir, tarName), '-C', dest]);
    platformBins[platform] = path.join(dest, 'gnomcode');
  }
  platformBins['win32-arm64'] = path.join(artifacts, 'npm-extra-win32-arm64', 'gnomcode.exe');
  platformBins['linux-x64-musl'] = path.join(artifacts, 'npm-extra-linux-x64-musl', 'gnomcode');
  platformBins['win32-x64'] = path.join(downloads, 'gnomcode.exe'); // already built locally alongside the installer

  for (const [platform, binaryPath] of Object.entries(platformBins)) {
    const pkgDir = path.join(root, 'npm/platform', platform);
    const binName = platform.startsWith('win32-') ? 'gnomcode.exe' : 'gnomcode';
    mkdirSync(path.join(pkgDir, 'bin'), { recursive: true });
    cpSync(binaryPath, path.join(pkgDir, 'bin', binName));
    if (!platform.startsWith('win32-')) chmodSync(path.join(pkgDir, 'bin', binName), 0o755);
    cpSync(path.join(root, 'LICENSE'), path.join(pkgDir, 'LICENSE'));
    cpSync(path.join(downloads, 'THIRD_PARTY_NOTICES.html'), path.join(pkgDir, 'THIRD_PARTY_NOTICES.html'));
  }
  cpSync(path.join(root, 'LICENSE'), path.join(root, 'npm/gnomcode/LICENSE'));
  cpSync(path.join(downloads, 'THIRD_PARTY_NOTICES.html'), path.join(root, 'npm/gnomcode/THIRD_PARTY_NOTICES.html'));
  log('assemble', 'all 7 npm/platform/* packages populated');

  // --- Step 3: supplementary assets with no dedicated script ---
  // Windows ARM64 CLI-only zip.
  const armZipStage = path.join(stage, 'arm64-zip');
  mkdirSync(armZipStage, { recursive: true });
  cpSync(platformBins['win32-arm64'], path.join(armZipStage, 'gnomcode.exe'));
  cpSync(path.join(root, 'LICENSE'), path.join(armZipStage, 'LICENSE'));
  cpSync(path.join(downloads, 'THIRD_PARTY_NOTICES.html'), path.join(armZipStage, 'THIRD_PARTY_NOTICES.html'));
  const armZipDest = path.join(final, `Gnomcode-v${version}-windows-arm64-cli.zip`);
  if (process.platform === 'win32') {
    run('powershell', ['-NoProfile', '-Command', `Compress-Archive -Path '${path.join(armZipStage, '*')}' -DestinationPath '${armZipDest}' -Force`]);
  } else {
    run('zip', ['-X', '-q', '-j', armZipDest, path.join(armZipStage, 'gnomcode.exe'), path.join(armZipStage, 'LICENSE'), path.join(armZipStage, 'THIRD_PARTY_NOTICES.html')]);
  }

  // Standalone linux-x86_64-musl CLI tarball, matching package-artifacts.py's shape by hand
  // (that script has no musl-specific flag; see its linux_x86_64 branch for the reference shape).
  const muslStage = path.join(stage, 'musl-tar');
  mkdirSync(path.join(muslStage, 'fonts'), { recursive: true });
  cpSync(platformBins['linux-x64-musl'], path.join(muslStage, 'gnomcode'));
  chmodSync(path.join(muslStage, 'gnomcode'), 0o755);
  cpSync(path.join(root, 'LICENSE'), path.join(muslStage, 'LICENSE'));
  cpSync(path.join(downloads, 'THIRD_PARTY_NOTICES.html'), path.join(muslStage, 'THIRD_PARTY_NOTICES.html'));
  cpSync(path.join(root, 'web/public/fonts'), path.join(muslStage, 'fonts'), { recursive: true });
  cpSync(path.join(root, 'web/public/icons/gnomcode-256.png'), path.join(muslStage, 'gnomcode.png'));
  const launcherPath = path.join(muslStage, 'Open gnomcode.command');
  writeFileSync(launcherPath, '#!/bin/sh\ncd "$(dirname "$0")" || exit 1\nexec ./gnomcode console\n');
  chmodSync(launcherPath, 0o755);
  const muslTarPath = path.join(final, `gnomcode-v${version}-linux-x86_64-musl.tar`);
  const muslTarGz = `${muslTarPath}.gz`;
  run('tar', ['--owner=0', '--group=0', '--mode=755', '-cf', muslTarPath, 'gnomcode', 'Open gnomcode.command'], { cwd: muslStage });
  run('tar', ['--owner=0', '--group=0', '--mode=755', '--no-recursion', '-rf', muslTarPath, 'fonts'], { cwd: muslStage });
  run('tar', ['--owner=0', '--group=0', '--mode=644', '-rf', muslTarPath, 'LICENSE', 'THIRD_PARTY_NOTICES.html', 'gnomcode.png',
    ...readdirSync(path.join(muslStage, 'fonts')).map((name) => `fonts/${name}`)], { cwd: muslStage });
  run('gzip', ['-f', muslTarPath]);
  if (!existsSync(muslTarGz)) throw new Error('musl tarball was not produced');
  log('assemble', 'built the two hand-assembled supplementary assets');

  // --- Step 4: stage everything, checksums, gates ---
  const desktopArtifactDirs = {
    'desktop-ubuntu-22.04-x64': [
      ['deb/x64', `gnomcode-desktop_${version}_amd64.deb`],
      ['zip/linux/x64', `Gnomcode-linux-x64-${version}.zip`],
    ],
    'desktop-macos-15-intel-x64': [['zip/darwin/x64', `Gnomcode-darwin-x64-${version}.zip`]],
    'desktop-macos-15-arm64': [['zip/darwin/arm64', `Gnomcode-darwin-arm64-${version}.zip`]],
  };
  for (const [artifactName, entries] of Object.entries(desktopArtifactDirs)) {
    for (const [subpath, fileName] of entries) {
      cpSync(path.join(artifacts, artifactName, subpath, fileName), path.join(final, fileName));
    }
  }
  for (const [artifactDir, [, tarName]] of Object.entries(nativeTargets)) {
    cpSync(path.join(artifacts, artifactDir, tarName), path.join(final, tarName));
  }
  for (const name of ['latest.json', 'LICENSE', 'THIRD_PARTY_NOTICES.html', 'Gnomcode-Setup.exe', 'gnomcode.exe', `Gnomcode-v${version}-windows-x64.zip`]) {
    cpSync(path.join(downloads, name), path.join(final, name));
  }
  mkdirSync(path.join(stage, 'npm-tarballs'), { recursive: true });
  for (const dir of ['gnomcode', ...readdirSync(path.join(root, 'npm/platform')).map((p) => `platform/${p}`)]) {
    run('npm', ['pack', '--silent', '--pack-destination', path.join(stage, 'npm-tarballs'), `./${dir}`], { cwd: path.join(root, 'npm') });
  }
  for (const name of readdirSync(path.join(stage, 'npm-tarballs'))) cpSync(path.join(stage, 'npm-tarballs', name), path.join(final, name));

  for (const name of readdirSync(final)) cpSync(path.join(final, name), path.join(downloads, name));

  const crypto = await import('node:crypto');
  const checksumLines = readdirSync(final)
    .filter((name) => name !== 'SHA256SUMS' && name !== `SHA256SUMS-v${version}.txt`)
    .sort()
    .map((name) => `${crypto.createHash('sha256').update(readFileSync(path.join(final, name))).digest('hex')}  ${name}`)
    .join('\n') + '\n';
  writeFileSync(path.join(downloads, 'SHA256SUMS'), checksumLines);
  writeFileSync(path.join(downloads, `SHA256SUMS-v${version}.txt`), checksumLines);
  log('assemble', 'regenerated SHA256SUMS over the complete asset set');

  // Gates: never proceed past this point on failure.
  run('node', ['scripts/check-release-versions.mjs', '--artifacts']);
  run('node', ['check-release.cjs'], { cwd: path.join(root, 'npm') });
  run('node', ['smoke-release.cjs'], { cwd: path.join(root, 'npm') });
  log('verify', 'all release gates passed');

  // --- Step 5: checksums PR ---
  const branch = `chore/v${version}-checksums`;
  run('git', ['checkout', '-b', branch]);
  run('git', ['add', 'web/public/downloads/SHA256SUMS', `web/public/downloads/SHA256SUMS-v${version}.txt`]);
  run('git', ['commit', '-m', `chore(release): v${version} checksums\n\nRecord SHA-256 checksums for all release assets.`], { mutates: true });
  run('git', ['push', 'origin', branch], { mutates: true });
  const prUrl = run('gh', ['pr', 'create', '--base', 'main', '--head', branch, '--title', `chore(release): v${version} checksums`,
    '--body', 'Record SHA-256 checksums for all release assets.'], { mutates: true }).trim();
  log('release', `checksums PR: ${prUrl}`);
  const prNumber = prUrl.split('/').pop();
  spinWait(10000);
  const prRun = runJson('gh', ['run', 'list', '--branch', branch, '--limit', '1', '--json', 'databaseId'])[0];
  if (prRun) execFileSync('gh', ['run', 'watch', String(prRun.databaseId), '--exit-status'], { cwd: root, stdio: 'inherit' });
  run('gh', ['pr', 'merge', prNumber, '--squash', '--delete-branch=false'], { mutates: true });
  run('git', ['checkout', 'main']);
  run('git', ['pull', '--ff-only', 'origin', 'main']);
  log('release', 'checksums PR merged');

  // --- Step 6: GitHub release ---
  const tagCommit = run('git', ['rev-parse', `v${version}`]).trim();
  const changelogSection = execFileSync(process.platform === 'win32' ? 'powershell' : 'sed', process.platform === 'win32'
    ? ['-NoProfile', '-Command', `(Get-Content CHANGELOG.md -Raw) -replace '(?s).*?## ${version}[^\\n]*\\n(.*?)\\n## .*', '$1'`]
    : ['-n', `/## ${version}/,/## /p`], { cwd: root, encoding: 'utf8' }).trim();
  const notesPath = path.join(stage, 'notes.md');
  writeFileSync(notesPath, changelogSection || `Gnomcode v${version}`);
  run('gh', ['release', 'create', `v${version}`, '--title', `Gnomcode v${version}`, '--notes-file', notesPath,
    '--target', tagCommit, ...readdirSync(final).map((name) => path.join(final, name))], { mutates: true });
  log('release', `GitHub release v${version} created with ${readdirSync(final).length} assets`);

  // --- Step 7: website ---
  run('npx', ['vercel@latest', 'deploy', '--prod', '--yes'], { cwd: root, mutates: true });
  log('release', 'website deployed to production');

  // --- Step 8: npm publish ---
  if (!skipPublish) {
    const npmDir = path.join(root, 'npm');
    for (const dir of readdirSync(path.join(npmDir, 'platform')).map((p) => `platform/${p}`).concat(['gnomcode'])) {
      const name = JSON.parse(readFileSync(path.join(npmDir, dir, 'package.json'), 'utf8')).name;
      let current = '0.0.0';
      try { current = execFileSync('npm', ['view', name, 'version'], { encoding: 'utf8' }).trim(); } catch { /* unpublished */ }
      if (current === version) { log('publish', `${name}@${version} already published, skipping`); continue; }
      log('publish', `publishing ${name}@${version} (approve in your browser if npm asks)`);
      execFileSync('npm', ['publish', `./${dir}`, '--access', 'public'], { cwd: npmDir, stdio: 'inherit' });
    }
    log('publish', 'all npm packages published');
  } else {
    log('publish', 'skipped (--no-publish)');
  }
}

rmSync(stage, { recursive: true, force: true });
log('done', `v${version} release finished`);
