//! **gruid-core** — Cross-platform grid-based UI and game framework (core types).
//!
//! This crate provides the foundational types used across the *gruid*
//! ecosystem: geometry primitives, styled cells, a shared-buffer grid, input
//! events, and the Elm-architecture application loop.

pub mod geom;
pub mod style;
pub mod cell;
pub mod grid;
pub mod messages;
pub mod app;
pub mod recording;

pub use geom::{Point, Range};
pub use style::{Style, Color, AttrMask};
pub use cell::Cell;
pub use grid::Grid;
pub use messages::*;
pub use app::{App, AppConfig, Model, Driver, EventLoopDriver, AppRunner, Effect, Cmd};
