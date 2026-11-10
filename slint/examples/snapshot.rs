//! Headless frames through Slint's software renderer, the web comparison, and frame cost.
//!
//! `cargo run --release --example snapshot -- <outdir>` writes `slint-{day,dusk,night}.png` (the
//! whole window) and `slint-*-canvas.png` (the canvas, 516×221 at 2× scale). If `web-*.png` from
//! `bun slint/compare/web.ts <outdir>` are there too, it writes `compare-*.png` (web | slint |
//! |diff|×4) and prints the per-state difference. Last it times frames at 2× scale.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor,
    RepaintBufferType,
};
use slint::platform::{
    Platform, PointerEventButton, WindowAdapter,
    WindowEvent,
};
use slint::{
    ComponentHandle, LogicalPosition, PhysicalSize,
    PlatformError,
};
use unicat_switch_slint::{
    Driver, Frame, SwitchFrame, UnicatSwitchWindow, View,
    attach,
};

const SCALE: f32 = 2.0;
/// Frames of 1/20 s before each capture, matching web.ts.
const SETTLE_FRAMES: usize = 160;

thread_local! {
    static LAST: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

struct Headless;

impl Platform for Headless {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let window = MinimalSoftwareWindow::new(
            RepaintBufferType::NewBuffer,
        );
        LAST.with(|last| {
            *last.borrow_mut() = Some(window.clone())
        });
        Ok(window)
    }
}

struct Rgba {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

/// One switch in a headless window, with the driver's last frame kept for checking.
struct Rig {
    ui: UnicatSwitchWindow,
    window: Rc<MinimalSoftwareWindow>,
    driver: Driver,
    last: Rc<RefCell<Option<Frame>>>,
    render_time: Rc<RefCell<Duration>>,
    pad: Rc<Cell<u32>>,
    size: PhysicalSize,
}

impl Rig {
    /// `switch_width` logical px; the window adds the 28px margin of window.slint.
    fn new(switch_width: f32) -> Rig {
        let ui = UnicatSwitchWindow::new().unwrap();
        let window = LAST
            .with(|last| last.borrow_mut().take())
            .unwrap();
        let driver = attach(&ui);
        let last = Rc::new(RefCell::new(None));
        let render_time =
            Rc::new(RefCell::new(Duration::ZERO));
        // Same as attach()'s handler, but keeps the frame and times the CPU render.
        let pad = Rc::new(Cell::new(0));
        let (d, l, t, p) = (
            driver.clone(),
            last.clone(),
            render_time.clone(),
            pad.clone(),
        );
        ui.on_render(
            move |width, height, pad, scale, night, _| {
                let start = Instant::now();
                let frame = d.render(View {
                    width: width as u32,
                    height: height as u32,
                    pad: pad as u32,
                    scale,
                    night,
                });
                *t.borrow_mut() += start.elapsed();
                p.set(pad as u32);
                *l.borrow_mut() = Some(frame.clone());
                SwitchFrame {
                    image: frame.image,
                    phase: frame.phase,
                    glow: frame.glow,
                }
            },
        );
        window.dispatch_event(
            WindowEvent::ScaleFactorChanged {
                scale_factor: SCALE,
            },
        );
        let height =
            switch_width * 143.0 / 315.0 + 56.0 + 20.0;
        let size = PhysicalSize::new(
            ((switch_width + 56.0) * SCALE).round() as u32,
            (height * SCALE).round() as u32,
        );
        window.set_size(size);
        ui.show().unwrap();
        let rig = Rig {
            ui,
            window,
            driver,
            last,
            render_time,
            pad,
            size,
        };
        rig.draw();
        rig
    }

    fn draw(&self) -> Rgba {
        let (w, h) = (
            self.size.width as usize,
            self.size.height as usize,
        );
        let mut buffer =
            vec![PremultipliedRgbaColor::default(); w * h];
        self.window.request_redraw();
        self.window.draw_if_needed(|renderer| {
            renderer.render(&mut buffer, w);
        });
        let data = buffer
            .iter()
            .flat_map(|p| [p.red, p.green, p.blue, p.alpha])
            .collect();
        Rgba {
            width: w,
            height: h,
            data,
        }
    }

    fn settle(&self) {
        for _ in 0..SETTLE_FRAMES {
            self.driver.advance(0.05);
        }
    }

    fn pointer(&self, event: WindowEvent) {
        self.window.dispatch_event(event);
    }

    /// Canvas size in physical px: the frame minus the bezel padding.
    fn canvas_size(&self) -> (usize, usize) {
        let (frame, pad) =
            (self.frame(), self.pad.get() as usize);
        (frame.width - 2 * pad, frame.height - 2 * pad)
    }

    fn frame(&self) -> Rgba {
        let image = self
            .last
            .borrow()
            .as_ref()
            .unwrap()
            .image
            .clone();
        let buffer = image.to_rgba8().unwrap();
        Rgba {
            width: buffer.width() as usize,
            height: buffer.height() as usize,
            data: buffer.as_bytes().to_vec(),
        }
    }
}

/// Where the canvas sits in the window, in physical px (same snapping as unicat-switch.slint).
fn canvas_origin(
    rig: &Rig,
    switch_width: f32,
) -> (usize, usize) {
    let phx = |v: f32| (v * SCALE).round() / SCALE;
    let switch_h = rig.size.height as f32 / SCALE - 56.0;
    let pill_w = switch_width.min(switch_h * 315.0 / 143.0);
    let pad = phx(pill_w / 45.0);
    let canvas_w = phx(pill_w - 2.0 * pad);
    let canvas_h = phx(canvas_w * 96.0 / 224.0);
    let bezel_x =
        phx((switch_width - (canvas_w + 2.0 * pad)) / 2.0);
    let bezel_y =
        phx((switch_h - (canvas_h + 2.0 * pad)) / 2.0);
    (
        ((28.0 + bezel_x + pad) * SCALE) as usize,
        ((28.0 + bezel_y + pad) * SCALE) as usize,
    )
}

fn crop(
    src: &Rgba,
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
) -> Rgba {
    let mut data = Vec::with_capacity(width * height * 4);
    for y in y0..y0 + height {
        data.extend_from_slice(
            &src.data[(y * src.width + x0) * 4
                ..(y * src.width + x0 + width) * 4],
        );
    }
    Rgba {
        width,
        height,
        data,
    }
}

fn save(path: &Path, img: &Rgba) {
    let file = std::io::BufWriter::new(
        std::fs::File::create(path).unwrap(),
    );
    let mut encoder = png::Encoder::new(
        file,
        img.width as u32,
        img.height as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&img.data)
        .unwrap();
    println!("{}", path.display());
}

fn load(path: &Path) -> Option<Rgba> {
    let decoder =
        png::Decoder::new(std::io::BufReader::new(
            std::fs::File::open(path).ok()?,
        ));
    let mut reader = decoder.read_info().ok()?;
    let mut data = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut data).ok()?;
    assert_eq!(info.color_type, png::ColorType::Rgba);
    data.truncate(
        info.width as usize * info.height as usize * 4,
    );
    Some(Rgba {
        width: info.width as usize,
        height: info.height as usize,
        data,
    })
}

/// Mean |ΔRGB| (0–255) over pixels opaque in both, the share of identical pixels, and the max.
fn diff(web: &Rgba, ours: &Rgba) -> (f64, f64, u8, Rgba) {
    let (mut sum, mut n, mut same, mut max) =
        (0u64, 0u64, 0u64, 0u8);
    let mut heat = Rgba {
        width: web.width,
        height: web.height,
        data: vec![0; web.data.len()],
    };
    for i in (0..web.data.len()).step_by(4) {
        heat.data[i + 3] = 255;
        if web.data[i + 3] != 255 || ours.data[i + 3] != 255
        {
            continue;
        }
        let d: Vec<u8> = (0..3)
            .map(|k| {
                web.data[i + k].abs_diff(ours.data[i + k])
            })
            .collect();
        sum += d.iter().map(|&v| u64::from(v)).sum::<u64>();
        n += 1;
        same += u64::from(d.iter().all(|&v| v == 0));
        max = max.max(*d.iter().max().unwrap());
        for k in 0..3 {
            heat.data[i + k] = d[k].saturating_mul(4);
        }
    }
    (
        sum as f64 / (n * 3) as f64,
        same as f64 / n as f64,
        max,
        heat,
    )
}

/// Images side by side with an 8px gap, transparent parts over a mid gray.
fn side_by_side(images: &[&Rgba]) -> Rgba {
    let gap = 8;
    let height =
        images.iter().map(|i| i.height).max().unwrap();
    let width =
        images.iter().map(|i| i.width).sum::<usize>()
            + gap * (images.len() - 1);
    let mut out = Rgba {
        width,
        height,
        data: [96, 96, 96, 255].repeat(width * height),
    };
    let mut x0 = 0;
    for img in images {
        for y in 0..img.height {
            for x in 0..img.width {
                let s = (y * img.width + x) * 4;
                let d = (y * width + x0 + x) * 4;
                let a = u16::from(img.data[s + 3]);
                for k in 0..3 {
                    let v = (u16::from(img.data[s + k])
                        * a
                        + 96 * (255 - a))
                        / 255;
                    out.data[d + k] = v as u8;
                }
            }
        }
        x0 += img.width + gap;
    }
    out
}

fn capture(
    out: &Path,
    name: &str,
    rig: &Rig,
    switch_width: f32,
) {
    let shot = rig.draw();
    save(&out.join(format!("slint-{name}.png")), &shot);
    let frame = rig.frame();
    let (x0, y0) = canvas_origin(rig, switch_width);
    let canvas = rig.canvas_size();
    let pad = (frame.width - canvas.0) / 2;
    // The software renderer must put the driver's buffer (bezel and canvas) on screen 1:1.
    let bezel = crop(
        &shot,
        x0 - pad,
        y0 - pad,
        frame.width,
        frame.height,
    );
    let off = (0..frame.data.len())
        .step_by(4)
        .filter(|&i| {
            frame.data[i + 3] == 255
                && frame.data[i..i + 3]
                    != bezel.data[i..i + 3]
        })
        .count();
    println!(
        "{name}: canvas {}x{} at ({x0},{y0}); {off} opaque pixels differ from the driver buffer",
        canvas.0, canvas.1
    );
    let canvas = crop(&shot, x0, y0, canvas.0, canvas.1);
    save(
        &out.join(format!("slint-{name}-canvas.png")),
        &canvas,
    );
    if let Some(web) =
        load(&out.join(format!("web-{name}.png")))
    {
        assert_eq!(
            (web.width, web.height),
            (canvas.width, canvas.height),
            "web frame size"
        );
        let (mean, same, max, heat) = diff(&web, &canvas);
        println!(
            "{name}: mean |ΔRGB| {mean:.3}/255, identical pixels {:.2}%, max {max}",
            same * 100.0
        );
        save(
            &out.join(format!("compare-{name}.png")),
            &side_by_side(&[&web, &canvas, &heat]),
        );
    }
}

/// A click into night and back, timing every frame through the software renderer.
fn bench(switch_width: f32) {
    let rig = Rig::new(switch_width);
    let (mut frames, mut total) = (0u32, Duration::ZERO);
    *rig.render_time.borrow_mut() = Duration::ZERO;
    for night in [true, false] {
        rig.ui.set_night(night);
        rig.draw();
        for _ in 0..300 {
            if !rig.driver.is_animating() {
                break;
            }
            let start = Instant::now();
            rig.driver.advance(1.0 / 60.0);
            rig.draw();
            total += start.elapsed();
            frames += 1;
        }
    }
    let canvas = rig.canvas_size();
    let cpu = rig.render_time.borrow().as_secs_f64() * 1e3
        / f64::from(frames);
    let all = total.as_secs_f64() * 1e3 / f64::from(frames);
    println!(
        "bench: switch {switch_width}px at {SCALE}x, canvas {}x{}: {frames} frames, driver render {cpu:.2} ms/frame, \
         with software-renderer composite {all:.2} ms/frame; timer stopped after settling: {}",
        canvas.0,
        canvas.1,
        !rig.driver.is_animating()
    );
}

fn main() {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("usage: snapshot <outdir>"),
    );
    std::fs::create_dir_all(&out).unwrap();
    slint::platform::set_platform(Box::new(Headless))
        .unwrap();
    let width = 270.0;

    let day = Rig::new(width);
    day.settle();
    capture(&out, "day", &day, width);

    // Grab the ball, drag it to the middle of the track, hold: the sky follows to t = 0.5.
    let dusk = Rig::new(width);
    let (x0, y0) = canvas_origin(&dusk, width);
    let (cx, cy) = (x0 as f32 / SCALE, y0 as f32 / SCALE);
    let (canvas_w, canvas_h) = dusk.canvas_size();
    let track = canvas_w as f32 - canvas_h as f32;
    let ball = LogicalPosition::new(
        cx + canvas_h as f32 / 2.0 / SCALE,
        cy + canvas_h as f32 / 2.0 / SCALE,
    );
    let held = LogicalPosition::new(
        ball.x + track / 2.0 / SCALE,
        ball.y,
    );
    dusk.pointer(WindowEvent::PointerMoved {
        position: ball,
    });
    dusk.pointer(WindowEvent::PointerPressed {
        position: ball,
        button: PointerEventButton::Left,
    });
    dusk.pointer(WindowEvent::PointerMoved {
        position: held,
    });
    dusk.settle();
    capture(&out, "dusk", &dusk, width);

    let night = Rig::new(width);
    night.ui.set_night(true);
    night.draw();
    night.settle();
    capture(&out, "night", &night, width);

    bench(width);
    bench(540.0);
}
