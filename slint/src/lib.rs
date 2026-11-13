//! The unicat day/night switch for Slint, rendered entirely on the CPU into a pixel buffer so it
//! looks the same on the software, FemtoVG and Skia renderers.
//!
//! Two layers:
//! - [`Driver`] owns the state (springs, drag, cat, sky) and renders frames. It knows nothing of
//!   the generated Slint types, so any `.slint` that embeds the component can use it.
//! - `ui/unicat-switch.slint` holds `UnicatSwitch`, the component; [`UnicatSwitchWindow`] wraps it
//!   in a window for Rust callers without their own `.slint`, and [`attach`] wires the two.
//!
//! Embedding `UnicatSwitch` in an app's own `.slint`: add the `ui` directory to `slint-build`'s
//! library paths under a name (e.g. `unicat`) — a build script that depends on this crate reads it
//! from `DEP_UNICAT_SWITCH_LIBRARY_PATH`, and [`library_path`] returns it at runtime — `import { UnicatSwitch } from "@unicat/unicat-switch.slint"`,
//! forward its `tick` property and `render`/`input` callbacks to the app's root or a global,
//! and bind them to a [`Driver`] the same way [`attach`] does.

mod assets;
mod bezel;
mod cat;
mod color;
mod daylight;
mod dither;
mod driver;
mod glass;
mod layout;
mod raster;
mod scene;
mod sky;
mod spring;
mod switch;

pub use driver::{Commit, Driver, Frame, Input, View};

slint::include_modules!();

/// The directory holding `unicat-switch.slint`, for `slint_build::CompilerConfiguration::with_library_paths`.
pub fn library_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("ui")
}

/// Binds a [`UnicatSwitchWindow`] to a new [`Driver`]. Keep the returned driver alive as long as
/// the window (the window's callbacks only hold a weak reference).
pub fn attach(window: &UnicatSwitchWindow) -> Driver {
    let weak = window.as_weak();
    let driver = Driver::new(move |tick| {
        if let Some(window) = weak.upgrade() {
            window.set_tick(tick);
        }
    });
    let d = driver.clone();
    window.on_render(
        move |width, height, pad, scale, night, _tick| {
            let px = |v: i32| v.max(0) as u32;
            let view = View {
                width: px(width),
                height: px(height),
                pad: px(pad),
                scale,
                night,
            };
            let Frame { image, phase, glow } =
                d.render(view);
            SwitchFrame { image, phase, glow }
        },
    );
    let d = driver.clone();
    window.on_input(move |kind, x| {
        let input = match kind {
            SwitchInput::Down => Input::Down,
            SwitchInput::Move => Input::Move,
            SwitchInput::Up => Input::Up,
            SwitchInput::Cancel => Input::Cancel,
            SwitchInput::Leave => Input::Leave,
        };
        match d.input(input, x) {
            Commit::None => SwitchCommit::None,
            Commit::Toggle => SwitchCommit::Toggle,
            Commit::Set(false) => SwitchCommit::Day,
            Commit::Set(true) => SwitchCommit::Night,
        }
    });
    driver
}
