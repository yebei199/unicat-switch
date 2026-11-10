//! A window with the switch: `cargo run --example demo`.

use slint::ComponentHandle;

fn main() -> Result<(), slint::PlatformError> {
    let window =
        unicat_switch_slint::UnicatSwitchWindow::new()?;
    let _driver = unicat_switch_slint::attach(&window);
    window.on_toggled(|night| println!("night: {night}"));
    window.run()
}
