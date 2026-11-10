//! The component end to end on Slint's software renderer, headless: pointer input reaches the
//! driver, `night`/`toggled` follow, and the frame timer stops once everything has settled.
//! One test function: the Slint platform is per process and bound to one thread.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
use unicat_switch_slint::{UnicatSwitchWindow, attach};

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

const SIZE: PhysicalSize = PhysicalSize {
    width: 652,
    height: 400,
};

fn draw(
    window: &MinimalSoftwareWindow,
) -> Vec<PremultipliedRgbaColor> {
    let mut buffer = vec![
        PremultipliedRgbaColor::default();
        (SIZE.width * SIZE.height)
            as usize
    ];
    window.draw_if_needed(|renderer| {
        renderer.render(&mut buffer, SIZE.width as usize);
    });
    buffer
}

fn press(
    window: &MinimalSoftwareWindow,
    at: LogicalPosition,
) {
    window.dispatch_event(WindowEvent::PointerMoved {
        position: at,
    });
    window.dispatch_event(WindowEvent::PointerPressed {
        position: at,
        button: PointerEventButton::Left,
    });
}

fn release(
    window: &MinimalSoftwareWindow,
    at: LogicalPosition,
) {
    window.dispatch_event(WindowEvent::PointerReleased {
        position: at,
        button: PointerEventButton::Left,
    });
}

#[test]
fn component_round_trip() {
    slint::platform::set_platform(Box::new(Headless))
        .unwrap();
    let ui = UnicatSwitchWindow::new().unwrap();
    let window =
        LAST.with(|last| last.borrow_mut().take()).unwrap();
    let driver = attach(&ui);
    let toggles = Rc::new(Cell::new(0));
    let t = toggles.clone();
    ui.on_toggled(move |_| t.set(t.get() + 1));
    window.dispatch_event(
        WindowEvent::ScaleFactorChanged {
            scale_factor: 2.0,
        },
    );
    window.set_size(SIZE);
    ui.show().unwrap();

    // First frame: placed at day, nothing to animate, so no timer.
    let before = draw(&window);
    assert!(
        !driver.is_animating(),
        "idle switch must not run a timer"
    );
    assert!(
        before
            .iter()
            .any(|p| p.alpha == 255 && p.blue > p.red),
        "a blue day sky is on screen"
    );

    // A click (press and release without moving) flips night and wakes the frame loop.
    let ball = LogicalPosition::new(90.0, 100.0);
    press(&window, ball);
    release(&window, ball);
    assert!(ui.get_night());
    assert_eq!(toggles.get(), 1);
    draw(&window);
    assert!(driver.is_animating());

    // Run the frames the timer would run; it stops by itself once settled.
    let mut frames = 0;
    while driver.is_animating() {
        driver.advance(1.0 / 60.0);
        draw(&window);
        frames += 1;
        assert!(frames < 60 * 10, "never settled");
    }
    let night = draw(&window);
    assert!(
        frames > 60,
        "the sky takes seconds to turn, settled after {frames} frames"
    );
    let changed = before
        .iter()
        .zip(&night)
        .filter(|(a, b)| {
            (a.red, a.green, a.blue)
                != (b.red, b.green, b.blue)
        })
        .count();
    assert!(
        changed > before.len() / 4,
        "night repaints most of the switch ({changed} pixels)"
    );

    // Drag the ball from the right end most of the way left, slowly: lands on day.
    let right = LogicalPosition::new(236.0, 100.0);
    press(&window, right);
    for i in 1..=20 {
        window.dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(
                236.0 - 6.0 * i as f32,
                100.0,
            ),
        });
        std::thread::sleep(
            std::time::Duration::from_millis(5),
        );
    }
    release(&window, LogicalPosition::new(116.0, 100.0));
    assert!(!ui.get_night());
    assert_eq!(
        toggles.get(),
        2,
        "a drag that changes sides reports one toggle, the click after it none"
    );

    // A drag that pauses before letting go stays on the nearer side: no toggle. (Each move
    // halves the old velocity estimate, so twenty slow moves bleed off the first jump.)
    press(&window, LogicalPosition::new(90.0, 100.0));
    for i in 0..=20 {
        window.dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(
                110.0 + 0.1 * i as f32,
                100.0,
            ),
        });
        std::thread::sleep(
            std::time::Duration::from_millis(20),
        );
    }
    release(&window, LogicalPosition::new(112.0, 100.0));
    assert!(!ui.get_night());
    assert_eq!(toggles.get(), 2);

    // Setting `night` from outside animates too.
    ui.set_night(true);
    draw(&window);
    assert!(driver.is_animating());
}
