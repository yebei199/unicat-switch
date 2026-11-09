import type { Cat } from './cat.ts';
import { hex, mix, type RGB } from './color.ts';
import { daylight } from './daylight.ts';
import { posterize } from './dither.ts';
import { GLOW, H, W, WINDOW } from './layout.ts';
import { blit, Raster } from './raster.ts';
import { sky } from './sky.ts';

const LEVELS = 14;
const LAMP = hex('#ffcf73');
const POOL = hex('#ff9440');
const GLASS = hex('#46598f');

export type Art = { scene: ImageData; sit: ImageData; walk: ImageData };

/** 窗玻璃 = 窗框范围内的黄色像素；白天换成蓝玻璃，夜里发暖光。 */
function windowPixels(scene: ImageData): Set<number> {
  const found = new Set<number>();
  const d = scene.data;
  for (let y = WINDOW.y0; y <= WINDOW.y1; y++)
    for (let x = WINDOW.x0; x <= WINDOW.x1; x++) {
      const i = (y * W + x) * 4;
      if ((d[i] ?? 0) > 140 && (d[i + 1] ?? 0) > 140 && (d[i + 2] ?? 0) < 110) found.add(y * W + x);
    }
  return found;
}

export class Scene {
  readonly image = new ImageData(W, H);
  private readonly raster = new Raster(W, H);
  private readonly glass: Set<number>;

  constructor(private readonly art: Art) {
    this.glass = windowPixels(art.scene);
  }

  render(t: number, time: number, cat: Cat): ImageData {
    const day = daylight(t);
    sky(this.raster, t, day, time);
    this.foreground(day.light, day.night);
    this.cat(cat, day.light);
    this.lamplight(day.night);
    posterize(this.raster, this.image, LEVELS);
    return this.image;
  }

  private foreground(light: RGB, night: number): void {
    const d = this.art.scene.data;
    const lit = mix(GLASS, LAMP, night);
    for (let y = 0; y < H; y++)
      for (let x = 0; x < W; x++) {
        const i = (y * W + x) * 4;
        if ((d[i + 3] ?? 0) < 128) continue;
        if (this.glass.has(y * W + x)) {
          this.raster.set(x, y, mix([GLASS[0] * light[0], GLASS[1] * light[1], GLASS[2] * light[2]], lit, night));
          continue;
        }
        this.raster.set(x, y, [
          ((d[i] ?? 0) / 255) * light[0],
          ((d[i + 1] ?? 0) / 255) * light[1],
          ((d[i + 2] ?? 0) / 255) * light[2],
        ]);
      }
  }

  /** 窗光洒在墙上和门前草地上；在猫之后叠加，夜里坐在门口的猫也被照亮。 */
  private lamplight(night: number): void {
    if (night <= 0) return;
    const cx = (WINDOW.x0 + WINDOW.x1) / 2;
    const cy = (WINDOW.y0 + WINDOW.y1) / 2;
    this.raster.halo(cx, cy, 16, LAMP, 0.32 * night);
    for (let y = GLOW.y - GLOW.ry; y <= GLOW.y + GLOW.ry; y++)
      for (let x = GLOW.x - GLOW.rx; x <= GLOW.x + GLOW.rx; x++) {
        const f = 1 - ((x - GLOW.x) / GLOW.rx) ** 2 - ((y - GLOW.y) / GLOW.ry) ** 2;
        if (f > 0) this.raster.add(x, y, POOL, 0.42 * night * f);
      }
  }

  private cat(cat: Cat, light: RGB): void {
    const sheet = cat.walking ? this.art.walk : this.art.sit;
    const sw = cat.walking ? sheet.width / 4 : sheet.width;
    const sprite = { image: sheet, sx: cat.walking ? cat.frame * sw : 0, sy: 0, sw, sh: sheet.height };
    // 走路帧朝右，坐姿朝左；坐下时不翻转，让猫看向房子。
    const flip = cat.walking && cat.facing < 0;
    blit(this.raster, sprite, Math.round(cat.x - sw / 2), cat.baseline - sheet.height, flip, light);
  }
}
