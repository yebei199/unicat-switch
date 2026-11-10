//! Port of `src/layout.ts`: the low-resolution canvas and the object coordinates measured on
//! `art/scene.png` (scene pixels).

pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub struct Ellipse {
    pub x: i32,
    pub y: i32,
    pub rx: i32,
    pub ry: i32,
}

pub const W: usize = 224;
pub const H: usize = 96;
pub const HORIZON: f64 = 66.0;
pub const WINDOW: Rect = Rect {
    x0: 25,
    y0: 55,
    x1: 41,
    y1: 72,
};
pub const DOOR: Point = Point { x: 75.0, y: 84.0 };
pub const TREE: Point = Point { x: 164.0, y: 86.0 };
pub const GLOW: Ellipse = Ellipse {
    x: 56,
    y: 85,
    rx: 34,
    ry: 9,
};
