import sitUrl from '../art/cat-sit.png';
import walkUrl from '../art/cat-walk.png';
import sceneUrl from '../art/scene.png';
import { Cat } from './cat.ts';
import { clamp01, css, hex, lerp, mix } from './color.ts';
import { daylight } from './daylight.ts';
import { type Ball, Glass } from './glass.ts';
import { Scene } from './scene.ts';
import { Spring } from './spring.ts';

const PAGE = [hex('#ece5d4'), hex('#0a0d1d')] as const;
const BEZEL_HI = [hex('#fdfaf3'), hex('#3b3b55')] as const;
const BEZEL_LO = [hex('#d8cfbc'), hex('#17182b')] as const;
const BALL_FILL = 0.88;

async function pixels(url: string): Promise<ImageData> {
  const img = new Image();
  img.src = url;
  await img.decode();
  const ctx = new OffscreenCanvas(img.width, img.height).getContext('2d');
  if (!ctx) throw new Error('2d context unavailable');
  ctx.drawImage(img, 0, 0);
  return ctx.getImageData(0, 0, img.width, img.height);
}

function theme(root: HTMLElement, t: number, night: number): void {
  root.style.setProperty('--page', css(mix(PAGE[0], PAGE[1], t)));
  root.style.setProperty('--hi', css(mix(BEZEL_HI[0], BEZEL_HI[1], t)));
  root.style.setProperty('--lo', css(mix(BEZEL_LO[0], BEZEL_LO[1], t)));
  root.style.setProperty('--glow', String(night));
  root.style.setProperty('--drop', String(1 - night));
}

class Switch {
  private checked = false;
  private pointer: number | null = null;
  private pressed = false;
  private readonly cat = new Cat();
  private readonly pos: Spring;
  private readonly sky: Spring;
  private readonly lean = new Spring(0, 220, 22);
  private readonly press = new Spring(1, 420, 26);

  constructor(
    private readonly button: HTMLButtonElement,
    private readonly glass: Glass,
    private readonly scene: Scene,
    calm: boolean,
  ) {
    // 欠阻尼：球冲过头再弹回；天色用更慢的临界阻尼，让黄昏停留得久一点。
    this.pos = calm ? new Spring(0, 400, 40) : new Spring(0, 190, 15);
    this.sky = calm ? new Spring(0, 40, 12.6) : new Spring(0, 12, 6.9);
    button.addEventListener('click', () => this.toggle());
    button.addEventListener('pointermove', (e) => {
      this.pointer = this.devicePixels(e.clientX);
    });
    button.addEventListener('pointerleave', () => {
      this.pointer = null;
      this.pressed = false;
    });
    button.addEventListener('pointerdown', () => {
      this.pressed = true;
    });
    window.addEventListener('pointerup', () => {
      this.pressed = false;
    });
  }

  private devicePixels(clientX: number): number {
    const canvas = this.glass.canvas;
    const rect = canvas.getBoundingClientRect();
    return ((clientX - rect.left) * canvas.width) / rect.width;
  }

  private toggle(): void {
    this.checked = !this.checked;
    this.button.setAttribute('aria-checked', String(this.checked));
  }

  private ball(dt: number): Ball {
    const { width: w, height: h } = this.glass.canvas;
    const r = h * 0.5 * BALL_FILL;
    const margin = h * 0.5 - r;
    const x0 = margin + r;
    const x1 = w - margin - r;
    const x = lerp(x0, x1, this.pos.step(this.checked ? 1 : 0, dt));
    const toward = this.pointer === null ? 0 : (this.pointer - x) * 0.15;
    const lean = this.lean.step(Math.max(-0.14 * r, Math.min(0.14 * r, toward)), dt);
    const stretch = 1 + Math.min(Math.abs(this.pos.velocity * (x1 - x0)) / (r * 28), 0.22);
    const s = this.press.step(this.pressed ? 0.93 : 1, dt);
    return { x: x + lean, y: h / 2, rx: r * stretch * s, ry: (r / stretch) * s };
  }

  frame(dt: number, time: number): void {
    const t = clamp01(this.sky.step(this.checked ? 1 : 0, dt));
    const night = daylight(t).night;
    this.cat.update(dt, this.checked);
    const ball = this.ball(dt);
    this.glass.draw(this.scene.render(t, time, this.cat), ball, night);
    theme(document.documentElement, t, night);
  }
}

function fitCanvas(canvas: HTMLCanvasElement): void {
  new ResizeObserver(([entry]) => {
    if (!entry) return;
    const box = entry.devicePixelContentBoxSize?.[0];
    canvas.width = box ? box.inlineSize : Math.round(entry.contentRect.width * devicePixelRatio);
    canvas.height = box ? box.blockSize : Math.round(entry.contentRect.height * devicePixelRatio);
  }).observe(canvas);
}

async function main(): Promise<void> {
  const button = document.querySelector<HTMLButtonElement>('.switch');
  const canvas = button?.querySelector('canvas');
  if (!button || !canvas) throw new Error('missing .switch canvas');
  fitCanvas(canvas);
  const [scene, sit, walk] = await Promise.all([pixels(sceneUrl), pixels(sitUrl), pixels(walkUrl)]);
  const calm = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const view = new Switch(button, new Glass(canvas), new Scene({ scene, sit, walk }), calm);
  let last = performance.now();
  const tick = (now: number): void => {
    view.frame(Math.min((now - last) / 1000, 0.05), now / 1000);
    last = now;
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

await main();
