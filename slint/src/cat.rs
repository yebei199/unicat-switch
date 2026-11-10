//! Port of `src/cat.ts`: by day the cat sits under the tree; at night it walks back to the door and
//! sits in the window light; at dawn it walks back to the tree.

use crate::color::{js_round, lerp};
use crate::layout::{DOOR, TREE};

const SPEED: f64 = 30.0;

#[derive(Clone, Debug)]
pub struct Cat {
    pub x: f64,
    pub facing: f64,
    pub walking: bool,
    stride: f64,
}

impl Default for Cat {
    fn default() -> Self {
        Self {
            x: TREE.x,
            facing: -1.0,
            walking: false,
            stride: 0.0,
        }
    }
}

impl Cat {
    pub fn update(&mut self, dt: f64, night: bool) {
        let dx =
            (if night { DOOR.x } else { TREE.x }) - self.x;
        self.walking = dx.abs() > 0.5;
        if !self.walking {
            self.x += dx;
            return;
        }
        self.facing = if dx > 0.0 { 1.0 } else { -1.0 };
        self.x += self.facing * dx.abs().min(SPEED * dt);
        self.stride += dt;
    }

    /// Sits wherever `night` says without walking there (initial state).
    pub fn place(&mut self, night: bool) {
        self.x = if night { DOOR.x } else { TREE.x };
        self.walking = false;
    }

    pub fn frame(&self) -> usize {
        (self.stride * 9.0).floor() as usize % 4
    }

    /// The row the feet stand on: linear between the doorstep and the tree.
    pub fn baseline(&self) -> i32 {
        js_round(lerp(
            DOOR.y,
            TREE.y,
            (self.x - DOOR.x) / (TREE.x - DOOR.x),
        )) as i32
    }
}
