//! Shamogu terminal (crossterm) entry point.

use gruid_core::app::{App, AppConfig};
use gruid_crossterm::CrosstermDriver;
use shamogu_lib::{ShamoguModel, UI_HEIGHT, UI_WIDTH};

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
