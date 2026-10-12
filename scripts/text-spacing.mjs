// WCAG 1.4.12 text spacing, checked in a browser (plan RG-014C, D-78).
//
// Usage: node scripts/text-spacing.mjs <out-dir> <page.html>[:<tails>] ...
//
// For each page, at viewport widths 320, 420, the breakpoint less one, the
// breakpoint, and 1400, with 1.4.12's four properties injected, it asserts:
//   - every displayed <text> lies inside its SVG's box (nothing clipped);
//   - no two displayed texts overlap, numbers and halos aside;
//   - every tail starts at or after its name's last character;
//   - the number of tails per arrangement is <tails>, when given;
//   - the accessibility tree's roles and names do not change.
// It writes a screenshot per page and width, and exits non-zero on failure.
//
// It drives Playwright's headless Chromium over the DevTools protocol with
// no dependency of its own. CHROME overrides the browser's path.

import { spawn } from 'node:child_process';
import { readdirSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve, basename } from 'node:path';

const SPACING =
  '*{letter-spacing:.12em!important;word-spacing:.16em!important;line-height:1.5!important}' +
  'p{margin-bottom:2em!important}';
const PORT = 9334;

function chrome() {
  if (process.env.CHROME) return process.env.CHROME;
  const cache = join(homedir(), 'Library/Caches/ms-playwright');
  const shell = readdirSync(cache).filter((d) => d.startsWith('chromium_headless_shell')).sort().pop();
  if (!shell) throw new Error('no headless Chromium: set CHROME, or install Playwright\'s');
  return join(cache, shell, 'chrome-headless-shell-mac-arm64', 'chrome-headless-shell');
}

async function connect() {
  const proc = spawn(chrome(), [`--remote-debugging-port=${PORT}`, '--no-first-run', 'about:blank'], {
    stdio: 'ignore',
  });
  for (let i = 0; i < 50; i++) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${PORT}/json`)).json();
      const page = targets.find((t) => t.type === 'page');
      if (page) {
        const ws = new WebSocket(page.webSocketDebuggerUrl);
        await new Promise((r) => ws.addEventListener('open', r));
        let id = 0;
        const pending = new Map();
        ws.addEventListener('message', (e) => {
          const m = JSON.parse(e.data);
          if (m.id && pending.has(m.id)) {
            pending.get(m.id)(m);
            pending.delete(m.id);
          }
        });
        const send = (method, params = {}) =>
          new Promise((r) => {
            const i2 = ++id;
            pending.set(i2, r);
            ws.send(JSON.stringify({ id: i2, method, params }));
          });
        return { proc, ws, send };
      }
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('Chromium did not start');
}

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
    const texts = [...svg.querySelectorAll('text')].filter((t) =>
      t.getNumberOfChars() > 0 &&
      getComputedStyle(t).visibility !== 'hidden' &&
      t.getAttribute('aria-hidden') !== 'true' &&
      !/^[0-9]+$/.test(t.textContent));
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
  return { problems, tails };
})()`;

async function axNames(send) {
  const { result } = await send('Accessibility.getFullAXTree');
  return result.nodes
    .filter((n) => !n.ignored && n.name?.value)
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
  const { proc, ws, send } = await connect();
  const evaluate = async (expression) =>
    (await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true })).result.result.value;
  let failures = 0;
  try {
    await send('Page.enable');
    await send('Accessibility.enable');
    for (const arg of pages) {
      const [file, expected] = arg.split(':');
      await send('Page.navigate', { url: 'file://' + resolve(file) });
      await new Promise((r) => setTimeout(r, 800));
      const query = await evaluate(`(document.querySelector('style').textContent.match(/max-width: (\\d+)px/)||[])[1]`);
      const breakpoint = Number(query) + 1;
      for (const width of [320, 420, breakpoint - 1, breakpoint, 1400]) {
        await send('Emulation.setDeviceMetricsOverride', { width, height: 1200, deviceScaleFactor: 1, mobile: false });
        await evaluate(`document.getElementById('spacing')?.remove()`);
        await new Promise((r) => setTimeout(r, 100));
        const before = await axNames(send);
        await evaluate(
          `(() => { const s = document.createElement('style'); s.id = 'spacing'; s.textContent = ${JSON.stringify(SPACING)}; document.head.append(s); })()`,
        );
        await new Promise((r) => setTimeout(r, 150));
        const { problems, tails } = await evaluate(CHECK);
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
    ws.close();
    proc.kill();
  }
  process.exit(failures === 0 ? 0 : 1);
}

await main();
