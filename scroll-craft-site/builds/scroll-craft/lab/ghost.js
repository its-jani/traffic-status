// Bisects which element paints the faint ghost copy in the top strip of the peak.
const { chromium } = require('playwright-core');

const URL = 'http://localhost:4500';

async function shot(page, tag) {
  const buf = await page.screenshot({ clip: { x: 0, y: 0, width: 1440, height: 300 } });
  require('fs').writeFileSync(`lab/ghost-${tag}.png`, buf);
}

(async () => {
  const browser = await chromium.launch({
    executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe',
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto(URL, { waitUntil: 'load' });
  await page.waitForTimeout(1200);
  await page.evaluate(() => {
    const a = document.querySelector('.act--audit');
    const r = a.getBoundingClientRect();
    scrollTo({ top: r.top + scrollY + (a.offsetHeight - innerHeight) * 0.5, behavior: 'instant' });
  });
  await page.waitForTimeout(800);
  await shot(page, 'a-baseline');

  // ghosts from earlier sections?
  await page.evaluate(() => {
    document.querySelectorAll('.act').forEach((s) => {
      if (!s.classList.contains('act--audit')) s.style.visibility = 'hidden';
    });
  });
  await page.waitForTimeout(300);
  await shot(page, 'b-others-hidden');

  // ghost from inside the peak?
  await page.evaluate(() => {
    const a = document.querySelector('.act--audit');
    a.querySelectorAll('.anno').forEach((e) => (e.style.visibility = 'hidden'));
  });
  await page.waitForTimeout(300);
  await shot(page, 'c-annos-hidden');

  await page.evaluate(() => {
    document.querySelector('.audit__inner').style.visibility = 'hidden';
  });
  await page.waitForTimeout(300);
  await shot(page, 'd-inner-hidden');

  await browser.close();
})();
