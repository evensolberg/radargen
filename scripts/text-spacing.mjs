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
//   - the page never scrolls sideways, and no table cell's text spills
//     out of its cell (FR-41's table, opened first, D-78);
//   - with :scroll, the table's own region does scroll at 320 px, so the
//     check above is not vacuous for a table that happens to fit;
//   - the accessibility tree's roles and names do not change.
// It writes a screenshot per page and width, and exits non-zero on failure.
//
// It drives Playwright's headless Chromium over the DevTools protocol with
// no dependency of its own. CHROME overrides the browser's path. Chromium
// only: Firefox and Safari are not covered.

import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join, resolve, basename } from 'node:path';
import { pathToFileURL } from 'node:url';

const SPACING =
  '*{letter-spacing:.12em!important;word-spacing:.16em!important;line-height:1.5!important}' +
  'p{margin-bottom:2em!important}';
// Playwright's cache and its headless shell's folder, per platform.
const CACHES = {
  darwin: 'Library/Caches/ms-playwright',
  linux: '.cache/ms-playwright',
  win32: 'AppData/Local/ms-playwright',
};
const SHELLS = {
  'darwin-arm64': 'chrome-headless-shell-mac-arm64',
  'darwin-x64': 'chrome-headless-shell-mac-x64',
  'linux-x64': 'chrome-headless-shell-linux64',
  'win32-x64': 'chrome-headless-shell-win64',
};
const MISSING =
  "no headless Chromium: set CHROME, or run 'npx playwright install chromium-headless-shell' " +
  '(Playwright ships no shell for this platform when it is not macOS, linux-x64 or win32-x64)';
// How long Chromium may take to start, a page to load, or a frame to paint.
const TIMEOUT_MS = 15000;

function chrome() {
  if (process.env.CHROME) return process.env.CHROME;
  const cache = join(homedir(), CACHES[process.platform] ?? CACHES.linux);
  const folder = SHELLS[`${process.platform}-${process.arch}`];
  if (!folder || !existsSync(cache)) throw new Error(MISSING);
  // The newest build: names end in a build number, compared as a number.
  const build = (d) => Number(d.split('-').pop());
  const shell = readdirSync(cache)
    .filter((d) => /^chromium_headless_shell-\d+$/.test(d))
    .sort((a, b) => build(a) - build(b))
    .pop();
  const exe = shell && join(cache, shell, folder, process.platform === 'win32' ? 'chrome-headless-shell.exe' : 'chrome-headless-shell');
  if (!exe || !existsSync(exe)) throw new Error(MISSING);
  return exe;
}

// Polls `ready` every 100 ms until it returns a value, for up to TIMEOUT_MS.
async function poll(ready, what) {
  for (let i = 0; i < TIMEOUT_MS / 100; i++) {
    const value = await ready();
    if (value) return value;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(what);
}

// `promise`, or a rejection naming `what` after TIMEOUT_MS: a page that
// never loads or a frame that never paints fails the run, not hangs it.
function within(promise, what) {
  let timer;
  const late = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`timed out: ${what}`)), TIMEOUT_MS);
  });
  return Promise.race([promise, late]).finally(() => clearTimeout(timer));
}

async function connect() {
  // Port 0 lets Chromium pick a free port, which it writes to
  // DevToolsActivePort in its profile, so two runs never collide.
  const exe = chrome();
  const profile = mkdtempSync(join(tmpdir(), 'text-spacing-'));
  const proc = spawn(exe, ['--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', 'about:blank'], {
    stdio: 'ignore',
  });
  const exited = new Promise((r) => proc.once('close', r));
  const failed = new Promise((_, reject) => proc.once('error', reject));
  // Kills Chromium if it is still running, then removes its profile.
  const stop = async () => {
    if (proc.exitCode === null && proc.signalCode === null && proc.pid !== undefined) {
      proc.kill();
      await exited;
    }
    rmSync(profile, { recursive: true, force: true });
  };
  try {
    return await Promise.race([failed, attach(profile, stop)]);
  } catch (error) {
    await stop();
    throw error;
  }
}

async function attach(profile, stop) {
  const activePort = join(profile, 'DevToolsActivePort');
  const port = await poll(() => {
    try {
      return readFileSync(activePort, 'utf8').split('\n')[0];
    } catch {
      return undefined; // not written yet
    }
  }, 'Chromium did not start');
  const page = await poll(async () => {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      return targets.find((t) => t.type === 'page');
    } catch {
      return undefined; // not listening yet
    }
  }, 'Chromium has no page');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener('open', r));
  let id = 0;
  const pending = new Map();
  const waiting = new Map();
  ws.addEventListener('message', (e) => {
    const m = JSON.parse(e.data);
    if (m.id && pending.has(m.id)) {
      pending.get(m.id)(m);
      pending.delete(m.id);
    } else if (m.method && waiting.has(m.method)) {
      waiting.get(m.method)(m);
      waiting.delete(m.method);
    }
  });
  const send = (method, params = {}) =>
    new Promise((r) => {
      const i2 = ++id;
      pending.set(i2, r);
      ws.send(JSON.stringify({ id: i2, method, params }));
    });
  // Resolves on the next `method` event; register before the call that fires it.
  const next = (method) => new Promise((r) => waiting.set(method, r));
  const close = async () => {
    ws.close();
    await stop();
  };
  return { send, next, close };
}

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
  for (const cell of document.querySelectorAll('.table-scroll td, .table-scroll th, summary')) {
    // Visually hidden text is laid out unwrapped in a 1px box by design, so
    // only the text a reader sees is measured.
    const box = cell.getBoundingClientRect();
    const walk = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
    let right = -Infinity, bottom = -Infinity;
    for (let node = walk.nextNode(); node; node = walk.nextNode()) {
      if (node.parentElement.closest('.visually-hidden')) continue;
      const range = document.createRange();
      range.selectNodeContents(node);
      for (const r of range.getClientRects()) {
        right = Math.max(right, r.right);
        bottom = Math.max(bottom, r.bottom);
      }
    }
    if (bottom > box.bottom + slack || right > box.right + slack) {
      problems.push('clipped: ' + cell.textContent.slice(0, 40));
    }
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
  const { send, next, close } = await connect();
  const evaluate = async (expression) =>
    (await within(send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }), expression.slice(0, 40)))
      .result.result.value;
  let failures = 0;
  try {
    await send('Page.enable');
    await send('Accessibility.enable');
    for (const arg of pages) {
      // The tail count follows the last colon, so a Windows drive letter
      // stays in the path.
      const [, file, expected, scroll] = arg.match(/^(.*?)(?::(\d+))?(:scroll)?$/);
      const loaded = next('Page.loadEventFired');
      const navigated = await send('Page.navigate', { url: pathToFileURL(resolve(file)).href });
      if (navigated.result?.errorText) throw new Error(`${file}: ${navigated.result.errorText}`);
      await within(loaded, `${file} to load`);
      // Open the table view once, before any accessibility snapshot, so
      // opening it is never reported as the tree changing.
      await evaluate(`document.querySelectorAll('details').forEach((d) => { d.open = true; })`);
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
