// WCAG 2.1 AA's automatable criteria, audited in a browser (plan RG-015C).
//
// Usage: node scripts/a11y-audit.mjs <page.html> <d71.html> <d71-narrow-width>
//
// The first page is the interaction fixture; the second is D-71's, whose
// narrow arrangement is wider than 480 units, so its narrow targets fall
// below 24 px at 320 px by design: the one recorded 2.5.8 exception.
//
// Controls are found from the DOM and then looked up in Chromium's
// accessibility tree, so a control that lost its role is caught, not
// skipped. The audit uses trusted input only, waits for the script and
// the fonts, and fails on any page exception or dialog. Each numbered
// check names the WCAG criterion it backs, as RG-015C's table does.

import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { SPILLS, connect, poll } from './cdp.mjs';

const [page, d71, d71Width] = process.argv.slice(2);
if (!page || !d71 || !d71Width) {
  console.error('usage: node scripts/a11y-audit.mjs <page.html> <d71.html> <d71-narrow-width>');
  process.exit(2);
}
const W71 = Number(d71Width);
const TOLERANCE = 0.05;
// FR-94: a hit area's and a legend row's least size, in user units.
const TARGET_UNITS = 36;

const { send, on, close, evaluate, navigate, press } = await connect();
const thrown = [];
on('Runtime.exceptionThrown', (p) => thrown.push(p.exceptionDetails.exception?.description ?? p.exceptionDetails.text));
on('Page.javascriptDialogOpening', (p) => {
  thrown.push(`a dialog opened: ${p.message}`);
  send('Page.handleJavaScriptDialog', { accept: false });
});
await send('Page.enable');
await send('Runtime.enable');
await send('Accessibility.enable');
await send('Emulation.setFocusEmulationEnabled', { enabled: true });

const failures = [];
const check = (ok, what) => {
  if (!ok) failures.push(what);
};

async function load(file, width, { scheme = 'light', forced = false, script = true } = {}) {
  await send('Emulation.setScriptExecutionDisabled', { value: !script });
  await send('Emulation.setEmulatedMedia', {
    features: [
      { name: 'prefers-color-scheme', value: scheme },
      { name: 'forced-colors', value: forced ? 'active' : 'none' },
    ],
  });
  await send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
  await navigate(pathToFileURL(resolve(file)).href);
  await evaluate('document.fonts.ready.then(() => true)');
  await poll(async () => (await evaluate(`document.documentElement.classList.contains('js')`)) || undefined, 'the script to start');
}

// The accessibility node of the element `expression` evaluates to.
async function ax(expression) {
  const { result } = await send('Runtime.evaluate', { expression });
  if (!result.result.objectId) throw new Error(`no element: ${expression}`);
  const { result: tree } = await send('Accessibility.getPartialAXTree', {
    objectId: result.result.objectId,
    fetchRelatives: false,
  });
  return tree.nodes[0];
}
const prop = (node, name) => node.properties?.find((p) => p.name === name)?.value?.value;
const SHOWN = `[...document.querySelectorAll('.arrangement')].find((a) => getComputedStyle(a).display !== 'none')`;
const HIDDEN = `[...document.querySelectorAll('.arrangement')].find((a) => getComputedStyle(a).display === 'none')`;
const CONTROLS = '[data-marker],[data-legend-row]';
const words = (text) => (text.match(/[\p{L}\p{N}]+/gu) ?? []).map((w) => w.toLowerCase());

// 1–4: name, role, description, expanded state, uniqueness (4.1.2, 2.5.3).
async function names(label) {
  const count = await evaluate(`${SHOWN}.querySelectorAll('${CONTROLS}').length`);
  const plotted = await evaluate(`document.querySelectorAll('tr[data-row]:not(.unplotted)').length`);
  check(count === 2 * plotted, `${label}: a marker and a legend row per plotted row: ${count} for ${plotted}`);
  const seen = new Set();
  for (let i = 0; i < count; i++) {
    const el = `${SHOWN}.querySelectorAll('${CONTROLS}')[${i}]`;
    const node = await ax(el);
    const name = node.name?.value ?? '';
    // What is drawn: the control's text, not its <title> tooltip.
    const visible = await evaluate(`[...${el}.querySelectorAll('text')].map((t) => t.textContent).join(' ')`);
    const number = words(visible)[0];
    check(!node.ignored && node.role?.value === 'button', `${label}: control ${i} is a button: ${node.role?.value}`);
    check(name.startsWith(`${number}. `), `${label}: "${name}" starts with its number ${number}`);
    let at = 0;
    const named = words(name);
    for (const w of words(visible)) {
      at = named.indexOf(w, at);
      if (at < 0) break;
      at += 1;
    }
    check(at >= 0, `${label}: "${name}" holds its visible words "${visible.trim()}" in order (2.5.3)`);
    check(prop(node, 'expanded') === false, `${label}: "${name}" reports collapsed`);
    const key = (await evaluate(`${el}.hasAttribute('data-marker')`)) ? 'marker' : 'legend';
    check(!seen.has(key + name), `${label}: "${name}" is not a duplicate`);
    seen.add(key + name);
  }
  const rows = await evaluate(`document.querySelectorAll('tr[data-row] button').length`);
  for (let i = 0; i < rows; i++) {
    const el = `document.querySelectorAll('tr[data-row] button')[${i}]`;
    await evaluate(`document.querySelector('details.table-view').open = true`);
    const node = await ax(el);
    const text = await evaluate(`${el}.textContent`);
    check(!node.ignored && node.role?.value === 'button', `${label}: table button ${i} is a button`);
    check(
      words(node.name?.value ?? '').join(' ') === words(text).join(' '),
      `${label}: table button named by its text: "${node.name?.value}" for "${text}"`,
    );
  }
  await evaluate(`document.querySelector('details.table-view').open = false`);
  // The hidden arrangement reaches no assistive technology (Review Focus 3).
  const hidden = await evaluate(`${HIDDEN}?.querySelectorAll('${CONTROLS}').length ?? 0`);
  for (let i = 0; i < hidden; i++) {
    const node = await ax(`${HIDDEN}.querySelectorAll('${CONTROLS}')[${i}]`);
    check(node.ignored, `${label}: the hidden arrangement's control ${i} is ignored`);
  }
  const description = (await ax(`${SHOWN}.querySelector('[data-marker]')`)).description?.value;
  return description;
}

// 3: opening an item makes exactly its controls report expanded.
async function expanded(label, width) {
  const first = `${SHOWN}.querySelector('[data-marker]')`;
  const key = await evaluate(`${first}.getAttribute('data-marker')`);
  for (let i = 0; i < 30 && !(await evaluate(`document.activeElement === ${first}`)); i++) await press('Tab');
  await press('Enter');
  const state = async () => {
    const out = [];
    const n = await evaluate(`${SHOWN}.querySelectorAll('${CONTROLS}').length`);
    for (let i = 0; i < n; i++) {
      const el = `${SHOWN}.querySelectorAll('${CONTROLS}')[${i}]`;
      if (prop(await ax(el), 'expanded') === true) out.push(await evaluate(`${el}.getAttribute('data-row')`));
    }
    return out;
  };
  const open = await state();
  check(open.length === 2 && open.every((k) => k === key), `${label}: the opened item's two controls report expanded: ${open}`);
  const region = await ax(`document.getElementById('detail')`);
  check(region.role?.value === 'region' && region.name?.value === 'Item details', `${label}: the panel is a region named Item details`);
  const controls = await evaluate(`(() => {
    const all = [...document.querySelectorAll('[role=button][data-row], tr[data-row] button')];
    return [all.length, all.filter((e) => document.getElementById(e.getAttribute('aria-controls') ?? '')).length];
  })()`);
  check(controls[0] > 0 && controls[0] === controls[1], `${label}: every control's aria-controls names the panel: ${controls[1]} of ${controls[0]}`);
  if (width === 1400) {
    await send('Emulation.setDeviceMetricsOverride', { width: 400, height: 900, deviceScaleFactor: 1, mobile: false });
    await poll(async () => (await evaluate(`${SHOWN}.classList.contains('narrow')`)) || undefined, 'the narrow arrangement');
    const narrow = await state();
    check(narrow.length === 2 && narrow.every((k) => k === key), `${label}: after the breakpoint, the narrow controls report expanded: ${narrow}`);
  }
}

// 5: every target is at least 24 CSS px (2.5.8, FR-94).
async function targets(label, { exceptionWidth } = {}) {
  const width = await evaluate('innerWidth');
  const scale = exceptionWidth ? width / exceptionWidth : null;
  // Below its own width the narrow SVG spans the viewport, with no inset:
  // FR-94's premise (NARROW_HORIZONTAL_INSET_PX = 0), never measured before.
  if (await evaluate(`${SHOWN}.classList.contains('narrow')`)) {
    const [svg, own] = await evaluate(`(() => { const e = ${SHOWN}.querySelector('svg'); return [e.getBoundingClientRect().width, e.viewBox.baseVal.width]; })()`);
    const view = await evaluate('document.documentElement.clientWidth');
    const want = Math.min(view, own);
    check(Math.abs(svg - want) <= 0.5, `${label}: the narrow SVG is ${svg} px, not ${want} (the viewport, up to its own width)`);
  }
  const sizes = await evaluate(`(() => {
    const shown = ${SHOWN};
    const out = [];
    for (const hit of shown.querySelectorAll('[data-hit]')) {
      const r = hit.getBoundingClientRect();
      out.push({ what: 'marker ' + hit.getAttribute('data-hit'), w: r.width, h: r.height, svg: true });
    }
    for (const row of shown.querySelectorAll('[data-legend-row]')) {
      const r = row.getBoundingClientRect();
      out.push({ what: 'legend row ' + row.getAttribute('data-legend-row'), w: r.width, h: r.height, svg: true });
    }
    return out;
  })()`);
  const short = sizes.filter((s) => s.w < 24 - TOLERANCE || s.h < 24 - TOLERANCE);
  if (scale === null) {
    for (const s of short) failures.push(`${label}: ${s.what} is ${s.w.toFixed(2)}×${s.h.toFixed(2)} px`);
  } else if (scale < 24 / TARGET_UNITS) {
    // D-71's exception: some narrow targets fall short, none below what
    // the widened document's own scale gives them.
    check(short.length > 0, `${label}: D-71's fixture must fall short somewhere, or it tests nothing`);
    const floor = TARGET_UNITS * scale - TOLERANCE;
    for (const s of short) check(Math.min(s.w, s.h) >= floor, `${label}: ${s.what} below D-71's floor ${floor.toFixed(2)}`);
    console.log(`    D-71's recorded exception: ${short.length} narrow targets under 24 px (floor ${floor.toFixed(2)})`);
  }
  // The pointer reaches each marker anywhere within 12 px of its centre.
  if (scale === null) {
    const misses = await evaluate(`(() => {
      const shown = ${SHOWN};
      const out = [];
      for (const hit of shown.querySelectorAll('[data-hit]')) {
        hit.scrollIntoView({ block: 'center' });
        const r = hit.getBoundingClientRect();
        const key = hit.getAttribute('data-hit');
        const cx = r.x + r.width / 2, cy = r.y + r.height / 2;
        for (let a = 0; a < 8; a++) {
          const x = cx + 11.9 * Math.cos(a * Math.PI / 4), y = cy + 11.9 * Math.sin(a * Math.PI / 4);
          const at = document.elementFromPoint(x, y);
          const owner = at?.closest('[data-hit],[data-marker]');
          const got = owner?.getAttribute('data-hit') ?? owner?.getAttribute('data-marker');
          if (got !== key) { out.push(key + '@' + a); break; }
        }
      }
      return out;
    })()`);
    // Recorded, not failed: FR-94 sizes each hit area, but neighbours'
    // areas may overlap, and WCAG does not count overlapped area. Whether
    // layout must space markers 36 units apart is open for the maintainer
    // (docs/release-1.0.0.md; crumb).
    if (misses.length) console.log(`    2.5.8 overlap (open): markers whose 24 px circle another target takes: ${misses}`);
  }
  // HTML controls are not scaled, so they never fall short.
  await evaluate(`document.querySelector('details.table-view').open = true`);
  const html = await evaluate(`(() => {
    const boxes = [];
    for (const e of document.querySelectorAll('.theme-toggle, summary, tr[data-row] button')) {
      const r = e.getBoundingClientRect();
      boxes.push({ what: e.className || e.tagName.toLowerCase(), w: r.width, h: r.height });
    }
    return boxes;
  })()`);
  for (const b of html) check(b.w >= 24 - TOLERANCE && b.h >= 24 - TOLERANCE, `${label}: ${b.what} is ${b.w.toFixed(2)}×${b.h.toFixed(2)} px`);
  await evaluate(`document.querySelector('details.table-view').open = false`);
}

// 6: reflow and resize (1.4.10, 1.4.4). Chrome's text only.
async function reflow(label) {
  await evaluate(`document.querySelector('details.table-view').open = true`);
  const sideways = await evaluate(`document.documentElement.scrollWidth - innerWidth`);
  check(sideways <= 0.5, `${label}: the page scrolls sideways by ${sideways} px`);
  // The skip link is visually hidden until focused, so it is not measured.
  const spills = await evaluate(`(${SPILLS})('.theme-toggle, h2, summary, thead th', 0.5)`);
  check(spills.length === 0, `${label}: text spills its box: ${spills}`);
  await evaluate(`document.querySelector('details.table-view').open = false`);
}

// 8: structure (1.3.1, 1.3.2, 2.4.6, 1.1.1).
async function structure(label) {
  const { result } = await send('Accessibility.getFullAXTree');
  const live = result.nodes.filter((n) => !n.ignored);
  const levels = live.filter((n) => n.role?.value === 'heading').map((n) => prop(n, 'level'));
  check(levels.filter((l) => l === 1).length === 1, `${label}: exactly one level-1 heading: ${levels}`);
  for (let i = 1; i < levels.length; i++) check(levels[i] <= levels[i - 1] + 1, `${label}: heading levels skip: ${levels}`);
  const group = live.find((n) => n.role?.value === 'group' && /quadrant/i.test(n.name?.value ?? ''));
  check(group && /\d/.test(group.name.value), `${label}: the radar group is named with its counts: ${group?.name?.value}`);
  const numbers = await evaluate(`[...${SHOWN}.querySelectorAll('[data-legend-row]')].map((r) => Number(r.getAttribute('aria-label').split('.')[0]))`);
  check(numbers.every((n, i) => n === i + 1), `${label}: legend rows read in number order: ${numbers}`);
  await evaluate(`document.querySelector('details.table-view').open = true`);
  const header = await ax(`document.querySelector('tr[data-row] th')`);
  const column = await ax(`document.querySelector('thead th')`);
  check(header.role?.value === 'rowheader', `${label}: the name cell is a row header: ${header.role?.value}`);
  check(column.role?.value === 'columnheader', `${label}: the head cells are column headers: ${column.role?.value}`);
  const region = await ax(`document.querySelector('.table-scroll')`);
  check(region.role?.value === 'region' && region.name?.value === 'Every item', `${label}: the table's region is named`);
  await evaluate(`document.querySelector('details.table-view').open = false`);
}

// 9: keyboard (2.1.2, 2.4.3, 2.4.7, 3.2.1). A full Tab pass.
async function keyboard(label) {
  const order = [];
  for (let i = 0; i < 120; i++) {
    await press('Tab');
    const step = await evaluate(`(() => {
      const a = document.activeElement;
      if (a === document.body || !a) return null;
      const svg = a.closest('svg');
      const key = a.getAttribute('data-marker');
      let drawn;
      if (key) {
        const ring = svg.querySelector('[data-marker-indicator="' + key + '"] [data-focus-ring]');
        const rb = ring.getBoundingClientRect(), ab = a.getBoundingClientRect();
        drawn = getComputedStyle(ring).visibility === 'visible' && rb.right > ab.left && rb.left < ab.right && rb.bottom > ab.top && rb.top < ab.bottom;
      } else if (a.hasAttribute('data-legend-row')) {
        drawn = getComputedStyle(a.querySelector('[data-row-indicator]')).visibility === 'visible';
      } else {
        drawn = getComputedStyle(a).outlineStyle !== 'none';
      }
      return {
        id: (a.getAttribute('data-marker') ? 'm' + a.getAttribute('data-marker') : '') + (a.getAttribute('data-legend-row') ? 'l' + a.getAttribute('data-legend-row') : '') || a.className || a.tagName,
        label: a.getAttribute('aria-label'),
        drawn,
        opened: !document.getElementById('detail').hidden,
      };
    })()`);
    if (!step) break;
    if (order.length && step.id === order[0].id) break;
    order.push(step);
  }
  check(order.length > 0 && order.length < 120, `${label}: a Tab pass ends (no trap): ${order.length} stops`);
  for (const s of order) {
    check(s.drawn, `${label}: focus on ${s.id} is drawn (2.4.7)`);
    check(!s.opened, `${label}: focus on ${s.id} opened the panel (3.2.1)`);
  }
  const ids = order.map((s) => s.id);
  check(new Set(ids).size === ids.length, `${label}: a stop repeats: ${ids}`);
  const markers = order.filter((s) => s.id.startsWith('m')).map((s) => Number(s.label.split('.')[0]));
  const rows = order.filter((s) => s.id.startsWith('l')).map((s) => Number(s.label.split('.')[0]));
  check(markers.every((n, i) => n === i + 1), `${label}: markers in number order (2.4.3): ${markers}`);
  check(rows.every((n, i) => n === i + 1), `${label}: legend rows in number order (2.4.3): ${rows}`);
  check(ids.indexOf(ids.find((i) => i.startsWith('l'))) > ids.indexOf(ids.find((i) => i.startsWith('m'))), `${label}: the legend follows the radar`);
}

// 7: forced colours paint every state, and colour is never alone (1.4.1).
async function forced(label) {
  const clip = async (selector) => {
    const r = await evaluate(`(() => { const e = ${selector}; e.scrollIntoView({ block: 'center' }); const b = e.getBoundingClientRect(); return { x: b.x - 2, y: b.y - 2, width: b.width + 4, height: b.height + 4 }; })()`);
    const shot = await send('Page.captureScreenshot', { format: 'png', clip: { ...r, scale: 1 } });
    return shot.result.data;
  };
  const canvas = await evaluate(`(() => { const d = document.createElement('div'); d.style.color = 'Canvas'; document.body.append(d); const c = getComputedStyle(d).color; d.remove(); return c; })()`);
  const marker = `${SHOWN}.querySelector('[data-marker]')`;
  const key = await evaluate(`${marker}.getAttribute('data-marker')`);
  const indicator = `${SHOWN}.querySelector('[data-marker-indicator="${key}"]')`;
  const row = `${SHOWN}.querySelector('[data-legend-row="${key}"]')`;
  await evaluate(`document.querySelector('details.table-view').open = true`);
  const cell = `document.querySelector('tr[data-row="${key}"] > td')`;
  const before = { indicator: await clip(marker), row: await clip(row), cell: await clip(cell) };
  // Focus: the ring.
  for (let i = 0; i < 30 && !(await evaluate(`document.activeElement === ${marker}`)); i++) await press('Tab');
  check((await clip(marker)) !== before.indicator, `${label}: a focused marker's ring is painted`);
  const ring = await evaluate(`getComputedStyle(${indicator}.querySelector('[data-focus-ring] circle')).stroke`);
  check(ring !== canvas, `${label}: the ring is not Canvas: ${ring}`);
  // Selection: the square, the bar and the table's border.
  await press('Enter');
  const square = await evaluate(`getComputedStyle(${indicator}.querySelector('[data-selection] rect')).stroke`);
  check(square !== canvas, `${label}: the selection square is not Canvas: ${square}`);
  check((await clip(row)) !== before.row, `${label}: a selected legend row's bar is painted`);
  const bar = await evaluate(`getComputedStyle(${row}.querySelector('[data-selection] rect')).fill`);
  check(bar !== canvas, `${label}: the bar is not Canvas: ${bar}`);
  check((await clip(cell)) !== before.cell, `${label}: a selected table row is marked`);
  const border = await evaluate(`getComputedStyle(${cell}).borderInlineStartColor`);
  check(border !== canvas, `${label}: the table row's border is not Canvas: ${border}`);
  const panel = await evaluate(`getComputedStyle(document.getElementById('detail')).borderTopColor`);
  check(panel !== canvas, `${label}: the panel's border is not Canvas: ${panel}`);
  // 1.4.1: status keeps its shape; the legend names every quadrant.
  const quadrants = await evaluate(`new Set([...document.querySelectorAll('tr[data-row]')].map((r) => r.children[2].textContent)).size`);
  const named = await evaluate(`[...${SHOWN}.querySelectorAll('[role=heading][aria-level="2"]')].length`);
  check(named >= quadrants - 1, `${label}: the legend heads each quadrant: ${named} for ${quadrants}`);
}

async function run(name, body) {
  const before = failures.length;
  thrown.length = 0;
  try {
    await body();
  } catch (error) {
    failures.push(`${name}: ${error.message}`);
  }
  for (const t of thrown) failures.push(`${name}: page threw: ${t}`);
  const mine = failures.length - before;
  console.log(`${mine ? 'FAIL' : 'ok'} ${name}`);
  for (const f of failures.slice(before)) console.log('    ' + f);
}

try {
  const query = async () => {
    await load(page, 1400);
    return Number((await evaluate(`document.querySelector('main').getAttribute('data-narrow-query')`)).match(/\d+/)[0]);
  };
  const lastNarrow = await query();
  const widths = [1400, lastNarrow + 1, lastNarrow, 640, 400, 320];
  for (const width of widths) {
    await run(`names, roles, expanded state @${width}`, async () => {
      await load(page, width);
      await names(`@${width}`);
      await expanded(`@${width}`, width);
    });
    await run(`targets @${width}`, async () => {
      await load(page, width);
      await targets(`@${width}`);
    });
    await run(`reflow @${width}`, async () => {
      await load(page, width);
      await reflow(`@${width}`);
    });
    await run(`structure @${width}`, async () => {
      await load(page, width);
      await structure(`@${width}`);
    });
    await run(`keyboard @${width}`, async () => {
      await load(page, width);
      await keyboard(`@${width}`);
    });
  }
  await run('chrome text at 200% @1400', async () => {
    await load(page, 1400);
    await evaluate(`document.documentElement.style.fontSize = '200%'`);
    await reflow('root font 200%');
  });
  for (const scheme of ['dark']) {
    for (const width of [1400, 400]) {
      await run(`${scheme} @${width}`, async () => {
        await load(page, width, { scheme });
        await names(`${scheme} @${width}`);
        await keyboard(`${scheme} @${width}`);
      });
    }
  }
  for (const width of [1400, 400]) {
    await run(`forced colours @${width}`, async () => {
      await load(page, width, { forced: true });
      await forced(`forced @${width}`);
    });
    await run(`forced colours, keyboard @${width}`, async () => {
      await load(page, width, { forced: true });
      await keyboard(`forced @${width}`);
    });
  }
  check(W71 > 491, `D-71's fixture is ${W71} units wide; over 491 is needed to clear the tolerance`);
  for (const width of [400, 320]) {
    await run(`D-71's exception @${width}`, async () => {
      await load(d71, width);
      await targets(`D-71 @${width}`, { exceptionWidth: W71 });
    });
  }
} finally {
  await close();
}
process.exit(failures.length === 0 ? 0 : 1);
