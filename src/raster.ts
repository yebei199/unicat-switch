import type { RGB } from './color.ts';

/** 线性 RGB 浮点画布；所有图层先在这里合成，最后统一抖动量化。 */
export class Raster {
  readonly rgb: Float32Array;

  constructor(
    readonly w: number,
    readonly h: number,
  ) {
    this.rgb = new Float32Array(w * h * 3);
  }

  inside(x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x < this.w && y < this.h;
  }

  set(x: number, y: number, c: RGB): void {
    const i = (y * this.w + x) * 3;
    this.rgb[i] = c[0];
    this.rgb[i + 1] = c[1];
    this.rgb[i + 2] = c[2];
  }

  blend(x: number, y: number, c: RGB, a: number): void {
    if (!this.inside(x, y)) return;
    const i = (y * this.w + x) * 3;
    for (let k = 0; k < 3; k++) {
      const v = this.rgb[i + k] ?? 0;
      this.rgb[i + k] = v + ((c[k] ?? 0) - v) * a;
    }
  }

  add(x: number, y: number, c: RGB, k: number): void {
    if (!this.inside(x, y)) return;
    const i = (y * this.w + x) * 3;
    for (let n = 0; n < 3; n++) this.rgb[i + n] = (this.rgb[i + n] ?? 0) + (c[n] ?? 0) * k;
  }

  /** 实心圆盘，边缘不做抗锯齿：像素画要硬边。 */
  disc(cx: number, cy: number, r: number, c: RGB): void {
    for (let y = Math.floor(cy - r); y <= cy + r; y++)
      for (let x = Math.floor(cx - r); x <= cx + r; x++)
        if ((x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2 <= r * r) this.blend(x, y, c, 1);
  }

  /** 径向衰减的加法光晕，交给后面的抖动画成网点。 */
  halo(cx: number, cy: number, r: number, c: RGB, k: number): void {
    for (let y = Math.floor(cy - r); y <= cy + r; y++)
      for (let x = Math.floor(cx - r); x <= cx + r; x++) {
        const d = Math.hypot(x + 0.5 - cx, y + 0.5 - cy) / r;
        if (d < 1) this.add(x, y, c, k * (1 - d) ** 2);
      }
  }
}

/** ImageData 的一个矩形区域（精灵帧）。 */
export type Sprite = { image: ImageData; sx: number; sy: number; sw: number; sh: number };

/** 把精灵按 tint 相乘后贴上去；flip 水平翻转。 */
export function blit(dst: Raster, s: Sprite, dx: number, dy: number, flip: boolean, tint: RGB): void {
  const d = s.image.data;
  for (let y = 0; y < s.sh; y++)
    for (let x = 0; x < s.sw; x++) {
      const i = ((s.sy + y) * s.image.width + s.sx + (flip ? s.sw - 1 - x : x)) * 4;
      if ((d[i + 3] ?? 0) < 128) continue;
      const c: RGB = [
        ((d[i] ?? 0) / 255) * tint[0],
        ((d[i + 1] ?? 0) / 255) * tint[1],
        ((d[i + 2] ?? 0) / 255) * tint[2],
      ];
      dst.blend(dx + x, dy + y, c, 1);
    }
}
