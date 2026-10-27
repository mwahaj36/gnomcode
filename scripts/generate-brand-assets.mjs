// Generate every platform mark from the approved SVG. Run after npm ci --prefix web.
import { createRequire } from 'node:module';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
const require = createRequire(new URL('../web/package.json', import.meta.url));
const sharp = require('sharp');
const root = new URL('../', import.meta.url);
const source = await readFile(new URL('web/public/spawnt-mark.svg', root));
const directory = new URL('web/public/icons/', root);
await mkdir(directory, { recursive: true });
const sizes = [16, 24, 32, 48, 64, 128, 256, 512, 1024];
const images = new Map();
for (const size of sizes) {
  const png = await sharp(source).resize(size, size).png().toBuffer();
  images.set(size, png);
  await writeFile(new URL(`spawnt-${size}.png`, directory), png);
}
const iconSizes = sizes.filter(size => size <= 256);
const header = Buffer.alloc(6 + iconSizes.length * 16);
header.writeUInt16LE(1, 2);
header.writeUInt16LE(iconSizes.length, 4);
let offset = header.length;
for (const [index, size] of iconSizes.entries()) {
  const entry = 6 + index * 16;
  header[entry] = header[entry + 1] = size === 256 ? 0 : size;
  header.writeUInt16LE(1, entry + 4);
  header.writeUInt16LE(32, entry + 6);
  header.writeUInt32LE(images.get(size).length, entry + 8);
  header.writeUInt32LE(offset, entry + 12);
  offset += images.get(size).length;
}
const ico = Buffer.concat([header, ...iconSizes.map(size => images.get(size))]);
for (const path of ['scripts/fleethud.ico', 'web/app/favicon.ico', 'web/public/downloads/fleethud.ico']) {
  await writeFile(new URL(path, root), ico);
}
await writeFile(new URL('web/app/apple-icon.png', root), await sharp(source).resize(180, 180).flatten({ background: '#070708' }).png().toBuffer());
// Modern macOS icon family: 512 and 1024 px PNG representations.
const chunks = [ [512, 'ic09'], [1024, 'ic10'] ].map(([size, type]) => {
  const chunk = Buffer.alloc(8);
  chunk.write(type);
  chunk.writeUInt32BE(images.get(size).length + 8, 4);
  return Buffer.concat([chunk, images.get(size)]);
});
const icns = Buffer.alloc(8);
icns.write('icns');
icns.writeUInt32BE(8 + chunks.reduce((sum, chunk) => sum + chunk.length, 0), 4);
await writeFile(new URL('spawnt.icns', directory), Buffer.concat([icns, ...chunks]));
console.log('Generated Windows, macOS, Linux, favicon and Apple assets from spawnt-mark.svg');
