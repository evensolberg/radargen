// RG-102 in Firefox and WebKit, where a click need not focus a button (as
// in WebKit and Firefox on macOS): a focused legend row hidden by a click on
// the legend button hands focus to the button, and both legends hide. The
// CDP checks drive Chromium only, which focuses a clicked button.
//
// Playwright's locator.focus() sets up the focused row; the click itself is
// a real one.
import { mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

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

// A hidden legend is removed (narrow) or invisible in place (wide).
const HIDDEN = () =>
  [...document.querySelectorAll('.wide-legend,.narrow-legend')].every((p) => {
    const s = getComputedStyle(p);
    return s.display === 'none' || s.visibility === 'hidden';
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
    await page.goto(url);
    await page.locator('.arrangement.wide [data-legend-row]').first().focus();
    await page.locator('.legend-toggle').click();
    const focused = await page.evaluate(() => document.activeElement?.classList.contains('legend-toggle'));
    const hidden = await page.evaluate(HIDDEN);
    for (const [ok, what] of [[focused, 'focus moves to the button'], [hidden, 'both legends hide']]) {
      console.log(`${ok ? 'ok' : 'FAIL'} ${name}: ${what}`);
      failed ||= !ok;
    }
  } finally {
    await browser.close();
  }
}
process.exit(failed ? 1 : 0);
