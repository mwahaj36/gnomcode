import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

process.env.NODE_ENV = 'production';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const { webpack } = require(path.join(root, 'web/node_modules/next/dist/compiled/webpack/bundle5'))();
const terser = require(path.join(root, 'web/node_modules/next/dist/compiled/terser'));

const entry = path.join(root, 'web/local-console/main.tsx');
const outDir = path.join(root, 'crates/daemon/src');
const outFile = 'console-react.js';
const outPath = path.join(outDir, outFile);
const loaderPath = path.join(root, 'scripts/local-console-loader.cjs');

const config = {
  context: root,
  mode: 'production',
  devtool: false,
  entry,
  plugins: [
    new webpack.DefinePlugin({
      'process.env.NODE_ENV': JSON.stringify('production'),
    }),
  ],
  output: {
    path: outDir,
    filename: outFile,
    charset: true,
  },
  resolve: {
    extensions: ['.tsx', '.ts', '.mjs', '.js', '.json'],
    modules: [path.join(root, 'web/node_modules'), 'node_modules'],
  },
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        exclude: /node_modules/,
        use: [{ loader: loaderPath }],
      },
    ],
  },
  optimization: {
    nodeEnv: 'production',
    minimize: false,
    moduleIds: 'deterministic',
    chunkIds: 'deterministic',
    splitChunks: false,
    runtimeChunk: false,
  },
};

const compiler = webpack(config);
const memfs = new Map();
compiler.outputFileSystem = {
  mkdir(_p, optsOrCb, maybeCb) {
    const cb = typeof optsOrCb === 'function' ? optsOrCb : maybeCb;
    cb?.(null);
  },
  writeFile(filePath, data, optsOrCb, maybeCb) {
    const cb = typeof optsOrCb === 'function' ? optsOrCb : maybeCb;
    memfs.set(path.resolve(filePath), Buffer.isBuffer(data) ? data.toString('utf8') : String(data));
    cb?.(null);
  },
  stat(filePath, cb) {
    if (memfs.has(path.resolve(filePath))) cb(null, { isFile: () => true, isDirectory: () => false });
    else cb(Object.assign(new Error('ENOENT'), { code: 'ENOENT' }));
  },
  readFile(filePath, cb) {
    const v = memfs.get(path.resolve(filePath));
    if (v !== undefined) cb(null, Buffer.from(v, 'utf8'));
    else cb(Object.assign(new Error('ENOENT'), { code: 'ENOENT' }));
  },
  join: path.join,
};

await new Promise((resolve, reject) => {
  compiler.run((err, stats) => {
    if (err) return reject(err);
    const info = stats.toJson({ errors: true, warnings: false });
    if (stats.hasErrors()) {
      return reject(new Error(info.errors.map(e => e.message).join('\n')));
    }
    compiler.close(closeErr => (closeErr ? reject(closeErr) : resolve()));
  });
});

const raw = memfs.get(path.resolve(outPath));
assert.ok(raw && raw.length > 0, 'Compiled console-react.js must not be empty');
const minified = await terser.minify(raw, {
  compress: true,
  mangle: true,
  sourceMap: false,
  format: { ascii_only: true, comments: false },
});
assert.ok(minified.code && minified.code.length > 0, 'Minified console-react.js must not be empty');
const normalized = minified.code.replace(/\r\n/g, '\n').replace(/^\s*\/\/# sourceMappingURL=.*$/gm, '').trimEnd() + '\n';
assert.doesNotMatch(normalized, /sourceMappingURL/i, 'Built bundle must never contain sourceMappingURL');

if (process.argv.includes('--check')) {
  const existing = (await readFile(outPath, 'utf8')).replace(/\r\n/g, '\n');
  assert.equal(existing, normalized, `Stale crates/daemon/src/${outFile}; run node scripts/build-local-console.mjs`);
  console.log(`PASS: local console React bundle matches crates/daemon/src/${outFile}`);
} else {
  await writeFile(outPath, normalized, 'utf8');
  console.log(`WROTE crates/daemon/src/${outFile} (${normalized.length} bytes)`);
}
