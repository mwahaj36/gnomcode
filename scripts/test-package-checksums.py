#!/usr/bin/env python3
"""Release checksum files must work with Unix filename/checksum parsers."""
import hashlib
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ChecksumPackagingTest(unittest.TestCase):
    def test_manifests_use_lf_even_when_packaged_on_windows(self):
        with tempfile.TemporaryDirectory(prefix='gnomcode-checksums-') as temporary:
            root = Path(temporary)
            (root / 'scripts').mkdir()
            shutil.copyfile(Path(__file__).with_name('package-artifacts.py'), root / 'scripts/package-artifacts.py')
            (root / 'Cargo.toml').write_text('[workspace.package]\nversion = "1.1.0"\n')
            (root / 'LICENSE').write_text('Synthetic test fixture license')
            output = root / 'web/public/downloads'
            output.mkdir(parents=True)
            (output / 'THIRD_PARTY_NOTICES.html').write_text('Synthetic test notice. ' * 100)
            binary = root / 'fixture.exe'
            setup = root / 'fixture-setup.exe'
            binary.write_bytes(b'MZ synthetic test binary; never shipped')
            setup.write_bytes(b'MZ synthetic test installer; never shipped')
            for name in ['gnomcode.ico', 'gnomcode-tray.ico', 'tray.ps1', 'run-tray.vbs', 'open-console.vbs', 'open-dashboard.vbs', 'open-folder.ps1']:
                (root / 'scripts' / name).write_bytes(b'Synthetic test resource')
            result = subprocess.run([sys.executable, str(root / 'scripts/package-artifacts.py'), '--windows-exe', str(binary), '--windows-setup', str(setup)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            assets = sorted([output / 'gnomcode.exe', output / 'Gnomcode-Setup.exe', output / 'Gnomcode-v1.1.0-windows-x64.zip'])
            expected = ''.join(hashlib.sha256(path.read_bytes()).hexdigest() + '  ' + path.name + '\n' for path in assets).encode()
            for name in ['SHA256SUMS', 'SHA256SUMS-v1.1.0.txt']:
                self.assertEqual((output / name).read_bytes(), expected)


if __name__ == '__main__':
    unittest.main()
