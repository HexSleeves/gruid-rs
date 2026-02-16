//! Shamogu graphical (winit + softbuffer) entry point.

use gruid_core::app::{AppRunner, EventLoopDriver};
use gruid_winit::{WinitConfig, WinitDriver};
use shamogu_lib::{ShamoguModel, UI_HEIGHT, UI_WIDTH};

fn main() {
    let model = ShamoguModel::new();
    let driver = WinitDriver::new(WinitConfig {
        title: "Shamogu".into(),
        font_size: 18.0,
        grid_width: UI_WIDTH,
        grid_height: UI_HEIGHT,
        ..Default::default()
    });

    let runner = AppRunner::new(Box::new(model), UI_WIDTH, UI_HEIGHT);

    if let Err(e) = driver.run(runner) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
