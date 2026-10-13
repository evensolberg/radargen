// What a Tab costs in style recalculation on a large radar (rg-jri).
//
// Usage: node scripts/focus-cost.mjs <page.html> [tabs]
//
// It Tabs through the page with trusted key events, forcing layout after
// each, and prints the mean RecalcStyleDuration and LayoutDuration per Tab
// from Chromium's Performance domain. PR 14b's per-key :has rules cost
// about 49 ms per Tab at 300 markers; showing focus by attribute should
// cost a small fraction of a millisecond.

import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { connect } from './cdp.mjs';

const [file, count = '40'] = process.argv.slice(2);
if (!file) {
  console.error('usage: node scripts/focus-cost.mjs <page.html> [tabs]');
  process.exit(2);
}
const tabs = Number(count);

const { send, close, evaluate, navigate, press } = await connect();
try {
  await send('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Page.enable');
  await navigate(pathToFileURL(resolve(file)).href);
  await send('Performance.enable');
  const metrics = async () =>
    Object.fromEntries((await send('Performance.getMetrics')).result.metrics.map((m) => [m.name, m.value]));
  const before = await metrics();
  for (let i = 0; i < tabs; i++) {
    await press('Tab');
    await evaluate('document.body.offsetHeight');
  }
  const after = await metrics();
  const per = (name) => (((after[name] - before[name]) * 1000) / tabs).toFixed(2);
  console.log(`${tabs} Tabs: ${per('RecalcStyleDuration')} ms style recalc, ${per('LayoutDuration')} ms layout, per Tab`);
} finally {
  await close();
}
