// 录 README 顶部的演示 GIF：页面以 ?capture 打开，逐帧推进时间并截图，再交给 ffmpeg。
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { $ } from 'bun';
import { chromium, type Page } from 'playwright-core';
import index from '../index.html';

const FPS = 20;
const OUT = 'docs/demo.gif';
const MARGIN = 28;
// 画面本身已是抖动过的像素画，调色板映射时不再叠一层抖动。
const GIF_FILTER = 'split[a][b];[a]palettegen=max_colors=256:stats_mode=full[p];[b][p]paletteuse=dither=none';
// 截图里看不到系统光标，注入一个跟随 pointermove 的箭头，观众才知道是在拖。
const CURSOR = `addEventListener('DOMContentLoaded', () => {
  const c = document.createElement('div');
  c.style.cssText = 'position:fixed;left:0;top:0;pointer-events:none;z-index:9;transform:translate(-99px,-99px)';
  c.innerHTML = '<svg width="22" height="22" viewBox="0 0 20 20"><path d="M2 1L2 16L6 12.5L9 19L11.5 18L8.6 11.6L14 11.6Z" fill="#fff" stroke="#111" stroke-width="1.3" stroke-linejoin="round"/></svg>';
  document.body.append(c);
  addEventListener('pointermove', (e) => { c.style.transform = 'translate(' + (e.clientX - 2) + 'px,' + (e.clientY - 1) + 'px)'; }, true);
});`;

type Point = { x: number; y: number };
type Clip = Point & { width: number; height: number };

class Recorder {
  private n = 0;
  private at: Point = { x: 0, y: 0 };

  constructor(
    private readonly page: Page,
    private readonly dir: string,
    private readonly clip: Clip,
  ) {}

  async tick(): Promise<void> {
    await this.page.evaluate(
      (dt) => (window as unknown as { unicat: { frame(dt: number): void } }).unicat.frame(dt),
      1 / FPS,
    );
    await this.page.screenshot({ path: join(this.dir, `${String(this.n++).padStart(4, '0')}.png`), clip: this.clip });
  }

  async hold(seconds: number): Promise<void> {
    for (let i = 0; i < Math.round(seconds * FPS); i++) await this.tick();
  }

  async jump(to: Point): Promise<void> {
    this.at = to;
    await this.page.mouse.move(to.x, to.y);
  }

  /** 缓入缓出地把光标移到 to，每帧一步。 */
  async glide(to: Point, seconds: number): Promise<void> {
    const from = this.at;
    const frames = Math.round(seconds * FPS);
    for (let i = 1; i <= frames; i++) {
      const u = i / frames;
      const e = u * u * (3 - 2 * u);
      await this.page.mouse.move(from.x + (to.x - from.x) * e, from.y + (to.y - from.y) * e);
      await this.tick();
    }
    this.at = to;
  }

  async press(): Promise<void> {
    await this.page.mouse.down();
    await this.hold(0.12);
  }

  async release(): Promise<void> {
    await this.page.mouse.up();
  }
}

/** 白天 → 点击入夜（猫走回门口）→ 拖回黄昏停住 → 松手回到白天（猫走回树下），首尾相接循环。 */
async function storyboard(rec: Recorder, box: Clip): Promise<void> {
  const y = box.y + box.height / 2;
  const at = (u: number): Point => ({ x: box.x + box.height / 2 + (box.width - box.height) * u, y });
  await rec.jump({ x: box.x + box.width * 0.62, y: box.y + box.height + 40 });
  await rec.hold(0.6);
  await rec.glide(at(0.02), 0.7);
  await rec.press();
  await rec.release();
  await rec.hold(3.6);
  await rec.glide(at(1), 0.5);
  await rec.press();
  await rec.glide(at(0.48), 1.8);
  await rec.hold(1.2);
  await rec.glide(at(0.1), 0.8);
  await rec.release();
  await rec.hold(0.4);
  await rec.glide({ x: box.x + box.width * 0.62, y: box.y + box.height + 40 }, 0.8);
  await rec.hold(2.6);
}

const server = Bun.serve({ port: 0, routes: { '/': index } });
const executablePath = process.env.CHROME ?? Bun.which('google-chrome-stable') ?? Bun.which('chromium');
if (!executablePath) throw new Error('需要 Chrome/Chromium（或用 CHROME= 指定）');
const browser = await chromium.launch({
  executablePath,
  args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
const dir = mkdtempSync(join(tmpdir(), 'unicat-record-'));
try {
  const page = await browser.newPage({
    viewport: { width: 640, height: 340 },
    deviceScaleFactor: 1,
    reducedMotion: 'no-preference',
  });
  await page.addInitScript(CURSOR);
  await page.goto(`${server.url}?capture`);
  await page.waitForFunction(() => 'unicat' in window);
  const box = await page.locator('.switch canvas').boundingBox();
  const shell = await page.locator('.switch').boundingBox();
  if (!box || !shell) throw new Error('找不到开关');
  const clip = {
    x: shell.x - MARGIN,
    y: shell.y - MARGIN,
    width: shell.width + 2 * MARGIN,
    height: shell.height + 2 * MARGIN,
  };
  await storyboard(new Recorder(page, dir, clip), box);
  await $`ffmpeg -loglevel error -y -framerate ${FPS} -i ${dir}/%04d.png -vf ${GIF_FILTER} -loop 0 ${OUT}`;
  console.log(OUT);
} finally {
  await browser.close();
  await server.stop();
  rmSync(dir, { recursive: true, force: true });
}
