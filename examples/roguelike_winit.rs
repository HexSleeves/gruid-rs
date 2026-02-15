//! Graphical roguelike demo using winit + softbuffer.
//!
//! Run: cargo run --bin roguelike-winit

use gruid_core::app::{AppRunner, EventLoopDriver};
use gruid_examples::{Game, HEIGHT, WIDTH};
use gruid_winit::{WinitConfig, WinitDriver};

fn main() {
    let game = Game::new();
    let driver = WinitDriver::new(WinitConfig {
        title: "gruid-rs roguelike".into(),
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
