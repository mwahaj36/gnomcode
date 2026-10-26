#!/usr/bin/env node

/**
 * FleetHUD Version Synchronizer & Release Bumper
 * Usage:
 *   node .agents/skills/cut-release/scripts/bump-version.mjs <version | patch | minor | major>
 * Examples:
 *   node .agents/skills/cut-release/scripts/bump-version.mjs 1.1.2
 *   node .agents/skills/cut-release/scripts/bump-version.mjs patch
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = path.resolve(__dirname, "../../../../");

function read(relPath) {
  const fullPath = path.join(ROOT, relPath);
  return fs.existsSync(fullPath) ? fs.readFileSync(fullPath, "utf-8") : null;
}

function write(relPath, content) {
  const fullPath = path.join(ROOT, relPath);
  fs.writeFileSync(fullPath, content, "utf-8");
}

function getCurrentVersion() {
  const setupCs = read("scripts/Setup.cs");
  if (setupCs) {
    const match = setupCs.match(/AssemblyVersion\("(\d+\.\d+\.\d+)\.?\d*"\)/);
    if (match) return match[1];
  }
  const cargoToml = read("Cargo.toml");
  if (cargoToml) {
    const match = cargoToml.match(/\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"/);
    if (match && match[1] !== "0.0.1") return match[1];
  }
  const webPkg = read("web/package.json");
  if (webPkg) {
    try {
      const parsed = JSON.parse(webPkg);
      if (parsed.version) return parsed.version;
    } catch {}
  }
  return "1.1.2";
}

function calculateNextVersion(current, bumpType) {
  const parts = current.split(".").map((n) => parseInt(n, 10));
  while (parts.length < 3) parts.push(0);

  if (bumpType === "major") {
    return `${parts[0] + 1}.0.0`;
  } else if (bumpType === "minor") {
    return `${parts[0]}.${parts[1] + 1}.0`;
  } else if (bumpType === "patch") {
    return `${parts[0]}.${parts[1]}.${parts[2] + 1}`;
  }
  return bumpType.replace(/^v/, "").trim();
}

const inputArg = process.argv[2];
if (!inputArg) {
  console.error("Usage: node bump-version.mjs <version | patch | minor | major>");
  process.exit(1);
}

const currentVersion = getCurrentVersion();
const newVersion = calculateNextVersion(currentVersion, inputArg);

if (!/^\d+\.\d+\.\d+$/.test(newVersion)) {
  console.error(`Error: Invalid SemVer version: "${newVersion}". Expected format X.Y.Z`);
  process.exit(1);
}

console.log(`[fleethud:release] Updating version: ${currentVersion} -> ${newVersion}\n`);

const results = [];

// 1. Cargo.toml
const cargoPath = "Cargo.toml";
let cargo = read(cargoPath);
if (cargo) {
  cargo = cargo.replace(
    /(\[workspace\.package\][\s\S]*?version\s*=\s*")[^"]+(")/,
    `$1${newVersion}$2`
  );
  write(cargoPath, cargo);
  results.push({ file: cargoPath, detail: `[workspace.package] version = "${newVersion}"` });
}

// 2. scripts/Setup.cs
const setupPath = "scripts/Setup.cs";
let setup = read(setupPath);
if (setup) {
  setup = setup
    .replace(/AssemblyVersion\("[^"]+"\)/g, `AssemblyVersion("${newVersion}.0")`)
    .replace(/AssemblyFileVersion\("[^"]+"\)/g, `AssemblyFileVersion("${newVersion}.0")`)
    .replace(/lblVer\.Text = "v[^ ]+  ·  64-bit Native";/, `lblVer.Text = "v${newVersion}  ·  64-bit Native";`)
    .replace(/key\.SetValue\("DisplayVersion", "[^"]+"\);/, `key.SetValue("DisplayVersion", "${newVersion}");`);
  write(setupPath, setup);
  results.push({ file: setupPath, detail: `Assembly & GUI DisplayVersion -> "${newVersion}"` });
}

// 3. web/package.json
const webPkgPath = "web/package.json";
let webPkg = read(webPkgPath);
if (webPkg) {
  webPkg = webPkg.replace(/("version":\s*")[^"]+(")/, `$1${newVersion}$2`);
  write(webPkgPath, webPkg);
  results.push({ file: webPkgPath, detail: `"version": "${newVersion}"` });
}

// 4. npm/package.json
const npmPkgPath = "npm/package.json";
let npmPkg = read(npmPkgPath);
if (npmPkg) {
  npmPkg = npmPkg.replace(/("version":\s*")[^"]+(")/, `$1${newVersion}$2`);
  write(npmPkgPath, npmPkg);
  results.push({ file: npmPkgPath, detail: `"version": "${newVersion}"` });
}

// 5. npm/fleethud/package.json
const npmFleetPath = "npm/fleethud/package.json";
let npmFleet = read(npmFleetPath);
if (npmFleet) {
  npmFleet = npmFleet.replace(/("version":\s*")[^"]+(")/, `$1${newVersion}$2`);
  write(npmFleetPath, npmFleet);
  results.push({ file: npmFleetPath, detail: `"version": "${newVersion}"` });
}

// 6. scripts/install.ps1
const installPs1Path = "scripts/install.ps1";
let installPs1 = read(installPs1Path);
if (installPs1) {
  installPs1 = installPs1
    .replace(/(\$Version\s*=\s*")[^"]+(";?)/, `$1${newVersion}$2`)
    .replace(/FleetHUD-v[0-9.]+-windows-(?:\$Arch|x64)\.zip/g, `FleetHUD-v${newVersion}-windows-x64.zip`)
    .replace(/FleetHUD v[0-9.]+ installation complete/g, `FleetHUD v${newVersion} installation complete`);
  write(installPs1Path, installPs1);
  results.push({ file: installPs1Path, detail: `Archive URL & Banner -> v${newVersion}` });

  // Mirror to web/public/downloads/install.ps1
  write("web/public/downloads/install.ps1", installPs1);
  results.push({ file: "web/public/downloads/install.ps1", detail: `Synced from scripts/install.ps1` });
}

// Unix installer version and public mirror.
let unixInstaller = read("scripts/install.sh");
if (unixInstaller) {
  unixInstaller = unixInstaller.replace(/VERSION="[^"]+"/, `VERSION="${newVersion}"`);
  write("scripts/install.sh", unixInstaller);
  write("web/public/downloads/install.sh", unixInstaller);
  results.push({ file: "scripts/install.sh + public mirror", detail: `Verified archive version -> v${newVersion}` });
}

// Keep the public landing badge and npm lock root metadata synchronized.
let landing = read("web/app/page.tsx");
if (landing) write("web/app/page.tsx", landing.replace(/FleetHUD v[0-9.]+ Production Release/g, `FleetHUD v${newVersion} Production Release`));
let lockText = read("web/package-lock.json");
if (lockText) {
  const lock = JSON.parse(lockText);
  lock.version = newVersion;
  if (lock.packages?.[""]) lock.packages[""].version = newVersion;
  write("web/package-lock.json", JSON.stringify(lock, null, 2) + "\n");
}

// 7. web/app/downloads/page.tsx
const downloadsPagePath = "web/app/downloads/page.tsx";
let downloadsPage = read(downloadsPagePath);
if (downloadsPage) {
  downloadsPage = downloadsPage
    .replace(/Production Release &bull; v[0-9.]+/g, `Production Release &bull; v${newVersion}`)
    .replace(/\/downloads\/FleetHUD-v[0-9.]+-windows-x64\.zip/g, `/downloads/FleetHUD-v${newVersion}-windows-x64.zip`);
  write(downloadsPagePath, downloadsPage);
  results.push({ file: downloadsPagePath, detail: `UI Badge & Download Link -> v${newVersion}` });
}

// 8. Tauri config if present
const tauriConfigPath = "apps/tray/src-tauri/tauri.conf.json";
let tauriConfig = read(tauriConfigPath);
if (tauriConfig) {
  tauriConfig = tauriConfig.replace(/("version":\s*")[^"]+(")/, `$1${newVersion}$2`);
  write(tauriConfigPath, tauriConfig);
  results.push({ file: tauriConfigPath, detail: `"version": "${newVersion}"` });
}

console.log("Touched files:");
for (const res of results) {
  console.log(`  - ${res.file.padEnd(35)} : ${res.detail}`);
}

console.log(`\n[fleethud:release] Version successfully synchronized across all ${results.length} files to v${newVersion}`);
