//! Shamogu graphical (wgpu GPU-accelerated) entry point.

use gruid_core::app::{AppRunner, EventLoopDriver};
use gruid_wgpu::{WgpuConfig, WgpuDriver};
use shamogu_lib::tiles::ShamoguTileManager;
use shamogu_lib::{ShamoguModel, UI_HEIGHT, UI_WIDTH};

fn main() {
    let model = ShamoguModel::new();
    let driver = WgpuDriver::new(WgpuConfig {
        title: "Shamogu (wgpu)".into(),
        font_size: 18.0,
        grid_width: UI_WIDTH,
        grid_height: UI_HEIGHT,
        tile_manager: Some(Box::new(ShamoguTileManager::new())),
        ..Default::default()
    });

    let runner = AppRunner::new(Box::new(model), UI_WIDTH, UI_HEIGHT);

    if let Err(e) = driver.run(runner) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
