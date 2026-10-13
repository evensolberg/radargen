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
import { KEYS, connect, poll } from './cdp.mjs';

const WIDE = 1400;
const NARROW = 400;

const [file] = process.argv.slice(2);
if (!file) {
  console.error('usage: node scripts/interaction.mjs <page.html>');
  process.exit(2);
}
const url = pathToFileURL(resolve(file)).href;

const { send, on, close, evaluate, navigate, press } = await connect();
const thrown = [];
on('Runtime.exceptionThrown', (p) => thrown.push(p.exceptionDetails.exception?.description ?? p.exceptionDetails.text));
// A dialog means CSV text ran as script (NFR-6): fail, and dismiss it so
// the run goes on rather than hanging.
on('Page.javascriptDialogOpening', (p) => {
  thrown.push(`a dialog opened: ${p.message}`);
  send('Page.handleJavaScriptDialog', { accept: false });
});
const requests = [];
on('Network.requestWillBeSent', (p) => requests.push(p.request.url));

const click = async (x, y) => {
  for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
    await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
  }
};

const resize = (width, height = 900) =>
  send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });

// A fresh page at `width`, with the colour scheme and script switch set
// before it loads, so the script's start-up reads them.
async function load(width, { scheme = 'light', script = true, height = 900 } = {}) {
  await send('Emulation.setScriptExecutionDisabled', { value: !script });
  await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: scheme }] });
  await resize(width, height);
  requests.length = 0;
  await navigate(url);
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
    text: a.textContent.trim().slice(0, 60),
    arrangement: arrangement && (arrangement.classList.contains('wide') ? 'wide' : 'narrow'),
  };
})()`;
const active = () => evaluate(ACTIVE);
// Presses `key` until the focused element passes `want`, and returns it.
async function tabTo(want, what, key = 'Tab') {
  for (let i = 0; i < 80; i++) {
    await press(key);
    const a = await active();
    if (want(a)) return a;
  }
  throw new Error(`never reached ${what}`);
}
const tabToMarker = (n) => tabTo((a) => a.marker && a.label?.startsWith(`${n}. `), `marker ${n}`);
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

// The keys carrying aria-expanded="true" and data-selected, and how many
// selected elements are SVG controls (markers and legend rows).
const SETS = `(() => {
  const keys = (sel) => [...document.querySelectorAll(sel)].map((e) => e.getAttribute('data-row') || e.getAttribute('data-marker-indicator'));
  return {
    expanded: keys('[aria-expanded="true"]'),
    selected: keys('[data-selected]'),
    markers: document.querySelectorAll('[data-marker][data-selected],[data-legend-row][data-selected]').length,
  };
})()`;
// Whether the element `selector` matches has computed visibility visible.
const visible = (selector) => `getComputedStyle(document.querySelector('${selector}')).visibility === 'visible'`;
const panelHeading = () => evaluate(`document.querySelector('#detail .detail-heading').textContent`);
const inView = (element) => evaluate(`(() => { const r = (${element}).getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; })()`);
// The centre of the first element `selector` matches, scrolled into view,
// with its key.
const centreOf = (selector) => evaluate(`(() => {
  const e = document.querySelector('${selector.replace(/'/g, "\\'")}');
  e.scrollIntoView({ block: 'center' });
  const r = e.getBoundingClientRect();
  return { x: r.x + r.width / 2, y: r.y + r.height / 2, key: e.getAttribute('data-row') };
})()`);
const move = (x, y) => send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
// Clicks the centre of the first element `selector` matches; returns it.
const clickOn = async (selector) => {
  const at = await centreOf(selector);
  await click(at.x, at.y);
  return at;
};
const panelHidden = () => evaluate(`document.getElementById('detail').hidden`);
// Opens the table, then the panel of the row on CSV line `line`, by its
// button: by line, since the fixture repeats a name.
const openRow = async (line) => {
  await evaluate(`document.querySelector('details.table-view').open = true`);
  return clickOn(`tr[data-row="${line}"] button`);
};

const failures = [];
const check = (ok, what) => {
  if (!ok) failures.push(what);
};
// `ready` becomes truthy within the timeout, or `what` is a failure.
const eventually = (ready, what) => poll(ready, what).catch((e) => void failures.push(e.message));
const PRESSED = `document.querySelector('.theme-toggle').getAttribute('aria-pressed')`;
async function scenario(name, widths, body) {
  for (const width of widths) {
    const before = failures.length;
    thrown.length = 0;
    try {
      await body(width);
    } catch (error) {
      failures.push(error.message);
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
    const shown = width === WIDE ? 'wide' : 'narrow';
    const markers = await evaluate(`document.querySelectorAll('.arrangement.${shown} [data-marker]').length`);
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
    check(n > 0 && n === markers, `every marker is reached: ${n} of ${markers}`);
    check((await active()).legend !== null, 'legend rows follow the markers');
  });

  // A mouse click focuses without a ring; the next key press shows it.
  await scenario('mouse focus, then a key', [WIDE], async (width) => {
    await load(width);
    const at = await evaluate(`(() => {
      const m = [...document.querySelectorAll('.arrangement.wide [data-marker]')][0];
      const r = m.querySelector('circle').getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    })()`);
    // A click opens the panel; a second closes it, leaving focus on the
    // marker, so focus came from the mouse alone.
    await click(at.x, at.y);
    await click(at.x, at.y);
    check((await active()).marker !== null, 'the clicks focused a marker');
    check(!(await evaluate(RING)).focused, 'no ring after a mouse click');
    await press('Shift');
    await eventually(async () => (await evaluate(RING)).focused || undefined, 'the ring after a key press');
  });

  // 9b. A breakpoint change moves focus to the same item (architecture §5).
  await scenario('breakpoint keeps focus on the item', [WIDE], async (width) => {
    await load(width);
    const a = await tabToMarker(3);
    check(a.arrangement === 'wide', 'wide marker 3 is focused');
    await resize(NARROW);
    const moved = await eventually(async () => {
      const b = await active();
      return b.arrangement === 'narrow' && b.marker === a.marker ? b : undefined;
    }, 'focus on narrow marker 3');
    if (moved) {
      const ring = await evaluate(RING);
      check(ring.focused && ring.visible, 'its ring shows');
    }
  });

  // A reader who left the marker is not pulled back by a later resize.
  await scenario('a resize after leaving a marker', [WIDE], async (width) => {
    await load(width);
    await press('Tab');
    await press('Tab');
    await press('Tab');
    check((await active()).marker !== null, 'a marker is focused');
    await click(5, 5);
    check((await active()).tag === 'body', 'a click on blank page leaves it');
    await resize(NARROW);
    await evaluate('new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))');
    check((await active()).tag === 'body', `focus stays where the reader left it: ${JSON.stringify(await active())}`);
  });

  // FR-56: the toggle follows the viewer's preference until pressed.
  await scenario('theme toggle', [WIDE], async (width) => {
    for (const scheme of ['dark', 'light']) {
      await load(width, { scheme });
      const pressed = await evaluate(PRESSED);
      check(pressed === String(scheme === 'dark'), `${scheme}: aria-pressed ${pressed}`);
    }
    await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'dark' }] });
    await eventually(
      async () => (await evaluate(PRESSED)) === 'true' || undefined,
      'aria-pressed follows a live change of preference',
    );
    const ground = () => evaluate(`getComputedStyle(document.body).backgroundColor`);
    const before = await ground();
    await press('Tab');
    await press('Tab');
    check((await active()).cls?.includes('theme-toggle'), 'the toggle is reached');
    await press('Enter');
    check((await evaluate(`document.documentElement.getAttribute('data-theme')`)) === 'light', 'pressing it sets light');
    check((await evaluate(PRESSED)) === 'false', 'and unpresses');
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
    await tabTo((x) => x.tag === 'summary', 'the table toggle');
    await press('Enter');
    check(await evaluate(`document.querySelector('details.table-view').open`), 'Enter opens the table');
  });

  // NFR-5: the page requests nothing but itself.
  await scenario('no requests', [WIDE, NARROW], async (width) => {
    await load(width);
    await tabToMarker(1);
    await press('Enter');
    const fetched = requests.filter((u) => !u.startsWith('data:'));
    check(fetched.length === 1 && fetched[0] === url, `only the document, panel open: ${JSON.stringify(fetched)}`);
  });

  // ---- Part 2: the panel, selection, hover and keyboard (RG-015B) ----

  // P1. Open: focus in the panel, exact expanded and selected sets.
  await scenario('P1 open', [WIDE, NARROW], async (width) => {
    await load(width);
    const m = await tabToMarker(3);
    await press('Enter');
    const a = await active();
    check(a.cls === 'detail-heading', `focus on the panel heading, got ${JSON.stringify(a)}`);
    check((await panelHeading()).startsWith('3. '), 'the heading names item 3');
    check(await inView('document.activeElement'), 'the heading is on screen');
    const sets = await evaluate(SETS);
    check(sets.expanded.length === 5 && sets.expanded.every((k) => k === m.marker), `aria-expanded on five controls of key ${m.marker}: ${JSON.stringify(sets.expanded)}`);
    check(sets.selected.length === 8 && sets.selected.every((k) => k === m.marker), `data-selected on eight elements of key ${m.marker}: ${JSON.stringify(sets.selected)}`);
    const shown = width === WIDE ? 'wide' : 'narrow';
    check(await evaluate(visible(`.arrangement.${shown} [data-marker-indicator="${m.marker}"] [data-selection]`)), 'the square shows');
    check(!(await evaluate(visible(`.arrangement.${shown} [data-marker-indicator="${m.marker}"] [data-focus-ring]`))), 'the ring does not, focus being in the panel');
  });

  // P1b. Swapping items leaves only the new one.
  await scenario('P1b swap', [WIDE], async (width) => {
    await load(width);
    await tabToMarker(3);
    await press('Enter');
    const row = await centreOf(`.arrangement.wide [data-legend-row][aria-label^="5. "]`);
    await click(row.x, row.y);
    const key = row.key;
    const sets = await evaluate(SETS);
    check(sets.expanded.every((k) => k === key) && sets.selected.every((k) => k === key) && sets.selected.length === 8, `only key ${key}: ${JSON.stringify(sets)}`);
    const fields = await evaluate(`(() => {
      const row = document.querySelector('tr[data-row="${key}"]');
      const cells = [...row.children];
      const at = cells.findIndex((c) => c.tagName === 'TH');
      return [cells.slice(at + 1).filter((c) => c.textContent.trim()).length, document.querySelectorAll('#detail dt').length];
    })()`);
    check(fields[0] === fields[1], `the panel holds item 5's fields only: ${fields}`);
    await press('Escape');
    check((await active()).legend === key, 'Escape returns to legend row 5');
  });

  // P2. Closing: Escape, Space without scrolling, Close, a second click.
  await scenario('P2 close', [WIDE], async (width) => {
    await load(width);
    const m = await tabToMarker(3);
    await press('Enter');
    await press('Escape');
    const a = await active();
    check(a.marker === m.marker && a.arrangement === 'wide', 'Escape returns to marker 3');
    check(await inView('document.activeElement'), 'marker 3 on screen');
    check((await evaluate(SETS)).selected.length === 0, 'nothing selected');
    const row = await tabTo((x) => x.legend && x.label?.startsWith('2. '), 'legend row 2');
    const before = await evaluate('scrollY');
    await press('Space');
    check(!(await panelHidden()), 'Space opens the panel');
    check((await evaluate('scrollY')) === before, 'Space does not scroll the page');
    await press('Tab');
    check((await active()).cls === 'detail-close', 'Tab reaches Close');
    await press('Enter');
    check((await active()).legend === row.legend, 'Close returns to legend row 2');
    const at = await centreOf(`.arrangement.wide [data-legend-row="${row.legend}"]`);
    await click(at.x, at.y);
    check(!(await panelHidden()), 'a click opens it');
    await click(at.x, at.y);
    check(await panelHidden(), 'a second click closes it');
  });

  // P3. The table, with Enter and with Space, and P3b, a closed table.
  await scenario('P3 table', [WIDE], async (width) => {
    for (const key of ['Enter', 'Space']) {
      await load(width);
      const summary = await centreOf('details.table-view summary');
      await click(summary.x, summary.y);
      await tabTo((a) => a.tag === 'button' && a.text?.startsWith('Legacy Bus'), 'the unplotted row');
      await press(key);
      check((await panelHeading()).endsWith('not on the radar'), `${key}: the unplotted item's panel`);
      const sets = await evaluate(SETS);
      check(sets.markers === 0, `${key}: no marker or legend row selected`);
      await press('Escape');
      const a = await active();
      check(a.tag === 'button' && a.text?.startsWith('Legacy Bus'), `${key}: Escape returns to its button`);
    }
    await load(width);
    const summary = await centreOf('details.table-view summary');
    await click(summary.x, summary.y);
    await tabTo((a) => a.tag === 'button' && a.text?.startsWith('Legacy Bus'), 'the unplotted row');
    await press('Enter');
    const again = await centreOf('details.table-view summary');
    await click(again.x, again.y);
    check(!(await evaluate(`document.querySelector('details.table-view').open`)), 'the table closes');
    const head = await centreOf('#detail .detail-heading');
    await click(head.x, head.y);
    await press('Escape');
    check((await active()).tag === 'summary', 'with the table closed, focus returns to its toggle');
  });

  // P4. Two rows share a name; each opens and selects only its own key.
  await scenario('P4 duplicate names', [WIDE], async (width) => {
    await load(width);
    const keys = await evaluate(`[...document.querySelectorAll('tr[data-row]')].filter((r) => r.querySelector('th').textContent.startsWith('Kafka')).map((r) => r.getAttribute('data-row'))`);
    check(keys.length === 2, `two Kafka rows: ${keys}`);
    const headings = [];
    for (const key of keys) {
      await openRow(key);
      const sets = await evaluate(SETS);
      check(sets.selected.every((k) => k === key), `key ${key} alone: ${JSON.stringify(sets)}`);
      headings.push(await panelHeading());
    }
    check(headings[0] !== headings[1], `two panels: ${headings}`);
  });

  // P5. Hostile content stays text; only https is a link.
  await scenario('P5 hostile content', [WIDE], async (width) => {
    await load(width);
    await evaluate(`window.__alerts = 0; window.alert = () => { window.__alerts += 1; }`);
    await openRow(6);
    const spark = await evaluate(`(() => { const p = document.getElementById('detail'); return [p.querySelectorAll('img').length, [...p.querySelectorAll('dd')].map((d) => d.textContent)]; })()`);
    check(spark[0] === 0, 'no image in the panel');
    check(spark[1].includes('<img src=x onerror=alert(1)>'), `the description as text: ${spark[1]}`);
    await openRow(4);
    const flink = await evaluate(`(() => { const p = document.getElementById('detail'); return [p.querySelectorAll('a').length, p.textContent]; })()`);
    check(flink[0] === 0 && flink[1].includes('javascript:alert(1)'), 'a refused link is text');
    await openRow(2);
    const kafka = await evaluate(`(() => { const a = document.querySelector('#detail a'); const prose = [...document.querySelectorAll('#detail dd.prose')].find((d) => d.textContent.includes('replay')); return a && [a.getAttribute('href'), a.target, a.rel, prose?.textContent]; })()`);
    check(kafka && kafka[0] === 'https://kafka.apache.org' && kafka[1] === '_blank' && kafka[2] === 'noopener noreferrer', `the https link: ${kafka}`);
    check(kafka?.[3]?.includes('\n'), 'prose keeps its line break');
    check((await evaluate('window.__alerts')) === 0, 'nothing ran');
  });

  // P6. Hover lights the other half, ends off a control, and a tap sets none.
  await scenario('P6 hover', [WIDE], async (width) => {
    await load(width);
    const m = await centreOf('.arrangement.wide [data-marker]');
    await move(m.x, m.y);
    check(await evaluate(`document.querySelector('.arrangement.wide [data-legend-row="${m.key}"]').hasAttribute('data-hovered')`), 'the marker lights its legend row');
    await move(5, 5);
    check((await evaluate(`document.querySelectorAll('[data-hovered]').length`)) === 0, 'leaving clears it');
    const r = await centreOf(`.arrangement.wide [data-legend-row="${m.key}"]`);
    await move(r.x, r.y);
    check(await evaluate(`document.querySelector('.arrangement.wide [data-marker-indicator="${m.key}"]').hasAttribute('data-hovered')`), 'the legend row lights its marker');
    await move(5, 5);
    await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x: m.x, y: m.y }] });
    await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    check((await evaluate(`document.querySelectorAll('[data-hovered]').length`)) === 0, 'a tap leaves no hover');
    await press('Escape');
    const other = await centreOf('.arrangement.wide [data-marker]:not([data-marker="' + m.key + '"])');
    await tabTo((a) => a.marker === m.key, 'the first marker');
    await press('Enter');
    await move(other.x, other.y);
    const dash = (k) => evaluate(`getComputedStyle(document.querySelector('.arrangement.wide [data-marker-indicator="${k}"] [data-selection] rect')).strokeDasharray`);
    check((await dash(other.key)) !== (await dash(m.key)), 'hovered and selected look different');
  });

  // P7. A hit area's edge opens its marker.
  await scenario('P7 hit area', [WIDE], async (width) => {
    await load(width);
    const at = await evaluate(`(() => {
      const hit = document.querySelector('.arrangement.wide [data-hit]');
      const key = hit.getAttribute('data-hit');
      const disc = document.querySelector('.arrangement.wide [data-marker="' + key + '"] circle').getBoundingClientRect();
      const box = hit.getBoundingClientRect();
      return { key, x: box.x + box.width / 2 + (disc.width / 2 + box.width / 2) / 2, y: box.y + box.height / 2,
        label: document.querySelector('.arrangement.wide [data-marker="' + key + '"]').getAttribute('aria-label') };
    })()`);
    await click(at.x, at.y);
    const number = at.label.split('.')[0];
    check((await panelHeading()).startsWith(number + '. '), `the hit area's marker opens: ${await panelHeading()}`);
  });

  // P8. A breakpoint change with the panel open.
  await scenario('P8 breakpoint, panel open', [WIDE], async (width) => {
    await load(width);
    const m = await tabToMarker(3);
    await press('Enter');
    await resize(NARROW);
    await eventually(async () => (await evaluate(`getComputedStyle(document.querySelector('.arrangement.narrow')).display`)) !== 'none' || undefined, 'the narrow arrangement shows');
    check(await evaluate(`document.querySelector('.arrangement.narrow [data-marker="${m.marker}"]').hasAttribute('data-selected')`), 'the narrow marker is selected');
    await press('Escape');
    const a = await active();
    check(a.arrangement === 'narrow' && a.marker === m.marker, `focus on the narrow marker: ${JSON.stringify(a)}`);
    check((await evaluate(SETS)).selected.length === 0, 'nothing selected anywhere');
  });

  // P9. The sticky panel keeps the radar in view and never hides focus.
  await scenario('P9 sticky panel', [WIDE], async (width) => {
    await load(width);
    const m = await tabToMarker(3);
    await press('Enter');
    check(await inView(`document.querySelector('.arrangement.wide [data-marker="${m.marker}"]')`), 'the selected marker stays on screen');
    const pad = await evaluate(`[document.documentElement.style.scrollPaddingBottom, document.getElementById('detail').offsetHeight + 'px']`);
    check(pad[0] === pad[1], `scroll padding is the panel's height: ${pad}`);
    for (let i = 0; i < 4; i++) {
      await press('ShiftTab');
      const hidden = await evaluate(`(() => {
        const r = document.activeElement.getBoundingClientRect();
        const p = document.getElementById('detail').getBoundingClientRect();
        return r.bottom > p.top + 0.5 && r.top < p.bottom;
      })()`);
      check(!hidden, `Shift+Tab ${i + 1}: the focused control is not under the panel`);
    }
  });

  // P10. A focused, selected legend row shows the outline and the bar.
  await scenario('P10 focused and selected', [WIDE], async (width) => {
    await load(width);
    const last = await evaluate(`(() => { const rows = document.querySelectorAll('.arrangement.wide [data-legend-row]'); return rows[rows.length - 1].getAttribute('data-legend-row'); })()`);
    const at = await centreOf(`.arrangement.wide [data-legend-row="${last}"]`);
    await click(at.x, at.y);
    await press('ShiftTab');
    check((await active()).legend === last, 'focus back on the selected row');
    const marks = await evaluate(`(() => {
      const row = document.querySelector('.arrangement.wide [data-legend-row="${last}"]');
      const ring = getComputedStyle(row.querySelector('[data-row-indicator] rect'));
      const probe = document.createElement('div');
      probe.style.color = 'var(--focus)';
      document.body.append(probe);
      const focus = getComputedStyle(probe).color;
      probe.remove();
      return [ring.stroke, ring.strokeDasharray, getComputedStyle(row.querySelector('[data-selection]')).visibility, focus];
    })()`);
    check(marks[0] === marks[3] && marks[1] === 'none' && marks[2] === 'visible', `a solid outline in the focus colour, and the bar: ${marks}`);
  });

  // P11. Negatives.
  await scenario('P11 negatives', [WIDE], async (width) => {
    await load(width);
    await press('Tab');
    await press('Escape');
    check((await active()).cls === 'skip', 'Escape with no panel changes nothing');
    await tabToMarker(1);
    await press('Enter');
    await tabTo((a) => a.cls?.includes('theme-toggle'), 'the toggle', 'ShiftTab');
    await press('Escape');
    check(!(await panelHidden()), 'Escape on the toggle leaves the panel open');
    const head = await centreOf('#detail .detail-heading');
    await click(head.x, head.y);
    const dl = await centreOf('#detail dl');
    await click(dl.x, dl.y);
    check(!(await panelHidden()), 'clicks inside the panel do not close it');
    await press('Escape');
    await tabToMarker(2);
    await send('Input.dispatchKeyEvent', { type: 'keyDown', ...KEYS.Enter });
    await send('Input.dispatchKeyEvent', { type: 'keyDown', ...KEYS.Enter, autoRepeat: true });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', ...KEYS.Enter });
    check(!(await panelHidden()), 'a held Enter opens once and stays open');
  });
  // P12. Opened from deep in a long table, the panel comes into view; the
  // sticky rule alone never brings it down from above.
  await scenario('P12 opened from deep in the table', [WIDE, NARROW], async (width) => {
    await load(width, { height: 300 });
    const last = await evaluate(`[...document.querySelectorAll('tr[data-row]')].pop().getAttribute('data-row')`);
    await openRow(last);
    check(await inView(`document.querySelector('#detail .detail-heading')`), 'the panel heading is on screen');
  });

  // P13. Escape on one of the open item's own controls closes the panel
  // and leaves focus there, rather than jumping to the opener.
  await scenario('P13 Escape on another of the item\'s controls', [WIDE], async (width) => {
    await load(width);
    await evaluate(`document.querySelector('details.table-view').open = true`);
    await tabToMarker(3);
    await press('Enter');
    const name = (await panelHeading()).replace(/^3\. /, '');
    await tabTo((a) => a.tag === 'button' && a.text?.startsWith(name), `the table button for ${name}`);
    await press('Escape');
    check(await panelHidden(), 'the panel closes');
    const a = await active();
    check(a.tag === 'button' && a.text?.startsWith(name), `focus stays on the table button: ${JSON.stringify(a)}`);
  });

  // P14. Space released on another control activates neither.
  await scenario('P14 Space moved between controls', [WIDE], async (width) => {
    await load(width);
    await tabToMarker(1);
    const { text, ...space } = KEYS.Space;
    await send('Input.dispatchKeyEvent', { type: 'keyDown', ...space, text });
    await press('Tab');
    await send('Input.dispatchKeyEvent', { type: 'keyUp', ...space });
    check(await panelHidden(), 'no panel opens');
  });

  // P15. Re-activating an item through its hit area keeps focus on its marker.
  await scenario('P15 hit area twice', [WIDE], async (width) => {
    await load(width);
    const at = await evaluate(`(() => {
      const hit = document.querySelector('.arrangement.wide [data-hit]');
      const key = hit.getAttribute('data-hit');
      const disc = document.querySelector('.arrangement.wide [data-marker="' + key + '"] circle').getBoundingClientRect();
      const box = hit.getBoundingClientRect();
      return { key, x: box.x + box.width / 2 + (disc.width / 2 + box.width / 2) / 2, y: box.y + box.height / 2 };
    })()`);
    await click(at.x, at.y);
    await click(at.x, at.y);
    check(await panelHidden(), 'the second click closes it');
    check((await active()).marker === at.key, 'focus is on the marker, not the page');
  });
} finally {
  await close();
}
process.exit(failures.length === 0 ? 0 : 1);
