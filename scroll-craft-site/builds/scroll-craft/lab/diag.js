const { chromium } = require("playwright-core");
(async () => {
  const b = await chromium.launch({ channel: "chrome" });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  await p.goto("http://localhost:4500", { waitUntil: "load" });
  await p.waitForTimeout(1500);
  const info = await p.evaluate(() => {
    const act = document.querySelector(".act--audit");
    const secTop = act.getBoundingClientRect().top + scrollY;
    const h = act.getBoundingClientRect().height;
    return { secTop, h, docH: document.body.scrollHeight };
  });
  console.log(info);
  // scroll to middle of the pinned travel
  await p.evaluate(({secTop, h}) => scrollTo(0, secTop + (h - innerHeight) * 0.5), info);
  await p.waitForTimeout(700);
  const dump = await p.evaluate(() => {
    const act = document.querySelector(".act--audit");
    const out = {};
    out.p = getComputedStyle(act).getPropertyValue("--sc-p");
    const stage = act.querySelector("[data-sc-stage]");
    const r = stage.getBoundingClientRect();
    out.stage = { top: r.top, h: r.height, cls: stage.className, display: getComputedStyle(stage).display };
    const inner = act.querySelector(".audit__inner");
    const ir = inner.getBoundingClientRect();
    out.inner = { top: ir.top, h: ir.height, left: ir.left, w: ir.width };
    const h2 = act.querySelector(".audit__h");
    const hr = h2.getBoundingClientRect();
    out.h2 = { top: hr.top, h: hr.height, left: hr.left, w: hr.width, op: getComputedStyle(h2).opacity, tr: getComputedStyle(h2).transform };
    const para = act.querySelector("[data-audit-para]");
    const pr = para.getBoundingClientRect();
    out.para = { top: pr.top, h: pr.height, w: pr.width };
    out.vals = {};
    act.querySelectorAll("[data-audit-out]").forEach(e => out.vals[e.dataset.auditOut] = e.textContent);
    return out;
  });
  console.log(JSON.stringify(dump, null, 1));
  await p.screenshot({ path: "lab/diag-audit.png" });
  await b.close();
})();
