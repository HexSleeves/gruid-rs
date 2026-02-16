//! Shamogu — a roguelike game built on gruid-rs.
#![allow(dead_code)]

pub mod colors;
pub mod combat;
pub mod entity;
pub mod fov_;
pub mod game;
pub mod gamemap;
pub mod log;
pub mod model;
pub mod terrain;
pub mod tile_data;
pub mod tiles;

pub use model::{ShamoguModel, UI_HEIGHT, UI_WIDTH};
pub use tiles::ATTR_IN_MAP;
