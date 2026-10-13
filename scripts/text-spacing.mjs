// WCAG 1.4.12 text spacing, checked in a browser (plan RG-014C, D-78).
//
// Usage: node scripts/text-spacing.mjs <out-dir> <page.html>[:<tails>][:scroll] ...
//
// For each page, at viewport widths 320, 420, the breakpoint less one, the
// breakpoint, and 1400, with 1.4.12's four properties injected, it asserts:
//   - every displayed <text> lies inside its SVG's box (nothing clipped);
//   - no two displayed texts overlap, numbers and halos aside;
//   - every tail starts at or after its name's last character;
//   - the number of tails per arrangement is <tails>, when given;
//   - the page never scrolls sideways, and no text in a table cell or in
//     the detail panel spills out of its box (FR-41's table and FR-38's
//     panel, both opened first, D-78);
//   - with :scroll, the table's own region does scroll at 320 px, so the
//     check above is not vacuous for a table that happens to fit;
//   - the accessibility tree's roles and names do not change.
// It writes a screenshot per page and width, and exits non-zero on failure.
//
// It drives Chromium through scripts/cdp.mjs. Chromium only: Firefox and
// Safari are not covered.

import { writeFileSync } from 'node:fs';
import { join, resolve, basename } from 'node:path';
import { pathToFileURL } from 'node:url';
import { SPILLS, connect } from './cdp.mjs';

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
    const svg = arrangement.querySelector('svg');
    const frame = svg.getBoundingClientRect();
    // A marker's number is drawn right after its circle and sits on it by
    // design; a name of digits alone is still checked. This leans on the
    // emitter drawing each number straight after its circle.
    const texts = [...svg.querySelectorAll('text')].filter((t) =>
      t.getNumberOfChars() > 0 &&
      getComputedStyle(t).visibility !== 'hidden' &&
      t.getAttribute('aria-hidden') !== 'true' &&
      t.previousElementSibling?.tagName !== 'circle');
    const boxes = texts.map((t) => ({ t, r: t.getBoundingClientRect() }));
    for (const { t, r } of boxes) {
      if (r.left < frame.left - slack || r.right > frame.right + slack ||
          r.top < frame.top - slack || r.bottom > frame.bottom + slack) {
        problems.push('outside: ' + t.textContent.slice(0, 40) + ' ' +
          JSON.stringify([r.left, r.right, frame.left, frame.right]));
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
    for (const tspan of svg.querySelectorAll('tspan[dx]')) {
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
  if (document.documentElement.scrollWidth > innerWidth + slack) {
    problems.push('the page scrolls sideways: ' + document.documentElement.scrollWidth + ' > ' + innerWidth);
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

async function axNames(send) {
  const { result } = await send('Accessibility.getFullAXTree');
  return result.nodes
    // InlineTextBox nodes are line fragments, not semantics: HTML text
    // rewraps under the added spacing, which is 1.4.12 working.
    .filter((n) => !n.ignored && n.name?.value && n.role?.value !== 'InlineTextBox')
    .map((n) => `${n.role?.value}:${n.name.value}`)
    .sort()
    .join('\n');
}

async function main() {
  const [outDir, ...pages] = process.argv.slice(2);
  if (!outDir || pages.length === 0) {
    console.error('usage: node scripts/text-spacing.mjs <out-dir> <page.html>[:<tails>] ...');
    process.exit(2);
  }
  const { send, close, evaluate, navigate } = await connect();
  let failures = 0;
  try {
    await send('Page.enable');
    await send('Accessibility.enable');
    for (const arg of pages) {
      // The tail count follows the last colon, so a Windows drive letter
      // stays in the path.
      const [, file, expected, scroll] = arg.match(/^(.*?)(?::(\d+))?(:scroll)?$/);
      await navigate(pathToFileURL(resolve(file)).href);
      // Open the table view once, before any accessibility snapshot, so
      // opening it is never reported as the tree changing.
      await evaluate(`document.querySelectorAll('details').forEach((d) => { d.open = true; })`);
      // And the detail panel, for the table's first row (FR-38), so its
      // text is checked too.
      await evaluate(`document.querySelector('tr[data-row] button')?.click()`);
      const query = await evaluate(`(document.querySelector('style').textContent.match(/max-width: (\\d+)px/)||[])[1]`);
      const breakpoint = Number(query) + 1;
      if (!Number.isInteger(breakpoint)) throw new Error(`${file}: no max-width breakpoint in its stylesheet`);
      for (const width of [320, 420, breakpoint - 1, breakpoint, 1400]) {
        await send('Emulation.setDeviceMetricsOverride', { width, height: 1200, deviceScaleFactor: 1, mobile: false });
        await evaluate(`document.getElementById('spacing')?.remove()`);
        await evaluate(SETTLED);
        const before = await axNames(send);
        await evaluate(
          `(() => { const s = document.createElement('style'); s.id = 'spacing'; s.textContent = ${JSON.stringify(SPACING)}; document.head.append(s); })()`,
        );
        await evaluate(SETTLED);
        const { problems, tails, scrolls } = await evaluate(CHECK);
        if (scroll && width === 320 && !scrolls) problems.push('the table region does not scroll at 320 px');
        if (expected !== undefined && tails !== Number(expected)) {
          problems.push(`tails: ${tails}, expected ${expected}`);
        }
        if ((await axNames(send)) !== before) problems.push('the accessibility tree changed');
        const shot = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
        writeFileSync(join(outDir, `${basename(file, '.html')}-${width}.png`), Buffer.from(shot.result.data, 'base64'));
        const verdict = problems.length === 0 ? 'ok' : 'FAIL';
        console.log(`${verdict} ${basename(file)} @${width}px (${tails} tails)`);
        for (const p of problems) console.log('    ' + p);
        failures += problems.length;
      }
    }
  } finally {
    await close();
  }
  process.exit(failures === 0 ? 0 : 1);
}

await main();
