// Renders the web pipeline (scene.ts + glass.ts, untouched) at fixed states so the Slint port can
// be diffed against it. Loaded by web.ts; exposes window.reference(state) → PNG data URL.
import sitUrl from '../../art/cat-sit.png';
import walkUrl from '../../art/cat-walk.png';
import sceneUrl from '../../art/scene.png';
import { Cat } from '../../src/cat.ts';
import { daylight } from '../../src/daylight.ts';
import { Glass } from '../../src/glass.ts';
import { DOOR, TREE } from '../../src/layout.ts';
import { Scene } from '../../src/scene.ts';

export type State = {
  width: number;
  height: number;
  t: number;
  u: number;
  press: number;
  night: boolean;
  time: number;
};

async function pixels(url: string): Promise<ImageData> {
  const img = new Image();
  img.src = url;
  await img.decode();
  const ctx = new OffscreenCanvas(img.width, img.height).getContext('2d');
  if (!ctx) throw new Error('2d context unavailable');
  ctx.drawImage(img, 0, 0);
  return ctx.getImageData(0, 0, img.width, img.height);
}

const found = document.querySelector('canvas');
if (!found) throw new Error('missing canvas');
const canvas: HTMLCanvasElement = found;
const [scene, sit, walk] = await Promise.all([pixels(sceneUrl), pixels(sitUrl), pixels(walkUrl)]);
const view = new Scene({ scene, sit, walk });
const glass = new Glass(canvas);

/** Same track geometry as Switch.track() in main.ts; the ball at rest (no stretch, no lean). */
function reference(s: State): string {
  canvas.width = s.width;
  canvas.height = s.height;
  const r = s.height * 0.5 * 0.88;
  const margin = s.height * 0.5 - r;
  const x = margin + r + (s.width - 2 * (margin + r)) * s.u;
  const cat = new Cat();
  cat.x = s.night ? DOOR.x : TREE.x;
  const image = view.render(s.t, s.time, cat);
  glass.draw(image, { x, y: s.height / 2, rx: r * s.press, ry: r * s.press }, daylight(s.t).night);
  return canvas.toDataURL('image/png');
}

Object.assign(window, { reference });
