// Playwright's headless Chromium over the DevTools protocol, with no
// dependency of its own: what the browser scripts share (text-spacing,
// interaction, focus-cost, a11y-audit, screenshots). CHROME overrides the
// browser's path.

import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';

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
  "no headless Chromium: set CHROME, or run 'just browser-setup' " +
  '(Playwright ships no shell for this platform when it is not macOS, linux-x64 or win32-x64)';
// How long Chromium may take to start, a page to load, or a frame to paint.
export const TIMEOUT_MS = 15000;

export function chrome() {
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
export async function poll(ready, what) {
  for (let i = 0; i < TIMEOUT_MS / 100; i++) {
    const value = await ready();
    if (value) return value;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(what);
}

// `promise`, or a rejection naming `what` after TIMEOUT_MS: a page that
// never loads or a frame that never paints fails the run, not hangs it.
export function within(promise, what) {
  let timer;
  const late = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`timed out: ${what}`)), TIMEOUT_MS);
  });
  return Promise.race([promise, late]).finally(() => clearTimeout(timer));
}

export async function connect() {
  // Port 0 lets Chromium pick a free port, which it writes to
  // DevToolsActivePort in its profile, so two runs never collide.
  const exe = chrome();
  const profile = mkdtempSync(join(tmpdir(), 'radargen-cdp-'));
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
  const listeners = [];
  ws.addEventListener('message', (e) => {
    const m = JSON.parse(e.data);
    for (const [method, fn] of listeners) if (m.method === method) fn(m.params);
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
  // Calls `fn` with the params of every `method` event from now on.
  const on = (method, fn) => listeners.push([method, fn]);
  // `expression` evaluated in the page, awaited, by value. An exception it
  // throws, or a hang, fails the caller.
  const evaluate = async (expression) => {
    const r = await within(
      send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }),
      expression.slice(0, 40),
    );
    if (r.result.exceptionDetails) throw new Error(`${expression.slice(0, 60)}: ${r.result.exceptionDetails.text}`);
    return r.result.result.value;
  };
  // Loads `url` and waits for its load event; a navigation error throws.
  const navigate = async (url) => {
    const loaded = next('Page.loadEventFired');
    const r = await send('Page.navigate', { url });
    if (r.result?.errorText) throw new Error(`${url}: ${r.result.errorText}`);
    await within(loaded, `${url} to load`);
  };
  // A trusted press and release of one of `KEYS`.
  const press = async (name) => {
    const { text, ...key } = KEYS[name];
    await send('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', ...key, ...(text ? { text } : {}) });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
  };
  return { send, next, on, close, evaluate, navigate, press };
}

// The keys the scripts press, as trusted key events describe them. A key
// with `text` is typed (keyDown); one without is a raw key.
export const KEYS = {
  Tab: { key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 },
  Enter: { key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' },
  Space: { key: ' ', code: 'Space', windowsVirtualKeyCode: 32, text: ' ' },
  Shift: { key: 'Shift', code: 'ShiftLeft', windowsVirtualKeyCode: 16 },
  ShiftTab: { key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9, modifiers: 8 },
  Escape: { key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 },
};

// Page-side source: the arrangement on show (FR-69 hides the other).
export const SHOWN = `[...document.querySelectorAll('.arrangement')].find((a) => getComputedStyle(a).display !== 'none')`;

// Page-side source: the theme toggle's icon on show (rg-1ru); the palette
// blocks pick it, so it is the one the stylesheet displays.
export const SHOWN_ICON = `[...document.querySelectorAll('.theme-toggle svg')].find((s) => getComputedStyle(s).display !== 'none')`;

// Page-side source: the text of every element `selector` matches whose
// visible text spills out of the element's box by more than `slack`.
// Visually hidden text is laid out unwrapped in a 1px box by design, so
// only the text a reader sees is measured.
export const SPILLS = `(selector, slack) => {
  const out = [];
  for (const cell of document.querySelectorAll(selector)) {
    const box = cell.getBoundingClientRect();
    const walk = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
    let left = Infinity, right = -Infinity, bottom = -Infinity;
    for (let node = walk.nextNode(); node; node = walk.nextNode()) {
      if (node.parentElement.closest('.visually-hidden')) continue;
      const range = document.createRange();
      range.selectNodeContents(node);
      for (const r of range.getClientRects()) {
        left = Math.min(left, r.left);
        right = Math.max(right, r.right);
        bottom = Math.max(bottom, r.bottom);
      }
    }
    if (bottom > box.bottom + slack || right > box.right + slack || left < box.left - slack) {
      out.push(cell.textContent.slice(0, 40));
    }
  }
  return out;
}`;
