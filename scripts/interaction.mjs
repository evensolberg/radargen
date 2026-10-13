// The interactive layer, driven in a browser (plan RG-015B).
//
// Usage: node scripts/interaction.mjs <page.html>
//
// Each scenario loads the page afresh, at the widths it names, and drives
// it with trusted input only (Input.dispatchKeyEvent and
// dispatchMouseEvent), never .focus() or dispatchEvent, since those skip
// :focus-visible and the browser's own activation. It waits by polling,
// never by sleeping, and an exception thrown in the page fails the run.
// Exits non-zero on any failure.

import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { connect, poll, within } from './cdp.mjs';

const WIDE = 1400;
const NARROW = 400;

const [file] = process.argv.slice(2);
if (!file) {
  console.error('usage: node scripts/interaction.mjs <page.html>');
  process.exit(2);
}
const url = pathToFileURL(resolve(file)).href;

const { send, next, on, close } = await connect();
const thrown = [];
on('Runtime.exceptionThrown', (p) => thrown.push(p.exceptionDetails.exception?.description ?? p.exceptionDetails.text));
const requests = [];
on('Network.requestWillBeSent', (p) => requests.push(p.request.url));

const evaluate = async (expression) => {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.result.exceptionDetails) throw new Error(`${expression}: ${r.result.exceptionDetails.text}`);
  return r.result.result.value;
};

const KEYS = {
  Tab: { key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 },
  Enter: { key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r' },
  Space: { key: ' ', code: 'Space', windowsVirtualKeyCode: 32, text: ' ' },
  Shift: { key: 'Shift', code: 'ShiftLeft', windowsVirtualKeyCode: 16 },
};
const press = async (name) => {
  const { text, ...k } = KEYS[name];
  await send('Input.dispatchKeyEvent', { type: text ? 'keyDown' : 'rawKeyDown', ...k, ...(text ? { text } : {}) });
  await send('Input.dispatchKeyEvent', { type: 'keyUp', ...k });
};
const click = async (x, y) => {
  for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
    await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
  }
};

// A fresh page at `width`, with the colour scheme and script switch set
// before it loads, so the script's start-up reads them.
async function load(width, { scheme = 'light', script = true } = {}) {
  await send('Emulation.setScriptExecutionDisabled', { value: !script });
  await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: scheme }] });
  await send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
  requests.length = 0;
  const loaded = next('Page.loadEventFired');
  await send('Page.navigate', { url });
  await within(loaded, 'the page to load');
}

// What the focused element is, as the scenarios compare it.
const ACTIVE = `(() => {
  const a = document.activeElement;
  const arrangement = a.closest('.arrangement');
  return {
    tag: a.tagName.toLowerCase(),
    cls: a.getAttribute('class'),
    marker: a.getAttribute('data-marker'),
    legend: a.getAttribute('data-legend-row'),
    label: a.getAttribute('aria-label'),
    arrangement: arrangement && (arrangement.classList.contains('wide') ? 'wide' : 'narrow'),
  };
})()`;
const active = () => evaluate(ACTIVE);
// Whether the focused marker's ring is shown, as the page computes it.
const RING = `(() => {
  const a = document.activeElement;
  const key = a.getAttribute('data-marker');
  const ind = a.closest('svg').querySelector('[data-marker-indicator="' + key + '"]');
  const ring = ind && ind.querySelector('[data-focus-ring]');
  return {
    focused: !!ind && ind.hasAttribute('data-focused'),
    visible: !!ring && getComputedStyle(ring).visibility === 'visible',
    last: !!ind && ind.parentNode.lastElementChild === ind,
  };
})()`;

const failures = [];
const check = (ok, what) => {
  if (!ok) failures.push(what);
};
async function scenario(name, widths, body) {
  for (const width of widths) {
    const before = failures.length;
    thrown.length = 0;
    try {
      await body(width);
    } catch (error) {
      failures.push(`${error.message}`);
    }
    for (const t of thrown) failures.push(`page threw: ${t}`);
    const mine = failures.splice(before);
    console.log(`${mine.length ? 'FAIL' : 'ok'} ${name} @${width}px`);
    for (const f of mine) console.log('    ' + f);
    failures.push(...mine.map((m) => `${name} @${width}: ${m}`));
  }
}

await send('Page.enable');
await send('Runtime.enable');
await send('Network.enable');
await send('Emulation.setFocusEmulationEnabled', { enabled: true });

try {
  // 1. Tab order (FR-34, FR-98), and each marker's ring (rg-jri).
  await scenario('tab order and rings', [WIDE, NARROW], async (width) => {
    await load(width);
    await press('Tab');
    check((await active()).cls === 'skip', 'first stop is the skip link');
    await press('Tab');
    check((await active()).cls?.includes('theme-toggle'), 'second stop is the theme toggle');
    const markers = await evaluate(`document.querySelectorAll('.arrangement:not([style*=none]) [data-marker]').length / 1`);
    let n = 0;
    for (;;) {
      await press('Tab');
      const a = await active();
      if (!a.marker) break;
      n += 1;
      check(a.label?.startsWith(`${n}. `), `marker ${n} in number order, got ${a.label}`);
      const ring = await evaluate(RING);
      check(ring.focused && ring.visible, `marker ${n} shows its ring`);
      check(ring.last, `marker ${n}'s indicator is last in its layer`);
    }
    check(n > 0, 'markers are reached');
    check((await active()).legend !== null, 'legend rows follow the markers');
    void markers;
  });

  // A mouse click focuses without a ring; the next key press shows it.
  await scenario('mouse focus, then a key', [WIDE], async (width) => {
    await load(width);
    const at = await evaluate(`(() => {
      const m = [...document.querySelectorAll('.arrangement.wide [data-marker]')][0];
      const r = m.querySelector('circle').getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    })()`);
    await click(at.x, at.y);
    check((await active()).marker !== null, 'the click focused a marker');
    check(!(await evaluate(RING)).focused, 'no ring after a mouse click');
    await press('Shift');
    await poll(async () => (await evaluate(RING)).focused || undefined, 'the ring after a key press').catch((e) =>
      failures.push(e.message),
    );
  });

  // 9b. A breakpoint change moves focus to the same item (architecture §5).
  await scenario('breakpoint keeps focus on the item', [WIDE], async (width) => {
    await load(width);
    let a;
    for (let i = 0; i < 6; i++) {
      await press('Tab');
      a = await active();
      if (a.label?.startsWith('3. ')) break;
    }
    check(a.label?.startsWith('3. ') && a.arrangement === 'wide', 'wide marker 3 is focused');
    await send('Emulation.setDeviceMetricsOverride', { width: NARROW, height: 900, deviceScaleFactor: 1, mobile: false });
    const moved = await poll(async () => {
      const b = await active();
      return b.arrangement === 'narrow' && b.marker === a.marker ? b : undefined;
    }, 'focus on narrow marker 3').catch((e) => void failures.push(e.message));
    if (moved) {
      const ring = await evaluate(RING);
      check(ring.focused && ring.visible, 'its ring shows');
    }
  });

  // FR-56: the toggle follows the viewer's preference until pressed.
  await scenario('theme toggle', [WIDE], async (width) => {
    for (const scheme of ['dark', 'light']) {
      await load(width, { scheme });
      const pressed = await evaluate(`document.querySelector('.theme-toggle').getAttribute('aria-pressed')`);
      check(pressed === String(scheme === 'dark'), `${scheme}: aria-pressed ${pressed}`);
    }
    await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'dark' }] });
    await poll(
      async () => (await evaluate(`document.querySelector('.theme-toggle').getAttribute('aria-pressed')`)) === 'true' || undefined,
      'aria-pressed follows a live change of preference',
    ).catch((e) => failures.push(e.message));
    const ground = () => evaluate(`getComputedStyle(document.body).backgroundColor`);
    const before = await ground();
    await press('Tab');
    await press('Tab');
    check((await active()).cls?.includes('theme-toggle'), 'the toggle is reached');
    await press('Enter');
    check((await evaluate(`document.documentElement.getAttribute('data-theme')`)) === 'light', 'pressing it sets light');
    check((await evaluate(`document.querySelector('.theme-toggle').getAttribute('aria-pressed')`)) === 'false', 'and unpresses');
    check((await ground()) !== before, 'the background changes');
  });

  // FR-98: visible when focused, and it reaches the legend.
  await scenario('skip link', [WIDE, NARROW], async (width) => {
    await load(width);
    await press('Tab');
    const box = await evaluate(`document.activeElement.getBoundingClientRect().width`);
    check(box > 20, `the focused skip link is visible (${box}px wide)`);
    await press('Enter');
    const a = await active();
    const want = width === WIDE ? 'wide' : 'narrow';
    check(a.legend !== null && a.arrangement === want, `focus on a ${want} legend row, got ${JSON.stringify(a)}`);
    const first = await evaluate(
      `document.querySelector('.arrangement.${want} [data-legend-row]').getAttribute('data-legend-row')`,
    );
    check(a.legend === first, 'the first legend row');
  });

  // Without script nothing claims what it cannot do, focus stays visible,
  // and the table still opens.
  await scenario('no script', [WIDE, NARROW], async (width) => {
    await load(width, { script: false });
    for (const sel of ['.theme-toggle', '.skip']) {
      check((await evaluate(`getComputedStyle(document.querySelector('${sel}')).display`)) === 'none', `${sel} is not rendered`);
    }
    await press('Tab');
    const a = await active();
    check(a.marker !== null, 'the first stop is a marker');
    const outline = await evaluate(
      `(() => { const s = getComputedStyle(document.activeElement); return [s.outlineStyle, s.outlineColor]; })()`,
    );
    check(outline[0] !== 'none' && outline[1] !== 'rgba(0, 0, 0, 0)', `the native outline shows: ${outline}`);
    for (let i = 0; i < 200 && (await active()).tag !== 'summary'; i++) await press('Tab');
    check((await active()).tag === 'summary', 'the table toggle is reached');
    await press('Enter');
    check(await evaluate(`document.querySelector('details.table-view').open`), 'Enter opens the table');
  });

  // NFR-5: the page requests nothing but itself.
  await scenario('no requests', [WIDE, NARROW], async (width) => {
    await load(width);
    const fetched = requests.filter((u) => !u.startsWith('data:'));
    check(fetched.length === 1 && fetched[0] === url, `only the document: ${JSON.stringify(fetched)}`);
  });
} finally {
  await close();
}
process.exit(failures.length === 0 ? 0 : 1);
