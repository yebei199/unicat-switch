import { hex, mix, type RGB, smoothstep } from './color.ts';

/** 一天里随 t（0 白天 → 0.5 黄昏 → 1 夜里）连续变化的全部光照参数。 */
export type Daylight = {
  skyTop: RGB;
  skyHorizon: RGB;
  light: RGB;
  dusk: number;
  night: number;
};

type Key = Pick<Daylight, 'skyTop' | 'skyHorizon' | 'light'>;

const DAY: Key = { skyTop: hex('#79a6da'), skyHorizon: hex('#e6eef0'), light: [1, 1, 1] };
const DUSK: Key = { skyTop: hex('#4b5486'), skyHorizon: hex('#f3a447'), light: [1, 0.74, 0.6] };
const NIGHT: Key = { skyTop: hex('#090d27'), skyHorizon: hex('#1d2350'), light: [0.27, 0.31, 0.55] };

function between(a: Key, b: Key, u: number): Key {
  return {
    skyTop: mix(a.skyTop, b.skyTop, u),
    skyHorizon: mix(a.skyHorizon, b.skyHorizon, u),
    light: mix(a.light, b.light, u),
  };
}

export function daylight(t: number): Daylight {
  const key = t < 0.5 ? between(DAY, DUSK, smoothstep(0, 0.5, t)) : between(DUSK, NIGHT, smoothstep(0.5, 1, t));
  return { ...key, dusk: 1 - Math.abs(2 * t - 1), night: smoothstep(0.55, 0.95, t) };
}
