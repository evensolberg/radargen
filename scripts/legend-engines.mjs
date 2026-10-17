// RG-102 in Firefox and WebKit, where a click need not focus a button (as
// in WebKit and Firefox on macOS): a focused legend row hidden by a click on
// the legend button hands focus to the button, and both legends hide. The
// CDP checks drive Chromium only, which focuses a clicked button.
//
// Playwright's locator.focus() sets up the focused row; the click itself is
// a real one. Only WebKit tells the guard apart everywhere: Firefox focuses
// a clicked button except on macOS, so on Linux it passes either way.
import { mkdirSync } from 'node:fs';
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
for (const name of ['firefox', 'webkit']) {
  const browser = await playwright[name].launch(LAUNCH[name]);
  try {
    const page = await (await browser.newContext({ viewport: { width: 1400, height: 900 } })).newPage();
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(url);
    await page.locator('.arrangement.wide [data-legend-row]').first().focus();
    const onRow = await page.evaluate(() => document.activeElement?.hasAttribute('data-legend-row'));
    await page.locator('.legend-toggle').click();
    const focused = await page.evaluate(() => document.activeElement?.classList.contains('legend-toggle'));
    const state = legendState(await page.evaluate(LEGEND_PARTS));
    const hidden = state === 'hidden';
    for (const [ok, what] of [
      [onRow, 'a legend row had focus before the click'],
      [focused, 'focus moves to the button'],
      [hidden, `both legends hide (saw ${state})`],
      [errors.length === 0, `no script errors: ${errors}`],
    ]) {
      console.log(`${ok ? 'ok' : 'FAIL'} ${name}: ${what}`);
      failed ||= !ok;
    }
  } finally {
    await browser.close();
  }
}
process.exit(failed ? 1 : 0);
