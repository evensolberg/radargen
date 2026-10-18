// RG-102 in Firefox and WebKit, where a click need not focus a button (as
// in WebKit and Firefox on macOS): a focused legend row hidden by a click on
// the legend button hands focus to the button, and both legends hide. The
// CDP checks drive Chromium only, which focuses a clicked button.
//
// D-97 too: between the two breakpoints a click on the button swaps the
// arrangements, narrow to wide and back. A focused narrow marker or legend
// row the first click hides hands focus to the button, as does a wide
// marker hidden by a click that shows the legend; the second click, which
// in WebKit first blurs the button, leaves it there. And, in Firefox at
// 1.5 device pixels per CSS pixel, a viewport a third of a pixel under
// either breakpoint shows exactly one arrangement, the one the script's
// matchMedia expects, whichever the legend's state.
//
// Playwright's locator.focus() sets up the focused row; the click itself is
// a real one. Only WebKit tells the guard apart everywhere: Firefox focuses
// a clicked button except on macOS, so on Linux it passes either way.
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { LEGEND_PARTS, legendState } from './cdp.mjs';

const [file] = process.argv.slice(2);
if (!file) {
  console.error('usage: node scripts/legend-engines.mjs <page.html>');
  process.exit(2);
}
const url = pathToFileURL(resolve(file)).href;

const playwright = await import('playwright').catch((error) => {
  if (error.code !== 'ERR_MODULE_NOT_FOUND') throw error;
  console.error("no Playwright: run 'just browser-setup' (npm ci, then the browsers)");
  process.exit(2);
});


// Firefox on macOS needs a home of its own, or it exits with "Could not
// find profile folder"; text-spacing.mjs explains, and shares the folder.
const firefoxHome = join(tmpdir(), 'radargen-firefox-home');
mkdirSync(firefoxHome, { recursive: true });
const LAUNCH = {
  firefox: { env: { ...process.env, CFFIXED_USER_HOME: firefoxHome } },
  webkit: {},
};

let failed = false;
const check = (name, ok, what) => {
  console.log(`${ok ? 'ok' : 'FAIL'} ${name}: ${what}`);
  failed ||= !ok;
};
// Page-side: which arrangement shows, how many do, and whether the button
// has focus.
const ARRANGEMENTS = () => [...document.querySelectorAll('.arrangement')].filter((a) => getComputedStyle(a).display !== 'none').map((a) => a.classList[1]);
const ON_BUTTON = () => !!document.activeElement?.classList.contains('legend-toggle');
// The page's breakpoints, from <main>.
const BREAKPOINTS = () => {
  const px = (name) => Number(document.querySelector('main').getAttribute(name).match(/\d+/)[0]) + 1;
  return { shown: px('data-narrow-query'), hidden: px('data-narrow-query-hidden') };
};

for (const name of ['firefox', 'webkit']) {
  const browser = await playwright[name].launch(LAUNCH[name]);
  try {
    const page = await (await browser.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    const shows = async () => (await page.evaluate(ARRANGEMENTS))[0];
    const onButton = () => page.evaluate(ON_BUTTON);
    const toggle = () => page.locator('.legend-toggle').click();

    await page.goto(url);
    await page.locator('.arrangement.wide [data-legend-row]').first().focus();
    check(name, await page.evaluate(() => document.activeElement?.hasAttribute('data-legend-row')), 'a legend row had focus before the click');
    await toggle();
    check(name, await onButton(), 'focus moves to the button');
    const state = legendState(await page.evaluate(LEGEND_PARTS));
    check(name, state === 'hidden', `both legends hide (saw ${state})`);

    // D-97: halfway between the breakpoints.
    const { shown, hidden } = await page.evaluate(BREAKPOINTS);
    const width = Math.round((shown + hidden) / 2);
    await page.setViewportSize({ width, height: 900 });
    for (const [what, focus] of [
      ['marker', '.arrangement.narrow [data-marker]'],
      ['legend row', '.arrangement.narrow [data-legend-row]'],
    ]) {
      await page.goto(url);
      check(name, (await shows()) === 'narrow', `@${width}: the shown legend gives the narrow arrangement`);
      await page.locator(focus).first().focus();
      check(name, await page.evaluate((s) => !!document.activeElement?.matches(s), focus), `@${width}: a narrow ${what} had focus before the click`);
      await toggle();
      check(name, (await shows()) === 'wide' && (await onButton()), `@${width}: hiding the legend swaps to wide, focus on the button`);
      await toggle();
      check(name, (await shows()) === 'narrow' && (await onButton()), `@${width}: showing it swaps back to narrow, focus still on the button`);
    }
    // The reverse: hidden, a wide marker focused, a click that shows it.
    await page.goto(url);
    await toggle();
    await page.locator('.arrangement.wide [data-marker]').first().focus();
    check(name, await page.evaluate(() => !!document.activeElement?.closest('.arrangement.wide [data-marker]')), `@${width}: a wide marker had focus before the click`);
    await toggle();
    check(name, (await shows()) === 'narrow' && (await onButton()), `@${width}: showing the legend swaps to narrow, focus on the button`);
    check(name, errors.length === 0, `no script errors: ${errors}`);
  } finally {
    await browser.close();
  }
}

// D-97 at fractional zoom, in Firefox: at 1.5 device pixels per CSS pixel
// an iframe 1/3 px under a breakpoint has a fractional viewport, in the gap
// `max-width` leaves between B - 1 and B. Exactly one arrangement shows,
// and it is the one the script's matchMedia for the legend's state says.
{
  const name = 'firefox';
  const browser = await playwright.firefox.launch({ ...LAUNCH.firefox, firefoxUserPrefs: { 'layout.css.devPixelsPerPx': '1.5' } });
  try {
    const page = await (await browser.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
    await page.goto(url);
    const { shown, hidden } = await page.evaluate(BREAKPOINTS);
    const dir = mkdtempSync(join(tmpdir(), 'radargen-frac-'));
    for (const breakpoint of [hidden, shown]) {
      const width = (breakpoint - 1 / 3).toFixed(4);
      const wrapper = join(dir, `wrap-${breakpoint}.html`);
      writeFileSync(wrapper, `<!DOCTYPE html><body style="margin:0"><iframe style="width:${width}px;height:800px;border:0" src="${url}"></iframe>`);
      await page.goto(pathToFileURL(wrapper).href);
      const frame = page.frames()[1];
      await frame.waitForFunction(() => document.documentElement.classList.contains('js'));
      for (const legend of ['shown', 'hidden']) {
        if (legend === 'hidden') await frame.locator('.legend-toggle').click();
        const m = await frame.evaluate(() => {
          const query = document.documentElement.hasAttribute('data-legend') ? 'data-narrow-query-hidden' : 'data-narrow-query';
          const showing = [...document.querySelectorAll('.arrangement')].filter((a) => getComputedStyle(a).display !== 'none').map((a) => a.classList[1]);
          return { viewport: document.documentElement.getBoundingClientRect().width, showing, narrow: matchMedia(document.querySelector('main').getAttribute(query)).matches };
        });
        const agrees = m.showing.length === 1 && (m.showing[0] === 'narrow') === m.narrow;
        check(name, agrees, `1.5x, ${m.viewport} px, legend ${legend}: one arrangement, as matchMedia says: ${JSON.stringify(m)}`);
      }
    }
  } finally {
    await browser.close();
  }
}
process.exit(failed ? 1 : 0);
