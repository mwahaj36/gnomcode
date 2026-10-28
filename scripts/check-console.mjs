// Compile the actual embedded dashboard script before packaging it into a binary.
import { readFileSync } from 'node:fs';
import { Script } from 'node:vm';
const html = readFileSync(new URL('../crates/daemon/src/console.html', import.meta.url), 'utf8');
for (const [, script] of html.matchAll(/<script>([\s\S]*?)<\/script>/g)) {
  new Script(script, { filename: 'embedded-console.js' });
}
console.log('PASS: embedded console JavaScript compiles');
