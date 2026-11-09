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
const DRAG_SLOP = 4;
const FLICK = 1.5;

type Track = { r: number; x0: number; x1: number; h: number };
type Drag = { startX: number; startTarget: number; moved: boolean; lastX: number; lastTime: number; velocity: number };

/** 拖过两端时按 1/4 的比例继续走，松手再弹回。 */
const rubber = (v: number): number => (v < 0 ? v * 0.25 : v > 1 ? 1 + (v - 1) * 0.25 : v);

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
  private target = 0;
  private pointer: number | null = null;
  private pressed = false;
  private drag: Drag | null = null;
  private swallowClick = false;
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
    button.addEventListener('click', () => this.click());
    button.addEventListener('pointerdown', (e) => this.down(e));
    button.addEventListener('pointermove', (e) => this.move(e));
    button.addEventListener('pointerup', () => this.up());
    button.addEventListener('pointercancel', () => this.up());
    button.addEventListener('pointerleave', () => {
      this.pointer = null;
    });
  }

  private down(e: PointerEvent): void {
    this.pressed = true;
    this.swallowClick = false;
    const x = this.devicePixels(e.clientX);
    this.drag = { startX: x, startTarget: this.target, moved: false, lastX: x, lastTime: e.timeStamp, velocity: 0 };
    this.button.setPointerCapture(e.pointerId);
  }

  private move(e: PointerEvent): void {
    const x = this.devicePixels(e.clientX);
    this.pointer = x;
    const drag = this.drag;
    if (!drag) return;
    const dx = x - drag.startX;
    if (!drag.moved && Math.abs(dx) < DRAG_SLOP * devicePixelRatio) return;
    drag.moved = true;
    const { x0, x1 } = this.track();
    this.target = rubber(drag.startTarget + dx / (x1 - x0));
    const dt = (e.timeStamp - drag.lastTime) / 1000;
    if (dt > 0) drag.velocity = lerp(drag.velocity, (x - drag.lastX) / (x1 - x0) / dt, 0.5);
    drag.lastX = x;
    drag.lastTime = e.timeStamp;
  }

  /** 拖动松手：快速甩动按方向定，否则停在离哪边近就去哪边；随后那次 click 不再切换。 */
  private up(): void {
    this.pressed = false;
    const drag = this.drag;
    this.drag = null;
    if (!drag?.moved) return;
    this.swallowClick = true;
    this.commit(Math.abs(drag.velocity) > FLICK ? drag.velocity > 0 : this.target > 0.5);
  }

  private click(): void {
    if (this.swallowClick) {
      this.swallowClick = false;
      return;
    }
    this.commit(!this.checked);
  }

  private devicePixels(clientX: number): number {
    const canvas = this.glass.canvas;
    const rect = canvas.getBoundingClientRect();
    return ((clientX - rect.left) * canvas.width) / rect.width;
  }

  private commit(checked: boolean): void {
    this.checked = checked;
    this.target = checked ? 1 : 0;
    this.button.setAttribute('aria-checked', String(checked));
  }

  private track(): Track {
    const { width: w, height: h } = this.glass.canvas;
    const r = h * 0.5 * BALL_FILL;
    const margin = h * 0.5 - r;
    return { r, x0: margin + r, x1: w - margin - r, h };
  }

  private ball(dt: number): Ball {
    const { r, x0, x1, h } = this.track();
    const x = lerp(x0, x1, this.pos.step(this.target, dt));
    const toward = this.pointer === null || this.drag?.moved ? 0 : (this.pointer - x) * 0.15;
    const lean = this.lean.step(Math.max(-0.14 * r, Math.min(0.14 * r, toward)), dt);
    const stretch = 1 + Math.min(Math.abs(this.pos.velocity * (x1 - x0)) / (r * 28), 0.22);
    const s = this.press.step(this.pressed ? 0.93 : 1, dt);
    return { x: x + lean, y: h / 2, rx: r * stretch * s, ry: (r / stretch) * s };
  }

  frame(dt: number, time: number): void {
    const t = clamp01(this.sky.step(clamp01(this.target), dt));
    const night = daylight(t).night;
    this.cat.update(dt, this.target > 0.5);
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
