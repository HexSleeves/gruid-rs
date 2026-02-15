//! Pathfinding algorithms for grid-based games.
//!
//! This crate provides efficient implementations of common pathfinding and
//! graph-search algorithms on 2D grids:
//!
//! - **A\*** shortest-path search ([`PathRange::astar_path`])
//! - **Dijkstra** multi-source distance maps ([`PathRange::dijkstra_map`])
//! - **BFS** unweighted distance maps ([`PathRange::bfs_map`])
//! - **Jump Point Search** optimised uniform-cost pathfinding ([`PathRange::jps_path`])
//! - **Connected Components** labelling ([`PathRange::cc_map_all`], [`PathRange::cc_map`])
//!
//! All algorithms operate through [`PathRange`], which owns and reuses internal
//! caches so that repeated queries incur zero allocations after warm-up.
//!
//! # Trait hierarchy
//!
//! | Trait | Required for |
//! |---|---|
//! | [`Pather`] | BFS, connected components |
//! | [`WeightedPather`] : [`Pather`] | Dijkstra |
//! | [`AstarPather`] : [`WeightedPather`] | A* |

mod traits;
mod neighbors;
mod distance;
mod pathrange;
mod astar;
mod dijkstra;
mod bfs;
mod jps;
mod cc;

pub use traits::{Pather, WeightedPather, AstarPather};
pub use neighbors::Neighbors;
pub use distance::{manhattan, chebyshev};
pub use pathrange::{PathRange, PathNode, UNREACHABLE};
