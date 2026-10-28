"""Check packaging with synthetic executable headers in an isolated tree."""
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
root=Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='spawnt-package-check-') as tmp:
    tree=Path(tmp)
    for name in ['scripts/package-artifacts.py','Cargo.toml','LICENSE','web/public/icons/spawnt.icns','web/public/icons/spawnt-256.png','web/public/fonts/Inter-LICENSE.txt']:
        target=tree/name; target.parent.mkdir(parents=True,exist_ok=True); target.write_bytes((root/name).read_bytes())
    out=tree/'web/public/downloads'; out.mkdir(parents=True)
    (out/'THIRD_PARTY_NOTICES.html').write_text('test notice '*150)
    binary=tree/'engine'; header=bytearray(64); header[:4]=b'\xcf\xfa\xed\xfe'; header[4:8]=(0x100000c).to_bytes(4,'little'); binary.write_bytes(header)
    def run(argument): return subprocess.run([sys.executable,str(tree/'scripts/package-artifacts.py'),argument,str(binary)],capture_output=True,text=True)
    assert run('--linux-x86-64').returncode != 0
    result=run('--darwin-aarch64'); assert result.returncode==0,result.stderr
    archive=next(out.glob('*darwin-aarch64.tar.gz'))
    with tarfile.open(archive) as bundle:
        names=bundle.getnames()
        assert 'spawnt.app/Contents/Info.plist' in names
        assert 'spawnt.app/Contents/Resources/spawnt.icns' in names
        assert bundle.getmember('spawnt.app/Contents/MacOS/fleethud').mode & 0o111
        assert 'fonts/Inter-LICENSE.txt' in names
    header[:4]=b'\x7fELF'; header[18:20]=(62).to_bytes(2,'little'); binary.write_bytes(header)
    assert run('--linux-x86-64').returncode==0
    manifest=(out/'SHA256SUMS').read_text()
    assert 'darwin-aarch64.tar.gz' in manifest and 'linux-x86_64.tar.gz' in manifest
print('PASS: architecture validation, executable launcher modes, canonical icons, font notices and multi-target checksums')
