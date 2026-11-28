#!/usr/bin/env node
// Builds the signed release APK into mobile/dist/gnomcode-android-<version>.apk (ADR 0066).
//
// Required environment:
//   JAVA_HOME                  JDK 21
//   ANDROID_HOME               Android SDK (platform 36, build-tools)
//   GNOMCODE_ANDROID_SIGNING   properties file with storeFile, storePassword, keyAlias, keyPassword
// Optional:
//   GNOMCODE_GOOGLE_SERVICES   path to google-services.json (Firebase push); never committed
//   --debug                    unsigned-for-release debug build, for local testing
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const android = path.join(root, 'android');
const debug = process.argv.includes('--debug');
const { version } = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));

function fail(message) {
  console.error(`[gnomcode:android] ${message}`);
  process.exit(1);
}

for (const name of ['JAVA_HOME', 'ANDROID_HOME']) if (!process.env[name] || !existsSync(process.env[name])) fail(`${name} is not set to an existing folder`);
if (!debug && !existsSync(process.env.GNOMCODE_ANDROID_SIGNING || '')) fail('GNOMCODE_ANDROID_SIGNING must point to the signing properties file for a release build');

writeFileSync(path.join(android, 'local.properties'), `sdk.dir=${process.env.ANDROID_HOME.replaceAll('\\', '/')}\n`);
if (process.env.GNOMCODE_GOOGLE_SERVICES) copyFileSync(process.env.GNOMCODE_GOOGLE_SERVICES, path.join(android, 'app', 'google-services.json'));

const shell = process.platform === 'win32';
execFileSync('npx', ['cap', 'sync', 'android'], { cwd: root, stdio: 'inherit', shell });
execFileSync(shell ? path.join(android, 'gradlew.bat') : './gradlew', [debug ? 'assembleDebug' : 'assembleRelease', '-q'], { cwd: android, stdio: 'inherit', shell });

const kind = debug ? 'debug' : 'release';
const built = path.join(android, 'app', 'build', 'outputs', 'apk', kind, `app-${kind}.apk`);
if (!existsSync(built)) fail(`expected ${built}`);
mkdirSync(path.join(root, 'dist'), { recursive: true });
const out = path.join(root, 'dist', `gnomcode-android-${version}${debug ? '-debug' : ''}.apk`);
copyFileSync(built, out);
console.log(`[gnomcode:android] ${out}`);
