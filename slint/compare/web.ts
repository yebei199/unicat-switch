// Writes the web reference frames for the Slint comparison: bun slint/compare/web.ts <outdir>
// States mirror examples/snapshot.rs: 160 frames of 1/20 s, then the settled day, the drag held
// at the middle (dusk, ball pressed), and night.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { chromium } from 'playwright-core';
import harness from './harness.html';
import type { State } from './harness.ts';

const out = process.argv[2];
if (!out) throw new Error('usage: bun slint/compare/web.ts <outdir>');
let time = 0;
for (let i = 0; i < 160; i++) time += 0.05;
const size = { width: 516, height: 221, time };
const states: Record<string, State> = {
  day: { ...size, t: 0, u: 0, press: 1, night: false },
  dusk: { ...size, t: 0.5, u: 0.5, press: 0.93, night: false },
  night: { ...size, t: 1, u: 1, press: 1, night: true },
};

const server = Bun.serve({ port: 0, routes: { '/': harness } });
const executablePath = process.env.CHROME ?? Bun.which('google-chrome-stable') ?? Bun.which('chromium');
if (!executablePath) throw new Error('needs Chrome/Chromium (or CHROME=)');
const browser = await chromium.launch({
  executablePath,
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
try {
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  await page.goto(server.url.href);
  await page.waitForFunction(() => 'reference' in window);
  for (const [name, state] of Object.entries(states)) {
    const url = await page.evaluate((s) => (window as unknown as { reference(s: State): string }).reference(s), state);
    writeFileSync(join(out, `web-${name}.png`), Buffer.from(url.split(',')[1] ?? '', 'base64'));
    console.log(join(out, `web-${name}.png`));
  }
} finally {
  await browser.close();
  await server.stop();
}
