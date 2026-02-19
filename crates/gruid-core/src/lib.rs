//! **gruid-core** — Cross-platform grid-based UI and game framework (core types).
//!
//! This crate provides the foundational types used across the *gruid*
//! ecosystem: geometry primitives, styled cells, a shared-buffer grid, input
//! events, and the Elm-architecture application loop.

pub mod app;
pub mod cell;
pub mod geom;
pub mod grid;
pub mod messages;
pub mod recording;
pub mod style;
pub mod tiles;

pub use app::{App, AppConfig, AppRunner, Cmd, Driver, Effect, EventLoopDriver, Model};
pub use cell::Cell;
pub use geom::{Point, Range};
pub use grid::Grid;
pub use messages::*;
pub use style::{AttrMask, Color, Style};
pub use tiles::TileManager;
