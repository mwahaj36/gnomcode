#!/usr/bin/env node
// Offline release tooling. Never uploads artifacts or prints key material.
import { createHash, createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { readFileSync, writeFileSync, renameSync } from 'node:fs';
import path from 'node:path';

try {
  const [manifestPath, assetsDir] = process.argv.slice(2);
  if (!manifestPath || !assetsDir || process.argv.length !== 4) throw new Error('Usage: node scripts/release/sign-update-manifest.mjs <manifest.json> <assets-directory>');
  const pem = process.env.GNOMCODE_UPDATE_PRIVATE_KEY;
  const keyFile = process.env.GNOMCODE_UPDATE_PRIVATE_KEY_FILE;
  if (Boolean(pem) === Boolean(keyFile)) throw new Error('Provide exactly one private-key source: GNOMCODE_UPDATE_PRIVATE_KEY or GNOMCODE_UPDATE_PRIVATE_KEY_FILE.');
  let key;
  try {
    key = createPrivateKey(pem || readFileSync(keyFile));
  } catch {
    throw new Error('Unable to load the private key. Supply an unencrypted Ed25519 PEM through the secure release environment.');
  }
  if (key.asymmetricKeyType !== 'ed25519') throw new Error('An Ed25519 private key is required.');
  const publicKey = Buffer.from(createPublicKey(key).export({ format: 'jwk' }).x, 'base64url').toString('hex');
  if (publicKey !== process.env.GNOMCODE_UPDATE_PUBLIC_KEY?.trim().toLowerCase()) throw new Error('Signing key does not match GNOMCODE_UPDATE_PUBLIC_KEY used for the release build.');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  if (!/^\d+\.\d+\.\d+$/.test(manifest.version) || !manifest.platforms || !Object.keys(manifest.platforms).length) throw new Error('Manifest requires a stable version and at least one platform.');
  for (const asset of Object.values(manifest.platforms)) {
    const name = asset.file_name || new URL(asset.url).pathname.split('/').pop();
    if (!name || !/^[A-Za-z0-9._-]+$/.test(name) || name.includes('..')) throw new Error('Unsafe asset filename.');
    const bytes = readFileSync(path.join(assetsDir, name));
    const hash = createHash('sha256').update(bytes).digest('hex');
    if (!bytes.length || hash !== asset.sha256?.toLowerCase()) throw new Error('Asset checksum mismatch; manifest was not changed.');
    asset.signature = sign(null, bytes, key).toString('hex');
  }
  const temporary = `${manifestPath}.signed-part`;
  writeFileSync(temporary, JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  renameSync(temporary, manifestPath);
  console.log(`Signed ${Object.keys(manifest.platforms).length} update asset(s). No files uploaded.`);
} catch (error) {
  // Node key-decoding errors may include inputs: only explicit safe messages escape.
  const safe = error instanceof Error && !error.code;
  console.error(safe ? error.message : 'Signing failed while reading or writing release inputs. Manifest was not published.');
  process.exitCode = 1;
}
