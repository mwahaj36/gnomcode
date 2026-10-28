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
// Legacy Windows tray/GDI+ cannot draw PNG-compressed small ICO frames.
// Keep the large frame compressed, and encode small frames as Windows DIBs.
async function windowsIcon(scale) {
const frames = new Map();
for (const size of iconSizes) {
  const markSize = Math.round(size * scale);
  const inset = Math.floor((size - markSize) / 2);
  const windowsImage = await sharp(images.get(size)).resize(markSize, markSize).extend({
    top: inset, left: inset, bottom: size - markSize - inset, right: size - markSize - inset,
    background: '#070708'
  }).flatten({ background: '#070708' }).png().toBuffer();
  if (size === 256) { frames.set(size, await sharp(windowsImage).png().toBuffer()); continue; }
  const pixels = await sharp(windowsImage).ensureAlpha().raw().toBuffer();
  const maskStride = Math.ceil(size / 32) * 4;
  const dib = Buffer.alloc(40 + size * size * 4 + maskStride * size);
  dib.writeUInt32LE(40, 0); dib.writeInt32LE(size, 4); dib.writeInt32LE(size * 2, 8);
  dib.writeUInt16LE(1, 12); dib.writeUInt16LE(32, 14);
  dib.writeUInt32LE(size * size * 4, 20);
  for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
    const input = (y * size + x) * 4, output = 40 + ((size - 1 - y) * size + x) * 4;
    dib[output] = pixels[input + 2]; dib[output + 1] = pixels[input + 1];
    dib[output + 2] = pixels[input]; dib[output + 3] = pixels[input + 3];
    if (!pixels[input + 3]) dib[40 + size * size * 4 + (size - 1 - y) * maskStride + (x >> 3)] |= 128 >> (x % 8);
  }
  frames.set(size, dib);
}
const header = Buffer.alloc(6 + iconSizes.length * 16);
header.writeUInt16LE(1, 2);
header.writeUInt16LE(iconSizes.length, 4);
let offset = header.length;
for (const [index, size] of iconSizes.entries()) {
  const entry = 6 + index * 16;
  header[entry] = header[entry + 1] = size === 256 ? 0 : size;
  header.writeUInt16LE(1, entry + 4);
  header.writeUInt16LE(32, entry + 6);
  header.writeUInt32LE(frames.get(size).length, entry + 8);
  header.writeUInt32LE(offset, entry + 12);
  offset += frames.get(size).length;
}
return Buffer.concat([header, ...iconSizes.map(size => frames.get(size))]);
}
const ico = await windowsIcon(0.65);
const trayIco = await windowsIcon(1);
for (const path of ['scripts/spawnt-tray.ico', 'web/public/downloads/spawnt-tray.ico']) await writeFile(new URL(path, root), trayIco);
const badgeDirectory = new URL('scripts/tray-badges/', root);
await mkdir(badgeDirectory, { recursive: true });
for (let count = 1; count <= 10; count++) {
  const label = count === 10 ? '9+' : String(count);
  const badge = Buffer.from(`<svg width="32" height="32" xmlns="http://www.w3.org/2000/svg"><rect x="13" y="13" width="18" height="18" fill="#ff2544" stroke="#070708" stroke-width="2"/><text x="22" y="26" text-anchor="middle" font-family="Arial" font-size="${count === 10 ? 10 : 14}" font-weight="bold" fill="#070708">${label}</text></svg>`);
  const icon = await sharp(images.get(32)).flatten({ background: '#070708' }).composite([{ input: badge }]).png().toBuffer();
  await writeFile(new URL(`spawnt-tray-${count === 10 ? '9plus' : count}.png`, badgeDirectory), icon);
}
for (const path of ['scripts/spawnt.ico', 'web/app/favicon.ico', 'web/public/downloads/spawnt.ico']) {
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
