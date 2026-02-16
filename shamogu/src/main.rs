//! Shamogu — a roguelike game built on gruid-rs.
#![allow(dead_code)]

mod colors;
mod combat;
mod entity;
mod fov_;
mod game;
mod gamemap;
mod log;
mod model;
mod terrain;

use gruid_core::app::{App, AppConfig};
use gruid_crossterm::CrosstermDriver;

use model::{ShamoguModel, UI_HEIGHT, UI_WIDTH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = ShamoguModel::new();
    let driver = CrosstermDriver::new();
    let mut app = App::new(AppConfig {
        model,
        driver,
        width: UI_WIDTH,
        height: UI_HEIGHT,
        frame_writer: None,
    });
    app.run()?;
    Ok(())
}
