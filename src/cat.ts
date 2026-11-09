import { lerp } from './color.ts';
import { DOOR, TREE } from './layout.ts';

const SPEED = 30;

/** 白天坐在树下；入夜走回门口，坐在窗光里；天亮再走回树下。 */
export class Cat {
  x: number = TREE.x;
  facing: 1 | -1 = -1;
  walking = false;
  private stride = 0;

  update(dt: number, night: boolean): void {
    const dx = (night ? DOOR.x : TREE.x) - this.x;
    this.walking = Math.abs(dx) > 0.5;
    if (!this.walking) {
      this.x += dx;
      return;
    }
    this.facing = dx > 0 ? 1 : -1;
    this.x += this.facing * Math.min(Math.abs(dx), SPEED * dt);
    this.stride += dt;
  }

  get frame(): number {
    return Math.floor(this.stride * 9) % 4;
  }

  /** 脚底所在行：门口到树下之间线性插值。 */
  get baseline(): number {
    return Math.round(lerp(DOOR.y, TREE.y, (this.x - DOOR.x) / (TREE.x - DOOR.x)));
  }
}
