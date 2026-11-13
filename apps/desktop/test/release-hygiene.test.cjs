'use strict';

const assert = require('node:assert/strict');
const { existsSync, readdirSync, readFileSync } = require('node:fs');
const path = require('node:path');
const test = require('node:test');

const desktopDir = path.resolve(__dirname, '..');

test('desktop TypeScript and release packaging disable sourcemaps and DevTools', () => {
  const tsconfig = JSON.parse(readFileSync(path.join(desktopDir, 'tsconfig.json'), 'utf8'));
  assert.equal(tsconfig.compilerOptions.sourceMap, false);
  assert.equal(tsconfig.compilerOptions.inlineSourceMap, false);
  assert.equal(tsconfig.compilerOptions.declarationMap, false);

  const mainSource = readFileSync(path.join(desktopDir, 'src', 'main.ts'), 'utf8');
  assert.match(mainSource, /devTools:\s*!app\.isPackaged/);

  const distDir = path.join(desktopDir, 'dist');
  if (existsSync(distDir)) {
    const distFiles = readdirSync(distDir);
    for (const file of distFiles) {
      assert.ok(!file.endsWith('.map'), `unexpected sourcemap file in dist: ${file}`);
      if (file.endsWith('.js')) {
        const content = readFileSync(path.join(distDir, file), 'utf8');
        assert.doesNotMatch(content, /sourceMappingURL/i, `${file} must not include sourceMappingURL`);
      }
    }
  }
});
