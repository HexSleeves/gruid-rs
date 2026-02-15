//! Terminal roguelike demo using crossterm.
//!
//! Run: cargo run --bin roguelike

use gruid_core::app::{App, AppConfig};
use gruid_crossterm::CrosstermDriver;
use gruid_examples::{Game, WIDTH, HEIGHT};

fn main() {
    let game = Game::new();
    let driver = CrosstermDriver::new();
    let mut app = App::new(AppConfig {
        model: game,
        driver,
        width: WIDTH,
        height: HEIGHT,
        frame_writer: None,
    });

    if let Err(e) = app.run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
