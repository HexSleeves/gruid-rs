//! Shamogu graphical (wgpu GPU-accelerated) entry point.

use gruid_core::app::{AppRunner, EventLoopDriver};
use gruid_wgpu::{WgpuConfig, WgpuDriver};
use shamogu_lib::tiles::ShamoguTileManager;
use shamogu_lib::{ShamoguModel, UI_HEIGHT, UI_WIDTH};

fn main() {
    let model = ShamoguModel::new();
    let tm: Box<dyn gruid_wgpu::TileManager> = Box::new(WgpuTileAdapter(ShamoguTileManager::new()));
    let driver = WgpuDriver::new(WgpuConfig {
        title: "Shamogu (wgpu)".into(),
        font_size: 18.0,
        grid_width: UI_WIDTH,
        grid_height: UI_HEIGHT,
        tile_manager: Some(tm),
        ..Default::default()
    });

    let runner = AppRunner::new(Box::new(model), UI_WIDTH, UI_HEIGHT);

    if let Err(e) = driver.run(runner) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Adapter: ShamoguTileManager already implements the same methods as
/// gruid_wgpu::TileManager, but the trait is a different type. We
/// delegate directly.
struct WgpuTileAdapter(ShamoguTileManager);

impl gruid_wgpu::TileManager for WgpuTileAdapter {
    fn tile_size(&self) -> (usize, usize) {
        self.0.tile_size_raw()
    }
    fn get_tile(&self, cell: &gruid_core::Cell) -> Option<&[u8]> {
        self.0.get_tile_raw(cell)
    }
}
