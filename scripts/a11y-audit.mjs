// WCAG 2.1 AA's automatable criteria, audited in a browser (plan RG-015C).
//
// Usage: node scripts/a11y-audit.mjs <page.html> <d71.html> <d71-narrow-width>
//
// The first page is the audit fixture (the interaction fixture plus a ring
// whose heading wraps, so a wrapped heading's label is read); the second is D-71's, whose
// narrow arrangement is wider than 480 units, so its narrow targets fall
// below 24 px at 320 px by design: D-71's recorded 2.5.8 exception. Markers meet 2.5.8
// through its Equivalent exception (PRD D-81).
//
// Controls are found from the DOM and then looked up in Chromium's
// accessibility tree, so a control that lost its role is caught, not
// skipped. The audit uses trusted input only, waits for the script and
// the fonts, and fails on any page exception or dialog. Each numbered
// check names the WCAG criterion it backs, as RG-015C's table does.

import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { SHOWN, SHOWN_ICON, SPILLS, connect, poll, shownIcon } from './cdp.mjs';

const [page, d71, d71Width] = process.argv.slice(2);
if (!page || !d71 || !d71Width) {
  console.error('usage: node scripts/a11y-audit.mjs <page.html> <d71.html> <d71-narrow-width>');
  process.exit(2);
}
const W71 = Number(d71Width);
const TOLERANCE = 0.05;
// WCAG 2.5.8's least target, in CSS px.
const TARGET_PX = 24;
// The most stops a Tab pass may take before it counts as a trap.
const TAB_LIMIT = 120;
// FR-94: a hit area's and a legend row's least size, in user units.
const TARGET_UNITS = 36;
// The narrowest D-71 fixture whose targets fall short at 320 px by more than
// the tolerance, so the exception check has something to bound.
const D71_MIN_WIDTH = (TARGET_UNITS * 320) / (TARGET_PX - TOLERANCE);

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

async function load(file, width, { scheme = 'light', forced = false } = {}) {
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
const HIDDEN = `[...document.querySelectorAll('.arrangement')].find((a) => getComputedStyle(a).display === 'none')`;
const CONTROLS = '[data-marker],[data-legend-row]';
const words = (text) => (text.match(/[\p{L}\p{N}]+/gu) ?? []).map((w) => w.toLowerCase());
const openTable = (open) => evaluate(`document.querySelector('details.table-view').open = ${open}`);
const plottedCount = () => evaluate(`document.querySelectorAll('tr[data-row]:not(.unplotted)').length`);
const tabTo = async (expression) => {
  for (let i = 0; i < TAB_LIMIT && !(await evaluate(`document.activeElement === ${expression}`)); i++) await press('Tab');
  check(await evaluate(`document.activeElement === ${expression}`), `Tab never reached ${expression.slice(0, 60)}`);
};
// The colours forced-colours mode paints with, resolved, so a mark's colour
// can be checked against them.
const SYSTEM_COLOURS = `(() => {
  const out = {};
  for (const name of ['Canvas', 'CanvasText', 'Highlight', 'HighlightText', 'ButtonText', 'ButtonFace', 'LinkText', 'GrayText', 'ButtonBorder', 'Mark', 'MarkText', 'AccentColor', 'AccentColorText']) {
    const d = document.createElement('div');
    d.style.color = name;
    document.body.append(d);
    out[name] = getComputedStyle(d).color;
    d.remove();
  }
  return out;
})()`;

// 1–4: name, role, description, expanded state, uniqueness (4.1.2, 2.5.3).
async function names(label) {
  const count = await evaluate(`${SHOWN}.querySelectorAll('${CONTROLS}').length`);
  const plotted = await plottedCount();
  check(plotted > 0 && count === 2 * plotted, `${label}: a marker and a legend row per plotted row: ${count} for ${plotted}`);
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
  // rg-nuy: the theme toggle is named by what a press does, and has no
  // pressed state to decode.
  const toggleNode = await ax(`document.querySelector('.theme-toggle')`);
  const toggleName = toggleNode.name?.value ?? '';
  // Every run but the dark ones loads the light preference.
  const action = `Switch to ${label.startsWith('dark') ? 'light' : 'dark'} theme`;
  check(
    toggleNode.role?.value === 'button' && toggleName === action,
    `${label}: the toggle is a button named "${action}": ${toggleNode.role?.value} "${toggleName}"`,
  );
  check(prop(toggleNode, 'pressed') === undefined, `${label}: the toggle has no pressed state`);
  // RG-102: the legend button too, named for where the page starts.
  const legendNode = await ax(`document.querySelector('.legend-toggle')`);
  check(
    legendNode.role?.value === 'button' && legendNode.name?.value === 'Hide legend',
    `${label}: the legend button is a button named "Hide legend": ${legendNode.role?.value} "${legendNode.name?.value}"`,
  );
  check(prop(legendNode, 'pressed') === undefined, `${label}: the legend button has no pressed state`);
  await openTable(true);
  const rows = await evaluate(`document.querySelectorAll('tr[data-row] button').length`);
  check(rows > 0, `${label}: the table has buttons`);
  for (let i = 0; i < rows; i++) {
    const el = `document.querySelectorAll('tr[data-row] button')[${i}]`;
    const node = await ax(el);
    const text = await evaluate(`${el}.textContent`);
    check(!node.ignored && node.role?.value === 'button', `${label}: table button ${i} is a button`);
    check(
      words(node.name?.value ?? '').join(' ') === words(text).join(' '),
      `${label}: table button named by its text: "${node.name?.value}" for "${text}"`,
    );
  }
  await openTable(false);
  // The hidden arrangement reaches no assistive technology (Review Focus 3).
  const hidden = await evaluate(`${HIDDEN}?.querySelectorAll('${CONTROLS}').length ?? 0`);
  for (let i = 0; i < hidden; i++) {
    const node = await ax(`${HIDDEN}.querySelectorAll('${CONTROLS}')[${i}]`);
    check(node.ignored, `${label}: the hidden arrangement's control ${i} is ignored`);
  }
}

// 3: opening an item makes exactly its controls report expanded.
async function expanded(label, width) {
  const first = `${SHOWN}.querySelector('[data-marker]')`;
  const key = await evaluate(`${first}.getAttribute('data-marker')`);
  await tabTo(first);
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
  await openTable(true);
  const button = await ax(`document.querySelector('tr[data-row="${key}"] button')`);
  check(prop(button, 'expanded') === true, `${label}: the opened item's table button reports expanded`);
  await openTable(false);
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

// 5: every target is at least 24 CSS px (2.5.8, FR-94; a WCAG 2.2
// criterion, claimed beside NFR-1's 2.1 AA).
async function targets(label, { exceptionWidth } = {}) {
  const width = await evaluate('innerWidth');
  const scale = exceptionWidth ? width / exceptionWidth : null;
  const exception = scale !== null && scale < TARGET_PX / TARGET_UNITS;
  // Below its own width the narrow SVGs span the viewport, with no inset:
  // FR-94's premise (NARROW_HORIZONTAL_INSET_PX = 0), never measured before.
  // The body's SVG is measured; the legend's shares its width (RG-102's
  // 'narrow parts stack' interaction check).
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
      out.push({ what: 'marker ' + hit.getAttribute('data-hit'), w: r.width, h: r.height });
    }
    for (const row of shown.querySelectorAll('[data-legend-row]')) {
      const r = row.getBoundingClientRect();
      out.push({ what: 'legend row ' + row.getAttribute('data-legend-row'), w: r.width, h: r.height });
    }
    return out;
  })()`);
  check(sizes.length > 0, `${label}: there are SVG targets to measure`);
  const short = sizes.filter((s) => s.w < TARGET_PX - TOLERANCE || s.h < TARGET_PX - TOLERANCE);
  if (!exception) {
    for (const s of short) failures.push(`${label}: ${s.what} is ${s.w.toFixed(2)}×${s.h.toFixed(2)} px`);
  } else {
    // D-71's exception: some narrow targets fall short, none below what
    // the widened document's own scale gives them.
    check(short.length > 0, `${label}: D-71's fixture must fall short somewhere, or it tests nothing`);
    const floor = TARGET_UNITS * scale - TOLERANCE;
    for (const s of short) check(Math.min(s.w, s.h) >= floor, `${label}: ${s.what} below D-71's floor ${floor.toFixed(2)}`);
    console.log(`    ${label}: D-71's recorded exception, ${short.length} narrow targets under ${TARGET_PX} px (floor ${floor.toFixed(2)})`);
    return;
  }
  // The pointer reaches each marker anywhere within 12 px of its centre.
  // A point that reaches nothing, or something other than a marker, is a
  // failure. A point that reaches a neighbouring marker is the spacing
  // question, which the Equivalent exception answers: the item's legend row
  // and table button open the same panel and each meet 24 px (checked here
  // and below), so the marker need not.
  const probe = await evaluate(`(() => {
    const shown = ${SHOWN};
    const lost = [], neighbours = [];
    for (const hit of shown.querySelectorAll('[data-hit]')) {
      hit.scrollIntoView({ block: 'center' });
      const r = hit.getBoundingClientRect();
      const key = hit.getAttribute('data-hit');
      const cx = r.x + r.width / 2, cy = r.y + r.height / 2;
      for (let a = 0; a < 8; a++) {
        const reach = ${TARGET_PX} / 2 - 0.1;
        const at = document.elementFromPoint(cx + reach * Math.cos(a * Math.PI / 4), cy + reach * Math.sin(a * Math.PI / 4));
        const owner = at?.closest('[data-hit],[data-marker]');
        const got = owner?.getAttribute('data-hit') ?? owner?.getAttribute('data-marker');
        if (got === key) continue;
        (got ? neighbours : lost).push(key + '@' + a);
        break;
      }
    }
    return { lost, neighbours };
  })()`);
  check(probe.lost.length === 0, `${label}: markers the pointer cannot reach within 12 px: ${probe.lost}`);
  if (probe.neighbours.length) {
    console.log(`    ${label}: 2.5.8 spacing, markers whose 24 px circle meets a neighbour: ${probe.neighbours}; met by the Equivalent exception (each item's legend row and table button)`);
  }
  // HTML controls are not scaled, so they never fall short: the toggle, the
  // summary, the table buttons, the skip link (focused) and Close (open).
  await openTable(true);
  const measure = (selector) => evaluate(`[...document.querySelectorAll('${selector}')].map((e) => { const r = e.getBoundingClientRect(); return { what: e.className || e.tagName.toLowerCase(), w: r.width, h: r.height }; })`);
  const html = [...(await measure('.legend-toggle, .theme-toggle, summary, tr[data-row] button'))];
  await tabTo(`document.querySelector('.skip')`);
  html.push(...(await measure('.skip')));
  await press('Enter');
  await tabTo(`${SHOWN}.querySelector('[data-marker]')`);
  await press('Enter');
  html.push(...(await measure('.detail-close')));
  check(html.length >= 4, `${label}: the HTML controls were measured: ${html.length}`);
  for (const b of html) check(b.w >= TARGET_PX - TOLERANCE && b.h >= TARGET_PX - TOLERANCE, `${label}: ${b.what} is ${b.w.toFixed(2)}×${b.h.toFixed(2)} px`);
}

// 6: reflow and resize (1.4.10, 1.4.4). Chrome's text only.
async function reflow(label) {
  await openTable(true);
  const sideways = await evaluate(`document.documentElement.scrollWidth - innerWidth`);
  check(sideways <= 0.5, `${label}: the page scrolls sideways by ${sideways} px`);
  // The skip link is visually hidden until focused, so it is not measured,
  // and the theme toggle shows an icon, its words visually hidden (rg-1ru).
  const spills = await evaluate(`(${SPILLS})('h2, summary, thead th', 0.5)`);
  check(spills.length === 0, `${label}: text spills its box: ${spills}`);
  // Each icon button (rg-1ru, RG-102) shows one icon, inside its box.
  // A --palette page has no theme toggle; every page with items has the
  // legend button, so its absence, or a zero box, fails.
  const iconFits = await evaluate(`['.theme-toggle', '.legend-toggle'].every((selector) => {
    const button = document.querySelector(selector);
    if (!button) return selector === '.theme-toggle';
    if (button.getBoundingClientRect().width === 0) return false;
    const o = button.getBoundingClientRect();
    const icon = [...button.querySelectorAll('svg')].find((s) => getComputedStyle(s).display !== 'none');
    if (!icon) return false;
    const i = icon.getBoundingClientRect();
    return i.left >= o.left && i.right <= o.right && i.top >= o.top && i.bottom <= o.bottom;
  })`);
  check(iconFits, `${label}: each icon button shows an icon, inside its box`);
  await openTable(false);
}

// 8: structure (1.3.1, 1.3.2, 2.4.6, 1.1.1).
async function structure(label) {
  const { result } = await send('Accessibility.getFullAXTree');
  const live = result.nodes.filter((n) => !n.ignored);
  const levels = live.filter((n) => n.role?.value === 'heading').map((n) => prop(n, 'level'));
  check(levels.length > 0 && levels.every((l) => Number.isInteger(l)), `${label}: headings report their levels: ${levels}`);
  check(levels.filter((l) => l === 1).length === 1, `${label}: exactly one level-1 heading: ${levels}`);
  for (let i = 1; i < levels.length; i++) check(levels[i] <= levels[i - 1] + 1, `${label}: heading levels skip: ${levels}`);
  const group = live.find((n) => n.role?.value === 'group' && /quadrant/i.test(n.name?.value ?? ''));
  check(group && /\d/.test(group.name.value), `${label}: the radar group is named with its counts: ${group?.name?.value}`);
  // The legend groups rows under their quadrant (level 3) and ring (level 4):
  // every quadrant has its heading, and each row sits under its own.
  const legend = await evaluate(`(() => {
    const shown = ${SHOWN};
    const quadrants = new Set([...document.querySelectorAll('tr[data-row]:not(.unplotted)')].map((r) => r.children[2].textContent.trim()));
    const headed = new Set();
    const misplaced = [];
    let quadrant = null, ring = null;
    for (const e of shown.querySelectorAll('[role=heading][aria-level="3"], [role=heading][aria-level="4"], [data-legend-row]')) {
      // A wrapped heading's tspans join with nothing between; its label
      // has the spaces (rg-g0c).
      const named = e.getAttribute('aria-label') ?? e.textContent.trim();
      if (e.getAttribute('aria-level') === '3') { quadrant = named; headed.add(quadrant); ring = null; continue; }
      if (e.getAttribute('aria-level') === '4') { ring = named; continue; }
      const label = e.getAttribute('aria-label');
      if (!label.includes('; ' + quadrant + ' quadrant, ' + ring + ' ring')) misplaced.push(label);
    }
    return { quadrants: [...quadrants].sort(), headed: [...headed].sort(), misplaced };
  })()`);
  check(legend.quadrants.length > 0 && legend.quadrants.join('|') === legend.headed.join('|'), `${label}: the legend heads every quadrant: ${legend.headed} for ${legend.quadrants}`);
  check(legend.misplaced.length === 0, `${label}: legend rows outside their quadrant and ring: ${legend.misplaced}`);
  const numbers = await evaluate(`[...${SHOWN}.querySelectorAll('[data-legend-row]')].map((r) => Number(r.getAttribute('aria-label').split('.')[0]))`);
  check(numbers.length === (await plottedCount()) && numbers.every((n, i) => n === i + 1), `${label}: legend rows read in number order: ${numbers}`);
  await openTable(true);
  const header = await ax(`document.querySelector('tr[data-row] th')`);
  const column = await ax(`document.querySelector('thead th')`);
  check(header.role?.value === 'rowheader', `${label}: the name cell is a row header: ${header.role?.value}`);
  check(column.role?.value === 'columnheader', `${label}: the head cells are column headers: ${column.role?.value}`);
  const region = await ax(`document.querySelector('.table-scroll')`);
  check(region.role?.value === 'region' && region.name?.value === 'Every item', `${label}: the table's region is named`);
  await openTable(false);
}

// 9: keyboard (2.1.2, 2.4.3, 2.4.7, 3.2.1). A full Tab pass.
// `legendHidden` (RG-102): the legend button was pressed, so the pass meets
// no legend row and the skip link goes to the table.
async function keyboard(label, { legendHidden = false } = {}) {
  const order = [];
  for (let i = 0; i < TAB_LIMIT; i++) {
    await press('Tab');
    const step = await evaluate(`(() => {
      const a = document.activeElement;
      if (a === document.body || !a) return null;
      const svg = a.closest('svg');
      const key = a.getAttribute('data-marker');
      const box = (e) => e.getBoundingClientRect();
      const overlaps = (p, q) => p.right > q.left && p.left < q.right && p.bottom > q.top && p.top < q.bottom;
      const painted = (c) => c && c !== 'none' && c !== 'transparent' && !/rgba\\([^)]*,\\s*0\\)$/.test(c);
      let drawn;
      if (key) {
        const ring = svg.querySelector('[data-marker-indicator="' + key + '"] [data-focus-ring]');
        drawn = getComputedStyle(ring).visibility === 'visible' && overlaps(box(ring), box(a));
      } else if (a.hasAttribute('data-legend-row')) {
        const rect = a.querySelector('[data-row-indicator] rect');
        drawn = getComputedStyle(rect).visibility === 'visible' && painted(getComputedStyle(rect).stroke);
      } else {
        const s = getComputedStyle(a);
        drawn = s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) >= 1 && painted(s.outlineColor);
      }
      return {
        id: (key ? 'm' + key : '') + (a.getAttribute('data-legend-row') ? 'l' + a.getAttribute('data-legend-row') : '') || a.className || a.tagName,
        label: a.getAttribute('aria-label'),
        drawn,
        opened: !document.getElementById('detail').hidden,
      };
    })()`);
    if (!step) break;
    if (order.length && step.id === order[0].id) break;
    order.push(step);
  }
  check(order.length > 0 && order.length < TAB_LIMIT, `${label}: a Tab pass ends (no trap): ${order.length} stops`);
  for (const s of order) {
    check(s.drawn, `${label}: focus on ${s.id} is drawn (2.4.7)`);
    check(!s.opened, `${label}: focus on ${s.id} opened the panel (3.2.1)`);
  }
  // Item stops are `m` or `l` and a number; `legend-toggle` is not a row.
  const ids = order.map((s) => s.id);
  check(new Set(ids).size === ids.length, `${label}: a stop repeats: ${ids}`);
  const plotted = await plottedCount();
  const markers = order.filter((s) => /^m\d/.test(s.id)).map((s) => Number(s.label.split('.')[0]));
  const rows = order.filter((s) => /^l\d/.test(s.id)).map((s) => Number(s.label.split('.')[0]));
  check(markers.length === plotted && markers.every((n, i) => n === i + 1), `${label}: every marker, in number order (2.4.3): ${markers}`);
  const wantRows = legendHidden ? 0 : plotted;
  check(rows.length === wantRows && rows.every((n, i) => n === i + 1), `${label}: ${wantRows} legend rows, in number order (2.4.3): ${rows}`);
  // The page's sequence: skip link, legend button, theme toggle, markers,
  // legend rows unless hidden, the table. With the legend hidden the pass
  // starts on the legend button just pressed, so it begins at the toggle.
  const at = (p) => ids.findIndex(p);
  const head = legendHidden ? [] : [at((i) => i === 'skip'), at((i) => i.includes('legend-toggle'))];
  const sequence = [...head, at((i) => i.includes('theme-toggle')), at((i) => /^m\d/.test(i)), ...(legendHidden ? [] : [at((i) => /^l\d/.test(i))]), at((i) => i === 'SUMMARY')];
  check(sequence.every((n) => n >= 0) && sequence.every((n, i) => i === 0 || n > sequence[i - 1]), `${label}: stops out of the page's order: ${ids}`);
}

// 7: forced colours paint every state. Each mark is shown by a pixel change
// with the panel hidden from the capture, is visible, and is painted in one
// of the system colours other than Canvas: a hard-coded colour, which
// Chromium does not force in SVG, fails.
async function forced(label) {
  const clip = async (selector) => {
    const r = await evaluate(`(() => {
      const panel = document.getElementById('detail');
      panel.style.visibility = 'hidden';
      const e = ${selector};
      e.scrollIntoView({ block: 'center' });
      const b = e.getBoundingClientRect();
      // The clip is in page coordinates, not the viewport's.
      return { x: b.x + scrollX - 2, y: b.y + scrollY - 2, width: b.width + 4, height: b.height + 4 };
    })()`);
    const shot = await send('Page.captureScreenshot', { format: 'png', clip: { ...r, scale: 1 } });
    await evaluate(`document.getElementById('detail').style.visibility = ''`);
    return shot.result.data;
  };
  const system = await evaluate(SYSTEM_COLOURS);
  const allowed = new Set(Object.entries(system).filter(([name]) => name !== 'Canvas').map(([, c]) => c));
  const painted = (what, colour) => check(allowed.has(colour), `${label}: ${what} is painted in a system colour other than Canvas: ${colour}`);
  const marker = `${SHOWN}.querySelector('[data-marker]')`;
  const key = await evaluate(`${marker}.getAttribute('data-marker')`);
  const indicator = `${SHOWN}.querySelector('[data-marker-indicator="${key}"]')`;
  const row = `${SHOWN}.querySelector('[data-legend-row="${key}"]')`;
  const other = await evaluate(`${SHOWN}.querySelectorAll('[data-marker]')[1].getAttribute('data-marker')`);
  const otherIndicator = `${SHOWN}.querySelector('[data-marker-indicator="${other}"]')`;
  await openTable(true);
  const cell = `document.querySelector('tr[data-row="${key}"] > td')`;
  const toggle = `document.querySelector('.theme-toggle')`;
  const before = { marker: await clip(marker), row: await clip(row), cell: await clip(cell), other: await clip(otherIndicator), toggle: await clip(toggle) };
  // The toggle: forced colours override both palettes, so its icon is the
  // only visible sign a press worked.
  await tabTo(toggle);
  const focusedToggle = await clip(toggle);
  await press('Enter');
  check((await clip(toggle)) !== focusedToggle, `${label}: a press repaints the toggle's icon`);
  painted("the toggle's icon", await evaluate(`(() => { const icon = ${SHOWN_ICON}; return icon ? getComputedStyle(icon).stroke : 'no icon shown'; })()`));
  painted("the legend button's icon", await evaluate(`(() => { const icon = ${shownIcon('.legend-toggle')}; return icon ? getComputedStyle(icon).stroke : 'no icon shown'; })()`));
  // Focus: a marker's ring.
  await tabTo(marker);
  check((await clip(marker)) !== before.marker, `${label}: a focused marker's ring is painted`);
  const ring = `${indicator}.querySelector('[data-focus-ring] circle')`;
  check((await evaluate(`getComputedStyle(${indicator}.querySelector('[data-focus-ring]')).visibility`)) === 'visible', `${label}: the ring is visible`);
  painted('the ring', await evaluate(`getComputedStyle(${ring}).stroke`));
  // Selection: the square, the bar, the table's border, the panel's border.
  await press('Enter');
  const square = `${indicator}.querySelector('[data-selection] rect')`;
  check((await evaluate(`getComputedStyle(${indicator}.querySelector('[data-selection]')).visibility`)) === 'visible', `${label}: the selection square is visible`);
  painted('the selection square', await evaluate(`getComputedStyle(${square}).stroke`));
  check((await clip(row)) !== before.row, `${label}: a selected legend row's bar is painted`);
  painted('the bar', await evaluate(`getComputedStyle(${row}.querySelector('[data-selection] rect')).fill`));
  check((await clip(cell)) !== before.cell, `${label}: a selected table row is marked`);
  painted("the table row's border", await evaluate(`getComputedStyle(${cell}).borderInlineStartColor`));
  check((await evaluate(`parseFloat(getComputedStyle(${cell}).borderInlineStartWidth)`)) >= 1, `${label}: the table row's border has width`);
  const panel = await evaluate(`(() => { const s = getComputedStyle(document.getElementById('detail')); return [s.borderTopColor, parseFloat(s.borderTopWidth), s.borderTopStyle]; })()`);
  painted("the panel's border", panel[0]);
  check(panel[1] >= 1 && panel[2] !== 'none', `${label}: the panel has a border: ${panel}`);
  // Hover: another marker's dashed square.
  const at = await evaluate(`(() => { const e = ${SHOWN}.querySelector('[data-marker="${other}"] circle'); e.scrollIntoView({ block: 'center' }); const b = e.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; })()`);
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at.x, y: at.y });
  check((await clip(otherIndicator)) !== before.other, `${label}: hover's dashes are painted`);
  painted("hover's dashes", await evaluate(`getComputedStyle(${otherIndicator}.querySelector('[data-selection] rect')).stroke`));
  // Focus on a legend row: its outline.
  const otherRow = `${SHOWN}.querySelector('[data-legend-row="${other}"]')`;
  await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 1, y: 1 });
  const rowBefore = await clip(otherRow);
  await press('Escape');
  await tabTo(otherRow);
  check((await clip(otherRow)) !== rowBefore, `${label}: a focused legend row's outline is painted`);
  painted("the legend row's outline", await evaluate(`getComputedStyle(${otherRow}.querySelector('[data-row-indicator] rect')).stroke`));
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
  // RG-102: with the legend hidden, its rows leave the tree and the tab
  // order, the button offers to show it, and the skip link goes to the table.
  for (const width of [1400, 400]) {
    await run(`legend hidden @${width}`, async () => {
      await load(page, width);
      await tabTo(`document.querySelector('.legend-toggle')`);
      await press('Enter');
      const button = await ax(`document.querySelector('.legend-toggle')`);
      check(button.name?.value === 'Show legend' && prop(button, 'pressed') === undefined, `@${width} hidden: the button is named "Show legend", unpressed: "${button.name?.value}"`);
      const row = await ax(`${SHOWN}.querySelector('[data-legend-row]')`);
      check(row.ignored === true, `@${width} hidden: a legend row is out of the accessibility tree`);
      const skip = await ax(`document.querySelector('.skip')`);
      check(skip.name?.value === 'Skip to the table', `@${width} hidden: the skip link is "Skip to the table": "${skip.name?.value}"`);
      await keyboard(`legend hidden @${width}`, { legendHidden: true });
    });
  }
  await run('chrome text at 200% @1400', async () => {
    await load(page, 1400);
    await evaluate(`document.documentElement.style.fontSize = '200%'`);
    await reflow('root font 200%');
  });
  for (const width of [1400, 400]) {
    await run(`dark @${width}`, async () => {
      await load(page, width, { scheme: 'dark' });
      await names(`dark @${width}`);
      await keyboard(`dark @${width}`);
    });
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
  check(W71 > D71_MIN_WIDTH, `D-71's fixture is ${W71} units wide; over ${D71_MIN_WIDTH.toFixed(0)} is needed to clear the tolerance`);
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
