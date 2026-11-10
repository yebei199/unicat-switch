//! The frame loop (`main()` in `src/main.ts`), rebuilt for render-on-demand: a `slint::Timer`
//! runs only while something is still moving and stops itself once everything settles. Each
//! step bumps a tick that the component's `render` binding depends on, so Slint asks for a new
//! frame exactly when one exists — and only if the switch is actually drawn.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use slint::{
    Image, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode,
};

use crate::assets::ART;
use crate::scene::Scene;
pub use crate::switch::Commit;
use crate::switch::Switch;
use crate::{bezel, glass};

/// Longest step one frame may integrate, like the TS `Math.min(..., 0.05)`.
const MAX_DT: f64 = 0.05;
const FRAME: Duration = Duration::from_millis(16);

/// What the component reports when it asks for a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Canvas size in physical pixels.
    pub width: u32,
    pub height: u32,
    /// Bezel padding around the canvas, physical pixels.
    pub pad: u32,
    /// Physical pixels per logical pixel.
    pub scale: f32,
    /// The component's `night` property.
    pub night: bool,
}

/// One rendered frame: bezel and canvas, plus the sky phase and night level for the shadows.
#[derive(Clone, Debug)]
pub struct Frame {
    pub image: Image,
    /// Sky phase: 0 day, 0.5 dusk, 1 night.
    pub phase: f32,
    /// How far into night the light is (window glow under the bezel).
    pub glow: f32,
}

/// Pointer events from the component's `TouchArea`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Down,
    Move,
    Up,
    Cancel,
    Leave,
}

struct Inner {
    switch: RefCell<Switch>,
    scene: RefCell<Scene>,
    timer: Timer,
    epoch: Instant,
    last: Cell<Instant>,
    placed: Cell<bool>,
    tick: Cell<i32>,
    on_tick: Box<dyn Fn(i32)>,
}

/// Owns the switch state and its wake path. Cheap to clone (shared handle).
#[derive(Clone)]
pub struct Driver(Rc<Inner>);

impl Driver {
    /// `on_tick` must store the tick into the component's `tick` property.
    pub fn new(on_tick: impl Fn(i32) + 'static) -> Self {
        Self(Rc::new(Inner {
            switch: RefCell::new(Switch::new(false)),
            scene: RefCell::new(Scene::new(&ART)),
            timer: Timer::default(),
            epoch: Instant::now(),
            last: Cell::new(Instant::now()),
            placed: Cell::new(false),
            tick: Cell::new(0),
            on_tick: Box::new(on_tick),
        }))
    }

    /// Critically damped springs, no bounce (the TS `prefers-reduced-motion` path).
    pub fn set_reduced_motion(&self, calm: bool) {
        self.0.switch.borrow_mut().set_reduced_motion(calm);
    }

    /// The component's `render` callback. The first call places the switch at `night` without
    /// animating; later calls follow changes of `night` made outside the pointer path.
    pub fn render(&self, view: View) -> Frame {
        let moved = {
            let mut switch = self.0.switch.borrow_mut();
            switch.resize(
                f64::from(view.width),
                f64::from(view.height),
                f64::from(view.scale),
            );
            if self.0.placed.replace(true) {
                switch.sync(view.night)
            } else {
                switch.place(view.night);
                false
            }
        };
        if moved {
            self.wake();
        }
        self.draw(
            view.width as usize,
            view.height as usize,
            view.pad as usize,
        )
    }

    /// The component's `input` callback; `x` is logical pixels from the canvas' left edge.
    pub fn input(&self, input: Input, x: f32) -> Commit {
        let now = self.0.epoch.elapsed().as_secs_f64();
        let commit = {
            let mut switch = self.0.switch.borrow_mut();
            let x = f64::from(x) * switch.dpr();
            match input {
                Input::Down => {
                    switch.down(x, now);
                    Commit::None
                }
                Input::Move => {
                    switch.move_to(x, now);
                    Commit::None
                }
                Input::Up => switch.up(),
                Input::Cancel => switch.cancel(),
                Input::Leave => {
                    switch.leave();
                    Commit::None
                }
            }
        };
        self.wake();
        commit
    }

    /// Steps the motion by `dt` seconds and requests a frame; stops the timer once settled.
    /// The timer calls this; tests and offline capture call it directly with a fixed `dt`.
    pub fn advance(&self, dt: f64) {
        let settled = {
            let mut switch = self.0.switch.borrow_mut();
            switch.advance(dt);
            switch.settle()
        };
        let tick = self.0.tick.get().wrapping_add(1);
        self.0.tick.set(tick);
        (self.0.on_tick)(tick);
        if settled {
            self.0.timer.stop();
        }
    }

    /// True while the frame timer runs.
    pub fn is_animating(&self) -> bool {
        self.0.timer.running()
    }

    fn wake(&self) {
        if self.0.timer.running() {
            return;
        }
        self.0.last.set(Instant::now());
        let weak: Weak<Inner> = Rc::downgrade(&self.0);
        self.0.timer.start(
            TimerMode::Repeated,
            FRAME,
            move || {
                let Some(inner) = weak.upgrade() else {
                    return;
                };
                let now = Instant::now();
                let dt = now
                    .duration_since(inner.last.replace(now))
                    .as_secs_f64()
                    .min(MAX_DT);
                Driver(inner).advance(dt);
            },
        );
    }

    fn draw(
        &self,
        width: usize,
        height: usize,
        pad: usize,
    ) -> Frame {
        let switch = self.0.switch.borrow();
        let (phase, glow) =
            (switch.phase(), switch.night() as f32);
        if width == 0 || height == 0 {
            return Frame {
                image: Image::default(),
                phase: phase as f32,
                glow,
            };
        }
        let (outer_w, outer_h) =
            (width + 2 * pad, height + 2 * pad);
        let mut buffer =
            SharedPixelBuffer::<Rgba8Pixel>::new(
                outer_w as u32,
                outer_h as u32,
            );
        let bytes = buffer.make_mut_bytes();
        bezel::draw(outer_w, outer_h, pad, phase, bytes);
        let mut scene = self.0.scene.borrow_mut();
        let pixels =
            scene.render(phase, switch.time, &switch.cat);
        let stride = outer_w * 4;
        glass::draw(
            pixels,
            width,
            height,
            switch.ball(),
            glow,
            &mut bytes[pad * stride + pad * 4..],
            stride,
        );
        Frame {
            image: Image::from_rgba8(buffer),
            phase: phase as f32,
            glow,
        }
    }
}
