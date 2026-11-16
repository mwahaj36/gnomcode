// Keep encrypted hex frames below the relay's 64 KiB ceiling.
export function* remoteRequestParts(value) {
  if (value.type !== 'upload_attachment') { yield value; return; }
  const text = JSON.stringify(value);
  if (text.length <= 6000) { yield value; return; }
  for (let start = 0; start < text.length;) {
    let end = Math.min(start + 6000, text.length);
    const last = text.charCodeAt(end - 1);
    if (end < text.length && last >= 0xd800 && last <= 0xdbff) end--;
    yield { type: 'upload_chunk', data: text.slice(start, end), final: end === text.length };
    start = end;
  }
}
