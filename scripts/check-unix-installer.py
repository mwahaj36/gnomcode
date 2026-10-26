#!/usr/bin/env python3
"""Exercise the real Unix updater against local release assets and a corrupt manifest."""
import hashlib
import http.server
import os
import subprocess
import tempfile
import threading
import tomllib
from pathlib import Path

root = Path(__file__).resolve().parent.parent
assets = root / 'web/public/downloads'
version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']

class Handler(http.server.SimpleHTTPRequestHandler):
    corrupt = False
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(assets), **kwargs)
    def log_message(self, *args):
        pass
    def do_GET(self):
        if self.corrupt and self.path.startswith('/SHA256SUMS-'):
            payload = b'0' * 64 + f'  fleethud-v{version}-linux-x86_64.tar.gz\n'.encode()
            self.send_response(200)
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        else:
            super().do_GET()

server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
try:
    with tempfile.TemporaryDirectory(prefix='fleethud-updater-check-') as temp:
        stage = Path(temp)
        install_dir = stage / 'bin'
        install_dir.mkdir()
        script = stage / 'install.sh'
        script.write_text((root / 'scripts/install.sh').read_text().replace(
            'https://fleethud.vercel.app/downloads', f'http://127.0.0.1:{server.server_port}'))
        env = dict(os.environ, FLEETHUD_INSTALL_DIR=str(install_dir))
        subprocess.run(['bash', str(script)], env=env, check=True, capture_output=True, timeout=30)
        binary = install_dir / 'fleethud'
        assert subprocess.check_output([str(binary), '--version'], text=True).strip() == f'fleethud {version}'
        assert (install_dir / 'fleethud-licenses/THIRD_PARTY_NOTICES.html').is_file()
        before = hashlib.sha256(binary.read_bytes()).digest()
        Handler.corrupt = True
        rejected = subprocess.run(['bash', str(script)], env=env, capture_output=True, timeout=30)
        assert rejected.returncode != 0
        assert b'checksum verification failed' in rejected.stderr
        assert hashlib.sha256(binary.read_bytes()).digest() == before
        assert not list(install_dir.glob('.fleethud-update.*'))
        print('PASS: verified Unix installation; corrupt manifest rejected; existing install preserved')
finally:
    server.shutdown()
    server.server_close()
