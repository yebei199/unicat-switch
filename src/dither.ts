import type { Raster } from './raster.ts';

const BAYER4 = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];

const GAMMA = 1.8;

/** 4×4 Bayer 有序抖动，在 gamma 空间把每个通道量化到 levels 级（暗部台阶更细，夜景不会满屏噪点）。 */
export function posterize(src: Raster, out: ImageData, levels: number): void {
  const steps = levels - 1;
  const data = out.data;
  for (let y = 0; y < src.h; y++)
    for (let x = 0; x < src.w; x++) {
      const threshold = ((BAYER4[(y & 3) * 4 + (x & 3)] ?? 0) + 0.5) / 16;
      const i = y * src.w + x;
      for (let k = 0; k < 3; k++) {
        const q = Math.min(1, Math.max(0, src.rgb[i * 3 + k] ?? 0)) ** (1 / GAMMA) * steps;
        const base = Math.floor(q);
        data[i * 4 + k] = ((base + (q - base > threshold ? 1 : 0)) / steps) ** GAMMA * 255;
      }
      data[i * 4 + 3] = 255;
    }
}
