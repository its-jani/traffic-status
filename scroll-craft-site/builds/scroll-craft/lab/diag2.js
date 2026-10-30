// Finds who paints the ghost copy under the peak act.
const { chromium } = require('playwright-core');

(async () => {
  const browser = await chromium.launch({
    executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe',
    args: ['--hide-scrollbars'],
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  await page.goto('http://localhost:4500', { waitUntil: 'load' });
  await page.waitForTimeout(1200);

  const target = await page.evaluate(() => {
    const act = document.querySelector('.act--audit');
    const r = act.getBoundingClientRect();
    const top = r.top + scrollY;
    const h = r.height;
    const vh = innerHeight;
    return { y: Math.round(top + (h - vh) * 0.5), top, h, vh };
  });
  console.log('audit act', target);
  await page.evaluate((y) => scrollTo({ top: y, behavior: 'instant' }), target.y);
  await page.waitForTimeout(900);

  const found = await page.evaluate(() => {
    const needle = 'Most of this work splits';
    const out = [];
    const walk = (el) => {
      for (const n of el.children) {
        if (n.children.length === 0 && (n.textContent || '').includes(needle.slice(0, 12))) {
          const r = n.getBoundingClientRect();
          const cs = getComputedStyle(n);
          const chain = [];
          let p = n;
          while (p && p !== document.body) {
            const s = getComputedStyle(p);
            chain.push({
              tag: p.tagName.toLowerCase(),
              cls: p.className && String(p.className).slice(0, 40),
              pos: s.position,
              z: s.zIndex,
              op: s.opacity,
              bg: s.backgroundColor,
              tf: s.transform === 'none' ? 'none' : s.transform,
              top: Math.round(p.getBoundingClientRect().top),
            });
            p = p.parentElement;
          }
          out.push({ text: (n.textContent || '').slice(0, 40), rect: { top: Math.round(r.top), bottom: Math.round(r.bottom) }, self: { op: cs.opacity, pos: cs.position, z: cs.zIndex, clip: cs.clipPath }, chain });
        }
        walk(n);
      }
    };
    walk(document.body);
    const grain = document.querySelector('.sc-grain');
    const gcs = grain ? getComputedStyle(grain) : null;
    return {
      out,
      scrollY,
      grain: gcs ? { op: gcs.opacity, z: gcs.zIndex, bgimg: (gcs.backgroundImage || '').slice(0, 80) } : null,
    };
  });
  console.log(JSON.stringify(found, null, 1).slice(0, 6000));
  await page.screenshot({ path: 'lab/diag2-peak.png' });
  await browser.close();
})();
