import { createRequire } from 'node:module';
const { chromium } = createRequire(import.meta.url)('playwright');
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const root = new URL('../', import.meta.url);
const font = (await readFile(new URL('web/public/fonts/inter-variable.woff2', root))).toString('base64');
const mark = await readFile(new URL('web/public/gnomcode-mark.svg', root), 'utf8');
const browser = await chromium.launch({headless:true,executablePath:process.env.GNOMCODE_TEST_BROWSER});
try {
  const page = await browser.newPage({viewport:{width:1200,height:630},deviceScaleFactor:1});
  await page.setContent(`<style>@font-face{font-family:Inter;src:url(data:font/woff2;base64,${font})}*{box-sizing:border-box}body{margin:0;background:#070708;color:#f5f5f5;font-family:Inter,sans-serif;padding:64px}.brand{display:flex;align-items:center;gap:24px;font-size:40px;font-weight:650}.brand svg{width:44px;height:44px;color:#4f67f0}h1{font-size:76px;line-height:1.08;letter-spacing:-3px;font-weight:650;margin:58px 0}footer{border-top:1px solid #333;padding-top:24px;color:#b6b8bd;font-size:23px;display:flex;gap:40px}</style><div class="brand">${mark}<span>gnomcode</span></div><h1>The runtime for your<br>AI coding agents.</h1><footer><span>Claude Code</span><span>Codex</span><span>Antigravity</span></footer>`);
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({path:fileURLToPath(new URL('web/public/og-image.png',root))});
} finally { await browser.close(); }
