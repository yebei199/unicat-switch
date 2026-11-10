//! Port of `src/spring.ts`: a damped spring integrated in fixed small steps, so dropped frames
//! never blow it up.

const STEP: f64 = 1.0 / 240.0;

#[derive(Clone, Debug)]
pub struct Spring {
    pub value: f64,
    pub velocity: f64,
    stiffness: f64,
    damping: f64,
    /// The target of the last `step`; `settled` measures against it.
    goal: f64,
}

impl Spring {
    pub fn new(
        value: f64,
        stiffness: f64,
        damping: f64,
    ) -> Self {
        Self {
            value,
            velocity: 0.0,
            stiffness,
            damping,
            goal: value,
        }
    }

    pub fn step(&mut self, target: f64, dt: f64) -> f64 {
        self.goal = target;
        let n = (dt / STEP).ceil().max(1.0);
        let h = dt / n;
        for _ in 0..n as u32 {
            let accel = self.stiffness
                * (target - self.value)
                - self.damping * self.velocity;
            self.velocity += accel * h;
            self.value += self.velocity * h;
        }
        self.value
    }

    /// At rest on its goal within `eps` (value units, and value units per second): snaps exactly
    /// onto the goal so the final frame is the true end state, and reports true.
    pub fn settle(&mut self, eps: f64) -> bool {
        if (self.goal - self.value).abs() < eps
            && self.velocity.abs() < eps
        {
            self.value = self.goal;
            self.velocity = 0.0;
            return true;
        }
        false
    }

    /// Jumps to `value` at rest, without animating.
    pub fn reset(&mut self, value: f64) {
        self.value = value;
        self.velocity = 0.0;
        self.goal = value;
    }
}

#[cfg(test)]
mod tests {
    use super::Spring;

    fn settle_time(mut spring: Spring, target: f64) -> f64 {
        let dt = 1.0 / 60.0;
        let mut time = 0.0;
        while time < 30.0 {
            spring.step(target, dt);
            time += dt;
            if spring.settle(1e-4) {
                assert_eq!(spring.value, target);
                return time;
            }
        }
        panic!("spring never settled");
    }

    #[test]
    fn ball_spring_overshoots_then_settles() {
        let mut s = Spring::new(0.0, 190.0, 15.0);
        let peak = (0..120)
            .map(|_| s.step(1.0, 1.0 / 60.0))
            .fold(0.0_f64, f64::max);
        assert!(
            peak > 1.05,
            "underdamped spring should overshoot, peak {peak}"
        );
        let t =
            settle_time(Spring::new(0.0, 190.0, 15.0), 1.0);
        assert!(
            (0.8..2.5).contains(&t),
            "ball settles in {t}s"
        );
    }

    #[test]
    fn sky_spring_does_not_overshoot_and_settles() {
        let mut s = Spring::new(0.0, 12.0, 6.9);
        let peak = (0..600)
            .map(|_| s.step(1.0, 1.0 / 60.0))
            .fold(0.0_f64, f64::max);
        assert!(
            peak <= 1.0 + 1e-3,
            "near-critical sky spring overshot to {peak}"
        );
        let t =
            settle_time(Spring::new(0.0, 12.0, 6.9), 1.0);
        assert!(
            (2.0..6.0).contains(&t),
            "sky settles in {t}s"
        );
    }
}
