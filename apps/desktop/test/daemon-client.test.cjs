const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { readDaemon } = require('../dist/daemon-client.js');

test('discovers only a healthy daemon with valid local metadata', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'spawnt-daemon-test-'));
  const token = 'a'.repeat(64);
  const server = http.createServer((request, response) => {
    response.writeHead(request.url === '/health' && request.headers.authorization === `Bearer ${token}` ? 200 : 401);
    response.end();
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  try {
    const port = server.address().port;
    await fs.writeFile(path.join(dir, 'daemon.json'), JSON.stringify({port, token}));
    assert.equal(await readDaemon(dir), `http://127.0.0.1:${port}/?token=${token}`);
    await fs.writeFile(path.join(dir, 'daemon.json'), JSON.stringify({port, token: 'b'.repeat(64)}));
    assert.equal(await readDaemon(dir), null);
    await fs.writeFile(path.join(dir, 'daemon.json'), '{malformed');
    assert.equal(await readDaemon(dir), null);
  } finally {
    await new Promise(resolve => server.close(resolve));
    await fs.rm(dir, {recursive: true, force: true});
  }
});
