const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { renderNotices } = require('../desktop-notices.cjs');

test('notices include and escape the actual production dependency license', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'spawnt-notices-'));
  try {
    fs.writeFileSync(path.join(dir,'package.json'), JSON.stringify({name:'example<unsafe>',version:'1.0.0',license:'MIT'}));
    assert.throws(() => renderNotices([dir]));
    fs.writeFileSync(path.join(dir,'LICENSE'),'Terms <literal> & attribution');
    const html=renderNotices([dir]);
    assert.match(html,/example&lt;unsafe&gt;/);
    assert.match(html,/Terms &lt;literal&gt; &amp; attribution/);
  } finally { fs.rmSync(dir,{recursive:true,force:true}); }
});
