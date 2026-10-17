// WCAG 1.4.12 text spacing, checked in a browser (plan RG-014C, D-78).
//
// Usage: node scripts/text-spacing.mjs <out-dir> <page.html>[:<tails>][:scroll] ...
//
// In each of Chromium, Firefox and WebKit, for each page, at viewport widths
// 320, 420, the breakpoint less one, the breakpoint, and 1400, with 1.4.12's
// four properties injected, it asserts:
//   - every displayed <text> lies inside its own SVG's box (nothing clipped),
//     the wide arrangement's parts each counting as one (D-85);
//   - no two displayed texts overlap, numbers and halos aside;
//   - every tail starts at or after its name's last character;
//   - the number of tails per arrangement is <tails>, when given;
//   - the page never scrolls sideways, and no text in a table cell or in
//     the detail panel spills out of its box (FR-41's table and FR-38's
//     panel, both opened first, D-78);
//   - with :scroll, the table's own region does scroll at 320 px, so the
//     check above is not vacuous for a table that happens to fit;
//   - the roles and names in Playwright's ARIA snapshot (and, in Chromium,
//     in its own accessibility tree) do not change.
// It writes a screenshot per engine, page and width, and exits non-zero on
// any failure, in any engine; an engine that will not start is a failure,
// not a skip (RG-015B).
//
// Why three engines (rg-497): every measurement here is the engine's own
// layout, so each engine is measured as it draws. Chromium turns optional
// ligatures off under letter-spacing; an engine that keeps them shapes a
// spaced name differently, and only that engine can say whether it still
// fits. The engines come from Playwright, a dev dependency (package.json);
// `just browser-setup` installs it and the three browsers. CHROME, as in
// scripts/cdp.mjs, overrides Chromium's path.

import { join, resolve, basename } from 'node:path';
import { pathToFileURL } from 'node:url';
import { SPILLS, TIMEOUT_MS, within } from './cdp.mjs';

const SPACING =
  '*{letter-spacing:.12em!important;word-spacing:.16em!important;line-height:1.5!important}' +
  'p{margin-bottom:2em!important}';

// Settles layout: the fonts are in, and two frames have painted since the
// last change, so a measurement sees the styles just applied.
const SETTLED = `document.fonts.ready.then(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))))`;

// Runs in the page: every check over the displayed arrangement.
const CHECK = `(() => {
  const slack = 0.5;
  const problems = [];
  let tails = 0;
  const shown = [...document.querySelectorAll('.arrangement')]
    .filter((e) => getComputedStyle(e).display !== 'none');
  for (const arrangement of shown) {
    // The wide arrangement is drawn in parts (D-85): each text is checked
    // against its own SVG's box, and for overlap against every other.
    const boxes = [];
    for (const svg of arrangement.querySelectorAll('svg')) {
      const frame = svg.getBoundingClientRect();
      // A marker's number is drawn right after its circle and sits on it by
      // design; a name of digits alone is still checked. This leans on the
      // emitter drawing each number straight after its circle.
      const texts = [...svg.querySelectorAll('text')].filter((t) =>
        t.getNumberOfChars() > 0 &&
        getComputedStyle(t).visibility !== 'hidden' &&
        t.getAttribute('aria-hidden') !== 'true' &&
        t.previousElementSibling?.tagName !== 'circle');
      for (const t of texts) {
        const r = t.getBoundingClientRect();
        boxes.push({ t, r });
        if (r.left < frame.left - slack || r.right > frame.right + slack ||
            r.top < frame.top - slack || r.bottom > frame.bottom + slack) {
          problems.push('outside: ' + t.textContent.slice(0, 40) + ' ' +
            JSON.stringify([r.left, r.right, frame.left, frame.right]));
        }
      }
    }
    for (let i = 0; i < boxes.length; i++) {
      for (let j = i + 1; j < boxes.length; j++) {
        const a = boxes[i].r, b = boxes[j].r;
        const w = Math.min(a.right, b.right) - Math.max(a.left, b.left);
        const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
        if (w > slack && h > slack) {
          problems.push('overlap: ' + boxes[i].t.textContent.slice(0, 30) + ' / ' +
            boxes[j].t.textContent.slice(0, 30));
        }
      }
    }
    for (const tspan of arrangement.querySelectorAll('tspan[dx]')) {
      tails++;
      const text = tspan.parentElement;
      const start = text.getNumberOfChars() - tspan.getNumberOfChars();
      const nameEnd = text.getEndPositionOfChar(start - 1).x;
      const tailStart = text.getStartPositionOfChar(start).x;
      if (tailStart < nameEnd - slack) {
        problems.push('tail before its name ends: ' + text.textContent.slice(0, 40));
      }
    }
  }
  // The table view (FR-41): its region may scroll, the page may not, and no
  // cell's text may spill out of the cell's box under 1.4.12's spacing.
  // clientWidth, not innerWidth: an engine with classic scrollbars counts
  // the bar in innerWidth, which would hide an overflow narrower than it.
  const viewport = document.documentElement.clientWidth;
  if (document.documentElement.scrollWidth > viewport + slack) {
    problems.push('the page scrolls sideways: ' + document.documentElement.scrollWidth + ' > ' + viewport);
  }
  const spills = ${SPILLS};
  for (const text of spills('.table-scroll td, .table-scroll th, summary, .detail-heading, .detail dt, .detail dd', slack)) {
    problems.push('clipped: ' + text);
  }
  const panel = document.getElementById('detail');
  if (document.querySelector('tr[data-row] button') && panel?.hidden) {
    problems.push('the detail panel did not open, so its text went unchecked');
  }
  const scrolls = [...document.querySelectorAll('.table-scroll')]
    .some((r) => r.scrollWidth > r.clientWidth + slack);
  return { problems, tails, scrolls };
})()`;

const playwright = await import('playwright').catch((error) => {
  if (error.code !== 'ERR_MODULE_NOT_FOUND') throw error;
  console.error("no Playwright: run 'just browser-setup' (npm ci, then the browsers)");
  process.exit(2);
});

// The engines, in the order they run. CHROME stands in for Playwright's
// headless Chromium, as it does for the CDP scripts.
const ENGINES = [
  ['chromium', { executablePath: process.env.CHROME || undefined }],
  ['firefox', {}],
  ['webkit', {}],
];

// The roles and names of the accessibility tree, as one string to compare.
// Every engine gives Playwright's ARIA snapshot, computed from the DOM and
// the engine's computed styles; Chromium also gives its own tree, which is
// what its screen readers read. Its InlineTextBox nodes are line fragments,
// not semantics: HTML text rewraps under the added spacing, which is 1.4.12
// working.
async function axNames(page, cdp) {
  const aria = await page.locator(':root').ariaSnapshot();
  if (!cdp) return aria;
  const { nodes } = await cdp.send('Accessibility.getFullAXTree');
  const native = nodes
    .filter((n) => !n.ignored && n.name?.value && n.role?.value !== 'InlineTextBox')
    .map((n) => `${n.role?.value}:${n.name.value}`)
    .sort()
    .join('\n');
  return `${aria}\n${native}`;
}

// `expression` evaluated in the page and awaited; a hang fails the run.
const evaluate = (page, expression) => within(page.evaluate(expression), expression.slice(0, 40));

// Every check on every page in one engine; returns the number of problems.
async function checkEngine(name, launchOptions, outDir, pages) {
  const browser = await playwright[name].launch(launchOptions);
  let failures = 0;
  try {
    const context = await browser.newContext({ viewport: { width: 1400, height: 1200 }, deviceScaleFactor: 1 });
    const page = await context.newPage();
    page.setDefaultTimeout(TIMEOUT_MS);
    const cdp = name === 'chromium' ? await context.newCDPSession(page) : undefined;
    if (cdp) await cdp.send('Accessibility.enable');
    for (const arg of pages) {
      // The tail count follows the last colon, so a Windows drive letter
      // stays in the path.
      const [, file, expected, scroll] = arg.match(/^(.*?)(?::(\d+))?(:scroll)?$/);
      await page.goto(pathToFileURL(resolve(file)).href);
      // Open the table view once, before any accessibility snapshot, so
      // opening it is never reported as the tree changing.
      await evaluate(page, `document.querySelectorAll('details').forEach((d) => { d.open = true; })`);
      // And the detail panel, for the table's first row (FR-38), so its
      // text is checked too.
      await evaluate(page, `document.querySelector('tr[data-row] button')?.click()`);
      const query = await evaluate(page, `(document.querySelector('style').textContent.match(/max-width: (\\d+)px/)||[])[1]`);
      const breakpoint = Number(query) + 1;
      if (!Number.isInteger(breakpoint)) throw new Error(`${file}: no max-width breakpoint in its stylesheet`);
      for (const width of [320, 420, breakpoint - 1, breakpoint, 1400]) {
        await page.setViewportSize({ width, height: 1200 });
        await evaluate(page, `document.getElementById('spacing')?.remove()`);
        await evaluate(page, SETTLED);
        const before = await axNames(page, cdp);
        await evaluate(
          page,
          `(() => { const s = document.createElement('style'); s.id = 'spacing'; s.textContent = ${JSON.stringify(SPACING)}; document.head.append(s); })()`,
        );
        await evaluate(page, SETTLED);
        const { problems, tails, scrolls } = await evaluate(page, CHECK);
        if (scroll && width === 320 && !scrolls) problems.push('the table region does not scroll at 320 px');
        if (expected !== undefined && tails !== Number(expected)) {
          problems.push(`tails: ${tails}, expected ${expected}`);
        }
        if ((await axNames(page, cdp)) !== before) problems.push('the accessibility tree changed');
        await page.screenshot({ path: join(outDir, `${basename(file, '.html')}-${name}-${width}.png`), fullPage: true });
        const verdict = problems.length === 0 ? 'ok' : 'FAIL';
        console.log(`${verdict} ${name} ${basename(file)} @${width}px (${tails} tails)`);
        for (const p of problems) console.log('    ' + p);
        failures += problems.length;
      }
    }
  } finally {
    await browser.close();
  }
  return failures;
}

async function main() {
  const [outDir, ...pages] = process.argv.slice(2);
  if (!outDir || pages.length === 0) {
    console.error('usage: node scripts/text-spacing.mjs <out-dir> <page.html>[:<tails>][:scroll] ...');
    process.exit(2);
  }
  // Every engine runs, whatever an earlier one found, so one run reports
  // all three; an engine that fails to start or throws counts as a failure.
  const failed = [];
  for (const [name, options] of ENGINES) {
    try {
      if ((await checkEngine(name, options, outDir, pages)) > 0) failed.push(name);
    } catch (error) {
      // Playwright follows a missing browser's first line with an install
      // banner that bypasses the pin, so the hint replaces it. Any other
      // failure keeps its whole message: a launch failure's cause is in the
      // browser logs after its first line.
      const message = String(error?.message ?? error);
      const text = message.includes("Executable doesn't exist")
        ? `${message.split('\n')[0]}; run 'just browser-setup'`
        : message;
      console.log(`FAIL ${name}: ${text}`);
      failed.push(name);
    }
  }
  console.log(failed.length === 0 ? 'text spacing: ok in chromium, firefox and webkit' : `text spacing: FAIL in ${failed.join(', ')}`);
  process.exit(failed.length === 0 ? 0 : 1);
}

await main();
