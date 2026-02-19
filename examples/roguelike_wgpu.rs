//! Graphical roguelike demo using wgpu (GPU-accelerated).
//!
//! Run: cargo run --bin roguelike-wgpu

use gruid_core::app::{AppRunner, EventLoopDriver};
use gruid_examples::{Game, HEIGHT, WIDTH};
use gruid_wgpu::{WgpuConfig, WgpuDriver};

fn main() {
    let game = Game::new();
    let driver = WgpuDriver::new(WgpuConfig {
        title: "gruid-rs roguelike (wgpu)".into(),
        font_size: 18.0,
        grid_width: WIDTH,
        grid_height: HEIGHT,
        ..Default::default()
    });

    let runner = AppRunner::new(Box::new(game), WIDTH, HEIGHT);

    if let Err(e) = driver.run(runner) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
