const STEP = 1 / 240;

/** 阻尼弹簧；固定小步长积分，掉帧时不会炸。 */
export class Spring {
  velocity = 0;

  constructor(
    public value: number,
    private readonly stiffness: number,
    private readonly damping: number,
  ) {}

  step(target: number, dt: number): number {
    const n = Math.max(1, Math.ceil(dt / STEP));
    const h = dt / n;
    for (let i = 0; i < n; i++) {
      const accel = this.stiffness * (target - this.value) - this.damping * this.velocity;
      this.velocity += accel * h;
      this.value += this.velocity * h;
    }
    return this.value;
  }
}
