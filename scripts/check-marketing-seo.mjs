import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { mkdir } from 'node:fs/promises';
const { chromium } = createRequire(import.meta.url)('playwright');
const base = process.argv[2] || 'http://127.0.0.1:43915';
const guides = ['getting-started', 'model-switching', 'remote-access', 'supported-agents', 'quota-and-cost', 'model-pricing', 'automate-multiple-agents', 'prompt-from-phone', 'faq', 'quota-limits-explained', 'privacy', 'claude-code-vs-codex'];
const routes = ['/', '/downloads', '/docs', '/pricing', '/remote', '/privacy', '/terms', ...guides.map(slug => `/docs/${slug}`)];
const browser = await chromium.launch({ headless: true, executablePath: process.env.SPAWNT_TEST_BROWSER });
const titles = new Set();
const descriptions = new Set();
try {
  const page = await browser.newPage({ reducedMotion: 'reduce' });
  for (const route of routes) {
    const response = await page.goto(base + route, { waitUntil: 'networkidle' });
    assert.equal(response.status(), 200, route);
    const title = await page.title();
    assert.ok(!titles.has(title), `duplicate title: ${title}`); titles.add(title);
    const description = await page.locator('meta[name="description"]').getAttribute('content');
    assert.ok(description && description.length <= 150, `description: ${route}`);
    assert.ok(!descriptions.has(description), `duplicate description: ${route}`); descriptions.add(description);
    assert.equal(await page.locator('link[rel="canonical"]').getAttribute('href'), `https://spawntv.vercel.app${route === '/' ? '' : route}`);
    assert.equal(await page.locator('meta[property="og:image"]').getAttribute('content'), 'https://spawntv.vercel.app/og-image.png');
    for (const script of await page.locator('script[type="application/ld+json"]').allTextContents()) JSON.parse(script);
    const text = await page.locator('body').innerText();
    assert.ok(!/\bTODO\b|\bTBD\b|\bplaceholder\b|coming soon|\p{Emoji_Presentation}/iu.test(text), `unfinished copy: ${route}`);
    if (route.startsWith('/docs/')) {
      assert.ok((await page.locator('article > p').first().innerText()).length > 60, `direct answer: ${route}`);
      assert.ok(await page.getByRole('heading', { name: 'Related guides', exact: true }).count());
    }
    for (const href of await page.locator('article a[href^="/"]').evaluateAll(nodes => [...new Set(nodes.map(n => n.getAttribute('href').split('#')[0]))])) {
      assert.ok((await page.request.get(base + href)).ok(), `broken link: ${route} -> ${href}`);
    }
  }
  await page.goto(base + '/docs/faq');
  const faq = JSON.parse(await page.locator('script[type="application/ld+json"]').textContent());
  assert.ok(faq.mainEntity.length >= 15);
  assert.equal(await page.locator('article h3').count(), faq.mainEntity.length);
  const imageResponse = await page.request.get(base + '/og-image.png');
  assert.ok(imageResponse.ok(), 'social preview image exists');
  assert.ok(imageResponse.headers()['content-type'].startsWith('image/png'));
  for (const route of ['/auth', '/console', '/settings']) {
    const html = await (await page.request.get(base + route)).text();
    assert.match(html, /name="robots" content="noindex, nofollow"/, `private route: ${route}`);
  }
  const sitemap = await (await page.request.get(base + '/sitemap.xml')).text();
  for (const route of routes) assert.ok(sitemap.includes(`<loc>https://spawntv.vercel.app${route === '/' ? '' : route}</loc>`), `sitemap: ${route}`);
  assert.ok(!/\/console<|\/auth<|\/settings</.test(sitemap));
  const robots = await (await page.request.get(base + '/robots.txt')).text();
  for (const rule of ['Disallow: /api/', 'Disallow: /_next/', 'Disallow: /console', 'User-Agent: GPTBot', 'User-Agent: PerplexityBot', 'User-Agent: ClaudeBot']) assert.ok(robots.toLowerCase().includes(rule.toLowerCase()), rule);
  await mkdir('.tmp/marketing-seo', { recursive: true });
  for (const width of [390, 1440]) for (const theme of ['dark', 'light']) {
    await page.setViewportSize({ width, height: 900 });
    for (const route of ['/', '/docs/model-switching', '/privacy', '/terms', '/downloads', '/pricing']) {
      await page.goto(base + route, { waitUntil: 'networkidle' });
      await page.evaluate(theme => { document.documentElement.dataset.theme = theme; localStorage.setItem('spawnt-theme', theme); }, theme);
      await page.evaluate(() => document.fonts.ready);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), `overflow: ${route} ${width}`);
      await page.screenshot({ path: `.tmp/marketing-seo/${route.replaceAll('/', '-') || 'home'}-${width}-${theme}.png`, fullPage: true });
    }
  }
  console.log(`Verified ${routes.length} public routes, metadata, links, schemas, sitemap, robots and responsive layouts.`);
} finally { await browser.close(); }
