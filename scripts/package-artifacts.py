#!/usr/bin/env python3
"""Package built release artifacts and notices; never relabel an old binary."""
import argparse
import hashlib
import shutil
import subprocess
import tarfile
import tomllib
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--windows-exe', type=Path)
parser.add_argument('--windows-setup', type=Path)
parser.add_argument('--linux-x86-64', type=Path)
parser.add_argument('--linux-aarch64', type=Path)
args = parser.parse_args()
version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
subprocess.run(['node', str(root / 'scripts/check-console.mjs')], check=True)
out = root / 'web/public/downloads'
notices = out / 'THIRD_PARTY_NOTICES.html'
if not notices.is_file() or notices.stat().st_size < 1000:
    raise SystemExit('Generate complete third-party notices before packaging')
shutil.copyfile(root / 'LICENSE', out / 'LICENSE')
assets = []
if bool(args.windows_exe) != bool(args.windows_setup):
    raise SystemExit('Supply both the Windows binary and matching compiled installer')
if args.windows_exe:
    for path in (args.windows_exe, args.windows_setup):
        if path.read_bytes()[:2] != b'MZ':
            raise SystemExit(f'Not a Windows executable: {path}')
    for source, name in [(args.windows_exe, 'fleethud.exe'), (args.windows_setup, 'FleetHUD-Setup.exe')]:
        if source.resolve() != (out / name).resolve():
            shutil.copyfile(source, out / name)
        assets.append(out / name)
    archive = out / f'FleetHUD-v{version}-windows-x64.zip'
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
        for name in ('fleethud.exe', 'FleetHUD-Setup.exe', 'LICENSE', 'THIRD_PARTY_NOTICES.html'):
            bundle.write(out / name, name)
        for name in ('fleethud.ico', 'tray.ps1', 'run-tray.vbs', 'open-console.vbs', 'open-dashboard.vbs'):
            bundle.write(root / 'scripts' / name, name)
    assets.append(archive)
for arch, source in [('x86_64', args.linux_x86_64), ('aarch64', args.linux_aarch64)]:
    if source is None:
        continue
    header = source.read_bytes()[:20]
    machine = 62 if arch == 'x86_64' else 183
    if header[:4] != b'\x7fELF' or int.from_bytes(header[18:20], 'little') != machine:
        raise SystemExit(f'Not a matching Linux {arch} executable: {source}')
    archive = out / f'fleethud-v{version}-linux-{arch}.tar.gz'
    with tarfile.open(archive, 'w:gz') as bundle:
        bundle.add(source, arcname='fleethud')
        bundle.add(root / 'LICENSE', arcname='LICENSE')
        bundle.add(notices, arcname='THIRD_PARTY_NOTICES.html')
    assets.append(archive)
if not assets:
    raise SystemExit('No built binaries supplied')
manifest = ''.join(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n' for path in sorted(assets))
(out / f'SHA256SUMS-v{version}.txt').write_text(manifest)
(out / 'SHA256SUMS').write_text(manifest)
for path in assets:
    print(f'{path.name}: {path.stat().st_size} bytes')
