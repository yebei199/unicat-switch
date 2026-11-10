//! Port of the `Switch` class in `src/main.ts`: springs, drag and release rules, and the ball's
//! squash, lean and press. Positions are canvas device pixels, like the TS `devicePixels`.

use crate::cat::Cat;
use crate::color::{clamp01, lerp};
use crate::daylight::daylight;
use crate::glass::Ball;
use crate::spring::Spring;

const BALL_FILL: f64 = 0.88;
const DRAG_SLOP: f64 = 4.0;
const FLICK: f64 = 1.5;

/// What a pointer release asks the component to do with its `night` property.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Commit {
    None,
    /// A click: flip `night`.
    Toggle,
    /// A drag ended: set `night` to this.
    Set(bool),
}

struct Track {
    r: f64,
    x0: f64,
    x1: f64,
    h: f64,
}

struct Drag {
    start_x: f64,
    start_target: f64,
    moved: bool,
    last_x: f64,
    last_time: f64,
    velocity: f64,
}

/// Past either end the drag keeps going at a quarter rate and springs back on release.
fn rubber(v: f64) -> f64 {
    if v < 0.0 {
        v * 0.25
    } else if v > 1.0 {
        1.0 + (v - 1.0) * 0.25
    } else {
        v
    }
}

/// Underdamped ball (it overshoots) and a slower critically damped sky so dusk lingers; with
/// reduced motion both are critically damped.
fn springs(calm: bool) -> (Spring, Spring) {
    if calm {
        (
            Spring::new(0.0, 400.0, 40.0),
            Spring::new(0.0, 40.0, 12.6),
        )
    } else {
        (
            Spring::new(0.0, 190.0, 15.0),
            Spring::new(0.0, 12.0, 6.9),
        )
    }
}

pub struct Switch {
    checked: bool,
    target: f64,
    pointer: Option<f64>,
    pressed: bool,
    drag: Option<Drag>,
    pub cat: Cat,
    pos: Spring,
    sky: Spring,
    lean: Spring,
    press: Spring,
    /// Seconds of animation so far: clouds drift and stars twinkle on this clock.
    pub time: f64,
    /// Canvas size in device pixels and device pixels per logical pixel.
    width: f64,
    height: f64,
    dpr: f64,
}

impl Switch {
    pub fn new(calm: bool) -> Self {
        let (pos, sky) = springs(calm);
        Self {
            checked: false,
            target: 0.0,
            pointer: None,
            pressed: false,
            drag: None,
            cat: Cat::default(),
            pos,
            sky,
            lean: Spring::new(0.0, 220.0, 22.0),
            press: Spring::new(1.0, 420.0, 26.0),
            time: 0.0,
            width: 0.0,
            height: 0.0,
            dpr: 1.0,
        }
    }

    pub fn set_reduced_motion(&mut self, calm: bool) {
        let (mut pos, mut sky) = springs(calm);
        pos.reset(self.pos.value);
        sky.reset(self.sky.value);
        (pos.velocity, sky.velocity) =
            (self.pos.velocity, self.sky.velocity);
        (self.pos, self.sky) = (pos, sky);
    }

    /// Jumps straight to `checked` at rest: the initial state, nothing animates.
    pub fn place(&mut self, checked: bool) {
        self.commit(checked);
        self.pos.reset(self.target);
        self.sky.reset(self.target);
        self.cat.place(checked);
    }

    /// Follows an outside change of `night` (keyboard, accessibility, the app). True if it moved.
    pub fn sync(&mut self, checked: bool) -> bool {
        if checked == self.checked {
            return false;
        }
        self.commit(checked);
        true
    }

    pub fn resize(
        &mut self,
        width: f64,
        height: f64,
        dpr: f64,
    ) {
        (self.width, self.height, self.dpr) =
            (width, height, dpr);
    }

    pub fn dpr(&self) -> f64 {
        self.dpr
    }

    pub fn down(&mut self, x: f64, now: f64) {
        self.pressed = true;
        self.drag = Some(Drag {
            start_x: x,
            start_target: self.target,
            moved: false,
            last_x: x,
            last_time: now,
            velocity: 0.0,
        });
    }

    pub fn move_to(&mut self, x: f64, now: f64) {
        self.pointer = Some(x);
        let span = self.track().x1 - self.track().x0;
        let Some(drag) = &mut self.drag else { return };
        let dx = x - drag.start_x;
        if !drag.moved && dx.abs() < DRAG_SLOP * self.dpr {
            return;
        }
        drag.moved = true;
        self.target = rubber(drag.start_target + dx / span);
        let dt = now - drag.last_time;
        if dt > 0.0 {
            drag.velocity = lerp(
                drag.velocity,
                (x - drag.last_x) / span / dt,
                0.5,
            );
        }
        drag.last_x = x;
        drag.last_time = now;
    }

    /// Release: a fast flick goes by its direction, otherwise to the nearer side; a press that
    /// never moved is a click. A release without a press (it began elsewhere) does nothing.
    pub fn up(&mut self) -> Commit {
        match self.release() {
            Some(None) => Commit::Toggle,
            Some(Some(checked)) => Commit::Set(checked),
            None => Commit::None,
        }
    }

    /// The pointer was taken away (TS `pointercancel`): a drag still lands, a press is no click.
    pub fn cancel(&mut self) -> Commit {
        self.release()
            .flatten()
            .map_or(Commit::None, Commit::Set)
    }

    pub fn leave(&mut self) {
        self.pointer = None;
    }

    /// Ends a press: `None` if there was none, `Some(None)` if it never became a drag, else the
    /// side the drag lands on.
    fn release(&mut self) -> Option<Option<bool>> {
        self.pressed = false;
        let drag = self.drag.take()?;
        if !drag.moved {
            return Some(None);
        }
        let checked = if drag.velocity.abs() > FLICK {
            drag.velocity > 0.0
        } else {
            self.target > 0.5
        };
        self.commit(checked);
        Some(Some(checked))
    }

    fn commit(&mut self, checked: bool) {
        self.checked = checked;
        self.target = if checked { 1.0 } else { 0.0 };
    }

    fn track(&self) -> Track {
        let h = self.height;
        let r = h * 0.5 * BALL_FILL;
        let margin = h * 0.5 - r;
        Track {
            r,
            x0: margin + r,
            x1: self.width - margin - r,
            h,
        }
    }

    /// One frame of motion (`frame()` in main.ts, minus drawing).
    pub fn advance(&mut self, dt: f64) {
        self.time += dt;
        self.sky.step(clamp01(self.target), dt);
        self.cat.update(dt, self.target > 0.5);
        let Track { r, x0, x1, .. } = self.track();
        let x =
            lerp(x0, x1, self.pos.step(self.target, dt));
        let toward = match (self.pointer, &self.drag) {
            (Some(p), drag)
                if !drag
                    .as_ref()
                    .is_some_and(|d| d.moved) =>
            {
                (p - x) * 0.15
            }
            _ => 0.0,
        };
        self.lean
            .step(toward.clamp(-0.14 * r, 0.14 * r), dt);
        self.press.step(
            if self.pressed { 0.93 } else { 1.0 },
            dt,
        );
    }

    /// Everything at rest (snapped exactly onto its target) and the cat sitting.
    pub fn settle(&mut self) -> bool {
        let springs = [
            self.pos.settle(1e-4),
            self.sky.settle(1e-4),
            self.lean.settle(1e-2),
            self.press.settle(1e-4),
        ];
        springs.iter().all(|&s| s) && !self.cat.walking
    }

    /// Sky phase `t`: 0 day, 0.5 dusk, 1 night.
    pub fn phase(&self) -> f64 {
        clamp01(self.sky.value)
    }

    pub fn night(&self) -> f64 {
        daylight(self.phase()).night
    }

    pub fn ball(&self) -> Ball {
        let Track { r, x0, x1, h } = self.track();
        let x = lerp(x0, x1, self.pos.value);
        let stretch = 1.0
            + ((self.pos.velocity * (x1 - x0)).abs()
                / (r * 28.0))
                .min(0.22);
        let s = self.press.value;
        Ball {
            x: (x + self.lean.value) as f32,
            y: (h / 2.0) as f32,
            rx: (r * stretch * s) as f32,
            ry: (r / stretch * s) as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sized() -> Switch {
        let mut s = Switch::new(false);
        s.resize(516.0, 221.0, 1.0);
        s
    }

    fn run(s: &mut Switch, seconds: f64) {
        for _ in 0..(seconds * 60.0) as usize {
            s.advance(1.0 / 60.0);
        }
    }

    #[test]
    fn click_toggles_and_everything_settles() {
        let mut s = sized();
        s.down(100.0, 0.0);
        assert_eq!(s.up(), Commit::Toggle);
        assert!(s.sync(true));
        run(&mut s, 6.0);
        assert!(s.settle());
        assert_eq!(s.phase(), 1.0);
        assert_eq!(s.cat.x, crate::layout::DOOR.x);
    }

    #[test]
    fn slow_drag_goes_to_the_nearer_side() {
        let mut s = sized();
        s.down(110.0, 0.0);
        // 295 px track: 0.6 of it in many small slow steps.
        for i in 1..=60 {
            s.move_to(
                110.0 + 177.0 * f64::from(i) / 60.0,
                f64::from(i),
            );
        }
        assert!((s.target - 0.6).abs() < 1e-9);
        assert_eq!(s.up(), Commit::Set(true));
    }

    #[test]
    fn fast_flick_goes_by_direction() {
        let mut s = sized();
        s.place(true);
        s.down(400.0, 0.0);
        // A short, fast flick left that ends still nearer the right side.
        s.move_to(380.0, 0.01);
        s.move_to(360.0, 0.02);
        assert!(s.target > 0.5);
        assert_eq!(s.up(), Commit::Set(false));
    }

    #[test]
    fn press_without_move_is_not_a_drag_on_cancel() {
        let mut s = sized();
        s.down(100.0, 0.0);
        s.move_to(102.0, 0.1);
        assert_eq!(s.cancel(), Commit::None);
    }

    #[test]
    fn release_without_press_is_nothing() {
        let mut s = sized();
        assert_eq!(s.up(), Commit::None);
    }

    #[test]
    fn rubber_band_past_the_ends() {
        assert!((rubber(-0.4) + 0.1).abs() < 1e-12);
        assert!((rubber(1.4) - 1.1).abs() < 1e-12);
        assert_eq!(rubber(0.3), 0.3);
    }
}
