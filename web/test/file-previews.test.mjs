import test from 'node:test';
import assert from 'node:assert/strict';
import { renderFileViewer, renderArtifactPreview } from '../lib/console-presentation.js';

const PNG = 'data:image/png;base64,iVBORw==';

test('images and SVG render only as img with click-to-open', () => {
  const svg = renderFileViewer({ path: 'a.svg', name: 'a.svg', kind: 'svg', size: 30, data_url: 'data:image/svg+xml;base64,PHN2Zz4=' });
  assert.match(svg, /<img src="data:image\/svg\+xml;base64,PHN2Zz4="/);
  assert.match(svg, /data-image-open=/);
  assert.doesNotMatch(svg, /<svg/);
  const bad = renderFileViewer({ path: 'a.png', kind: 'image', data_url: 'https://example.com/a.png', size: 3 });
  assert.doesNotMatch(bad, /<img/);
});

test('markdown, json, csv, code and text previews', () => {
  assert.match(renderFileViewer({ path: 'r.md', kind: 'markdown', content: '# T\n<script>x</script>' }), /<h1>T<\/h1>[\s\S]*&lt;script&gt;/);
  assert.match(renderFileViewer({ path: 'd.json', kind: 'json', content: '{"a":1}' }), /sp-code-lang">json[\s\S]*&quot;a&quot;/);
  const csv = renderFileViewer({ path: 'r.csv', kind: 'csv', content: 'h1,h2\n"a,b",<c>\n' });
  assert.match(csv, /<th>h1<\/th><th>h2<\/th>/);
  assert.match(csv, /<td>a,b<\/td><td>&lt;c&gt;<\/td>/);
  assert.match(renderFileViewer({ path: 'm.rs', kind: 'code', language: 'rust', content: 'fn x() {}' }), /sp-hl-keyword">fn/);
  assert.match(renderFileViewer({ path: 't.txt', kind: 'text', content: 'plain' }), /sp-file-code/);
});

test('pdf opens via button, binary shows size and download, large files are capped', () => {
  assert.match(renderFileViewer({ path: 'a.pdf', kind: 'pdf', size: 10, data_url: 'data:application/pdf;base64,JVBERg==' }), /data-pdf-open/);
  const bin = renderFileViewer({ path: 'x/blob.bin', name: 'blob.bin', kind: 'binary', size: 2048, data_url: 'data:application/octet-stream;base64,AAAA' });
  assert.match(bin, /2\.0 KB/);
  assert.match(bin, /download="blob.bin"/);
  const big = renderFileViewer({ path: 'big.png', kind: 'image', size: 5e6, truncated: true });
  assert.match(big, /Over the 1 MiB preview limit/);
  assert.doesNotMatch(big, /Download/);
  const longText = renderFileViewer({ path: 'l.log', kind: 'text', size: 9e5, truncated: true, content: 'x' });
  assert.match(longText, /Preview truncated at 512 KiB/);
});

test('artifact code uses the Rust language and images open full size', () => {
  assert.match(renderArtifactPreview({ path: 'a.py', kind: 'other', language: 'python', content: 'def f(): pass' }), /sp-code-lang">python/);
  assert.match(renderArtifactPreview({ path: 'a.png', kind: 'image', data_url: PNG }), /data-image-open/);
});
