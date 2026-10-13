// Screenshots for the release's visual sign-off (plan RG-015C, Task 4).
//
// Usage: node scripts/screenshots.mjs <out-dir> <page.html> ...
//
// Each page at 1400 px, the breakpoint, the breakpoint less one, 400 and
// 320 px, in light, dark and forced colours, with the panel closed and then
// open (by a trusted click, the mouse then moved off so no hover lingers).
// Nothing is compared: font rendering varies across machines, so a person
// looks at them against docs/release-1.0.0.md's list.

import { writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { SHOWN, connect } from './cdp.mjs';

const [outDir, ...pages] = process.argv.slice(2);
if (!outDir || pages.length === 0) {
  console.error('usage: node scripts/screenshots.mjs <out-dir> <page.html> ...');
  process.exit(2);
}

const { send, close, evaluate, navigate } = await connect();
await send('Page.enable');
const shoot = async (name) => {
  const shot = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
  writeFileSync(join(outDir, `${name}.png`), Buffer.from(shot.result.data, 'base64'));
};
const modes = [
  ['light', [{ name: 'prefers-color-scheme', value: 'light' }]],
  ['dark', [{ name: 'prefers-color-scheme', value: 'dark' }]],
  ['forced', [{ name: 'forced-colors', value: 'active' }]],
];

let count = 0;
try {
  for (const file of pages) {
    const url = pathToFileURL(resolve(file)).href;
    await send('Emulation.setDeviceMetricsOverride', { width: 1400, height: 900, deviceScaleFactor: 1, mobile: false });
    await navigate(url);
    const query = await evaluate(`document.querySelector('main').getAttribute('data-narrow-query')`);
    const last = Number(query.match(/\d+/)[0]);
    for (const width of [1400, last + 1, last, 400, 320]) {
      for (const [mode, features] of modes) {
        await send('Emulation.setEmulatedMedia', { features });
        await send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
        await navigate(url);
        await evaluate('document.fonts.ready.then(() => true)');
        const stem = `${basename(file, '.html')}-${width}-${mode}`;
        await shoot(`${stem}-closed`);
        const at = await evaluate(`(() => {
          const shown = ${SHOWN};
          const m = shown.querySelector('[data-marker] circle');
          if (!m) return null;
          m.scrollIntoView({ block: 'center' });
          const r = m.getBoundingClientRect();
          return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
        })()`);
        if (at) {
          for (const type of ['mouseMoved', 'mousePressed', 'mouseReleased']) {
            await send('Input.dispatchMouseEvent', { type, x: at.x, y: at.y, button: 'left', clickCount: 1 });
          }
          await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 2, y: 2 });
          await shoot(`${stem}-open`);
          count += 1;
        }
        count += 1;
      }
    }
  }
} finally {
  await close();
}
console.log(`${count} screenshots in ${outDir}`);
