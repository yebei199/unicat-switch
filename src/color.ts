export type RGB = readonly [number, number, number];

export const lerp = (a: number, b: number, u: number): number => a + (b - a) * u;

export const clamp01 = (x: number): number => Math.min(1, Math.max(0, x));

export function smoothstep(e0: number, e1: number, x: number): number {
  const u = clamp01((x - e0) / (e1 - e0));
  return u * u * (3 - 2 * u);
}

export const mix = (a: RGB, b: RGB, u: number): RGB => [lerp(a[0], b[0], u), lerp(a[1], b[1], u), lerp(a[2], b[2], u)];

export function hex(code: string): RGB {
  const n = Number.parseInt(code.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

export const css = (c: RGB, alpha = 1): string =>
  `rgb(${c.map((v) => Math.round(clamp01(v) * 255)).join(' ')} / ${alpha})`;
