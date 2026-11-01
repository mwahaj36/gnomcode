"""Measure a Linux debug daemon and three idle synthetic PTYs in an isolated home."""

import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

if not os.path.isdir('/proc/self'):
    raise SystemExit('Linux /proc is required')

binary = str(Path(__file__).resolve().parents[1] / 'target/debug/spawnt')


def sample(pids, seconds=4):
    def read(pid):
        try:
            with open(f'/proc/{pid}/stat') as source:
                fields = source.read().rsplit(')', 1)[1].split()
            with open(f'/proc/{pid}/status') as source:
                rss = next(int(line.split()[1]) for line in source if line.startswith('VmRSS:'))
            return int(fields[11]) + int(fields[12]), rss
        except (OSError, StopIteration, ValueError):
            return None

    before = {pid: read(pid) for pid in pids}
    start = time.monotonic()
    time.sleep(seconds)
    elapsed = time.monotonic() - start
    after = {pid: read(pid) for pid in pids}
    if any(before[pid] is None or after[pid] is None for pid in pids):
        raise RuntimeError('a measured process exited or could not be inspected')
    ticks_per_second = os.sysconf(os.sysconf_names['SC_CLK_TCK'])
    return [
        {'pid': pid, 'rss_kib': after[pid][1],
         'cpu_pct': round((after[pid][0] - before[pid][0]) / ticks_per_second / elapsed * 100, 2)}
        for pid in pids if before[pid] and after[pid]
    ]


with tempfile.TemporaryDirectory(prefix='spawnt-bench-') as directory:
    root = Path(directory)
    env = {
        'SPAWNT_HOME': str(root / 'state'), 'HOME': directory, 'USERPROFILE': directory,
        'APPDATA': directory, 'CLAUDE_CONFIG_DIR': str(root / 'claude'),
        'CODEX_HOME': str(root / 'codex'), 'GEMINI_HOME': str(root / 'gemini'),
        'PATH': f'{root / "bin"}:/usr/bin:/bin',
    }
    (root / 'bin').mkdir()
    (root / 'repo').mkdir()
    fake_agent = root / 'bin' / 'codex'
    fake_agent.write_text('#!/bin/sh\nexec /bin/sleep 90\n')
    fake_agent.chmod(0o755)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    daemon = subprocess.Popen(
        [binary, 'daemon', '--port', str(port)], env=env,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    sessions = []
    try:
        for _ in range(100):
            if (root / 'state' / 'daemon.json').exists():
                break
            if daemon.poll() is not None:
                raise RuntimeError('daemon exited during startup')
            time.sleep(0.1)
        else:
            raise RuntimeError('daemon did not become ready')
        time.sleep(2)
        print('idle', json.dumps(sample([daemon.pid])))
        for _ in range(3):
            result = subprocess.run(
                [binary, 'launch', 'codex', '--repo', str(root / 'repo')],
                env=env, capture_output=True, text=True, timeout=12,
            )
            if result.returncode:
                raise RuntimeError('synthetic launch failed: ' + result.stderr[-500:])
            sessions.append(next(line.split(':', 1)[1].strip() for line in result.stdout.splitlines()
                                 if line.startswith('Session ID:')))
            time.sleep(1.1)
        records = [json.loads((root / 'state' / 'sessions' / f'{sid}.json').read_text())
                   for sid in sessions]
        pids = [daemon.pid] + [int(record[key]) for record in records for key in ('host_pid', 'child_pid')]
        print('three_synthetic_sessions', json.dumps(sample(pids)))
    finally:
        session_dir = root / 'state' / 'sessions'
        for record in session_dir.glob('*.json') if session_dir.exists() else []:
            try:
                subprocess.run([binary, 'stop', record.stem], env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=8)
            except subprocess.TimeoutExpired:
                pass
        daemon.terminate()
        try:
            daemon.wait(timeout=5)
        except subprocess.TimeoutExpired:
            daemon.kill()
            daemon.wait()
