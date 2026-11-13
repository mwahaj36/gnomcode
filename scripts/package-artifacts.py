#!/usr/bin/env python3
"""Package built release artifacts and notices; never relabel an old binary."""
import argparse
import hashlib
import shutil
import subprocess
import tarfile
import re
import zipfile
import plistlib
import tempfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--windows-exe', type=Path)
parser.add_argument('--windows-setup', type=Path)
parser.add_argument('--linux-x86-64', type=Path)
parser.add_argument('--linux-aarch64', type=Path)
parser.add_argument('--darwin-x86-64', type=Path)
parser.add_argument('--darwin-aarch64', type=Path)
args = parser.parse_args()
version = re.search(r'\[workspace\.package\][\s\S]*?\nversion\s*=\s*"([^"]+)"', (root / 'Cargo.toml').read_text()).group(1)
out = root / 'web/public/downloads'
notices = out / 'THIRD_PARTY_NOTICES.html'
if not notices.is_file() or notices.stat().st_size < 1000:
    raise SystemExit('Generate complete third-party notices before packaging')
shutil.copyfile(root / 'docs/legal/EULA.txt', out / 'LICENSE')
assets = []
if bool(args.windows_exe) != bool(args.windows_setup):
    raise SystemExit('Supply both the Windows binary and matching compiled installer')
if args.windows_exe:
    for path in (args.windows_exe, args.windows_setup):
        if path.read_bytes()[:2] != b'MZ':
            raise SystemExit(f'Not a Windows executable: {path}')
    for source, name in [(args.windows_exe, 'gnomcode.exe'), (args.windows_setup, 'Gnomcode-Setup.exe')]:
        if source.resolve() != (out / name).resolve():
            shutil.copyfile(source, out / name)
        assets.append(out / name)
    archive = out / f'Gnomcode-v{version}-windows-x64.zip'
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
        for name in ('gnomcode.exe', 'Gnomcode-Setup.exe', 'LICENSE', 'THIRD_PARTY_NOTICES.html'):
            bundle.write(out / name, name)
        for name in ('gnomcode.ico', 'gnomcode-tray.ico', 'tray.ps1', 'run-tray.vbs', 'open-console.vbs', 'open-dashboard.vbs', 'open-folder.ps1'):
            bundle.write(root / 'scripts' / name, name)
    assets.append(archive)
for os_name, arch, source in [
    ('linux', 'x86_64', args.linux_x86_64), ('linux', 'aarch64', args.linux_aarch64),
    ('darwin', 'x86_64', args.darwin_x86_64), ('darwin', 'aarch64', args.darwin_aarch64),
]:
    if source is None:
        continue
    header = source.read_bytes()[:20]
    if os_name == 'linux':
        valid = header[:4] == b'\x7fELF' and int.from_bytes(header[18:20], 'little') == (62 if arch == 'x86_64' else 183)
    else:
        valid = header[:4] == b'\xcf\xfa\xed\xfe' and int.from_bytes(header[4:8], 'little') == (0x1000007 if arch == 'x86_64' else 0x100000c)
    if not valid:
        raise SystemExit(f'Not a matching {os_name} {arch} executable: {source}')
    archive = out / f'gnomcode-v{version}-{os_name}-{arch}.tar.gz'
    with tempfile.TemporaryDirectory(prefix='gnomcode-package-') as temp:
        stage = Path(temp)
        shutil.copyfile(source, stage / 'gnomcode')
        (stage / 'gnomcode').chmod(0o755)
        shutil.copyfile(root / 'docs/legal/EULA.txt', stage / 'LICENSE')
        shutil.copyfile(notices, stage / 'THIRD_PARTY_NOTICES.html')
        shutil.copytree(root / 'web/public/fonts', stage / 'fonts')
        shutil.copyfile(root / 'web/public/icons/gnomcode-256.png', stage / 'gnomcode.png')
        launcher = stage / 'Open gnomcode.command'
        launcher.write_text('#!/bin/sh\ncd "$(dirname "$0")" || exit 1\nexec ./gnomcode console\n')
        launcher.chmod(0o755)
        if os_name == 'darwin':
            contents = stage / 'gnomcode.app/Contents'
            (contents / 'MacOS').mkdir(parents=True)
            (contents / 'Resources').mkdir()
            shutil.copyfile(stage / 'gnomcode', contents / 'MacOS/gnomcode')
            (contents / 'MacOS/gnomcode').chmod(0o755)
            entry = contents / 'MacOS/gnomcode'
            entry.write_text('#!/bin/sh\nexec "$(dirname "$0")/gnomcode" console\n')
            entry.chmod(0o755)
            shutil.copyfile(root / 'web/public/icons/gnomcode.icns', contents / 'Resources/gnomcode.icns')
            (contents / 'Info.plist').write_bytes(plistlib.dumps({
                'CFBundleName':'gnomcode', 'CFBundleDisplayName':'gnomcode', 'CFBundleIdentifier':'app.gnomcode.console',
                'CFBundleExecutable':'gnomcode', 'CFBundlePackageType':'APPL', 'CFBundleIconFile':'gnomcode.icns',
                'CFBundleShortVersionString':version, 'CFBundleVersion':version, 'LSMinimumSystemVersion':'12.0',
            }))
        def archive_mode(info):
            if info.isfile():
                info.mode = 0o755 if info.name.endswith(('/MacOS/gnomcode', '/gnomcode', '.command')) or info.name == 'gnomcode' else 0o644
            return info
        with tarfile.open(archive, 'w:gz') as bundle:
            for entry in sorted(stage.iterdir()):
                bundle.add(entry, arcname=entry.name, filter=archive_mode)
    assets.append(archive)
if not assets:
    raise SystemExit('No built binaries supplied')
# Preserve checksums from other targets built in the same release.
assets = list({path.name: path for path in [*assets, *out.glob(f'gnomcode-v{version}-*.tar.gz'), *out.glob(f'Gnomcode-v{version}-*.zip')]}.values())
manifest = ''.join(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n' for path in sorted(assets))
(out / f'SHA256SUMS-v{version}.txt').write_text(manifest)
(out / 'SHA256SUMS').write_text(manifest)
for path in assets:
    print(f'{path.name}: {path.stat().st_size} bytes')
