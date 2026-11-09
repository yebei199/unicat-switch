import { hex, lerp, mix, type RGB, smoothstep } from './color.ts';
import type { Daylight } from './daylight.ts';
import { H, HORIZON, W } from './layout.ts';
import type { Raster } from './raster.ts';

const SUN = hex('#fff4c8');
const SUNSET = hex('#ff9a4a');
const MOON = hex('#f1eedc');
const CRATER = hex('#c9c4ad');
const STAR = hex('#fffbe6');

const CLOUDS = [
  { x: 10, y: 9, shape: ['...####.....', '.#########..', '############'] },
  { x: 120, y: 5, shape: ['....###...', '.########.', '##########'] },
  { x: 80, y: 24, shape: ['..####.......', '############.', '#############'] },
];

function mulberry32(seed: number): () => number {
  let a = seed;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const rand = mulberry32(7);
const STARS = Array.from({ length: 48 }, () => ({
  x: Math.floor(rand() * W),
  y: Math.floor(rand() * (HORIZON - 8)),
  rate: 1 + rand() * 2.5,
  phase: rand() * Math.PI * 2,
  big: rand() < 0.12,
}));

function gradient(dst: Raster, day: Daylight): void {
  for (let y = 0; y < H; y++) {
    const c = mix(day.skyTop, day.skyHorizon, Math.min(1, y / HORIZON) ** 1.5);
    for (let x = 0; x < W; x++) dst.set(x, y, c);
  }
}

/** 太阳从中上方往右下落，月亮从左下升到左上；t 在 0.45–0.55 之间两者都在地平线附近。 */
function celestial(dst: Raster, t: number, day: Daylight): void {
  const s = smoothstep(0, 0.55, t);
  if (s < 1) {
    const sun = mix(SUN, SUNSET, day.dusk);
    const x = lerp(100, 236, s);
    const y = 12 + 62 * s ** 1.6;
    dst.halo(x, y, 12, sun, 0.45);
    dst.disc(x, y, 4.5, sun);
  }
  const m = smoothstep(0.45, 1, t);
  if (m > 0) {
    const x = lerp(-10, 86, m);
    const y = 14 + 60 * (1 - m) ** 1.6;
    dst.halo(x, y, 10, MOON, 0.18 * m);
    dst.disc(x, y, 4, MOON);
    dst.blend(Math.floor(x) - 1, Math.floor(y) - 1, CRATER, 1);
    dst.blend(Math.floor(x) + 1, Math.floor(y) + 1, CRATER, 1);
  }
}

function stars(dst: Raster, night: number, time: number): void {
  if (night <= 0) return;
  for (const s of STARS) {
    const a = night * (0.55 + 0.45 * Math.sin(time * s.rate + s.phase));
    dst.blend(s.x, s.y, STAR, a);
    if (!s.big) continue;
    for (const [dx, dy] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ] as const)
      dst.blend(s.x + dx, s.y + dy, STAR, a * 0.45);
  }
}

function clouds(dst: Raster, day: Daylight, time: number): void {
  const lit = mix(day.skyTop, day.light, 0.82);
  const shade: RGB = [lit[0] * 0.86, lit[1] * 0.88, lit[2] * 0.93];
  for (const cloud of CLOUDS) {
    const width = cloud.shape[0]?.length ?? 0;
    const x0 = Math.floor(((cloud.x + time * 1.2) % (W + width)) - width);
    cloud.shape.forEach((row, dy) => {
      const c = dy === cloud.shape.length - 1 ? shade : lit;
      for (let dx = 0; dx < row.length; dx++) if (row[dx] === '#') dst.blend(x0 + dx, cloud.y + dy, c, 1);
    });
  }
}

export function sky(dst: Raster, t: number, day: Daylight, time: number): void {
  gradient(dst, day);
  stars(dst, day.night, time);
  celestial(dst, t, day);
  clouds(dst, day, time);
}
