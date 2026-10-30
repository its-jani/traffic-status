// Under reduced motion the pan act must still hand every panel to the reader.
const { chromium } = require('playwright-core');

(async () => {
  const browser = await chromium.launch({
    executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe',
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: 'reduce' });
  await page.goto('http://localhost:4500', { waitUntil: 'load' });
  await page.waitForTimeout(1000);

  const out = await page.evaluate(() => {
    const stage = document.querySelector('[data-sc-act="pan"] [data-sc-stage]');
    const rail = document.querySelector('[data-sc-pan]');
    const last = rail.lastElementChild;
    stage.scrollLeft = 99999;
    return {
      stageScrollWidth: stage.scrollWidth,
      stageClientWidth: stage.clientWidth,
      railWidth: Math.round(rail.getBoundingClientRect().width),
      railScrollWidth: rail.scrollWidth,
      lastRight: Math.round(last.getBoundingClientRect().right),
      overflowX: getComputedStyle(stage).overflowX,
      scrolledTo: stage.scrollLeft,
    };
  });
  console.log(JSON.stringify(out, null, 1));
  await browser.close();
})();
