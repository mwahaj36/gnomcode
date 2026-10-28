// Compile the actual embedded dashboard script before packaging it into a binary.
import { readFileSync } from 'node:fs';
import { Script } from 'node:vm';
for (const file of ['theme-init.js', 'console-app.js']) {
  const script = readFileSync(new URL('../crates/daemon/src/' + file, import.meta.url), 'utf8');
  new Script(script, { filename: file });
}
console.log('PASS: embedded console JavaScript compiles');
