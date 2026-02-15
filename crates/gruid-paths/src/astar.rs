use std::collections::BinaryHeap;

use gruid_core::Point;

use crate::PathRange;
use crate::pathrange::{NodeRef, UNREACHABLE};
use crate::traits::AstarPather;

impl PathRange {
    /// Compute the shortest path from `from` to `to` using A*.
    ///
    /// Returns the full path (including both endpoints) or `None` if no path
    /// exists within the current range.
    pub fn astar_path<P: AstarPather>(
        &mut self,
        pather: &P,
        from: Point,
        to: Point,
    ) -> Option<Vec<Point>> {
        let start_idx = self.idx(from)?;
        let goal_idx = self.idx(to)?;

        if start_idx == goal_idx {
            return Some(vec![from]);
        }

        // Bump generation to lazily invalidate all nodes.
        self.astar_generation = self.astar_generation.wrapping_add(1);
        let cur_gen = self.astar_generation;

        // Initialise the start node.
        {
            let node = &mut self.astar_nodes[start_idx];
            node.g = 0;
            node.f = pather.estimate(from, to);
            node.parent = usize::MAX;
            node.generation = cur_gen;
            node.open = true;
        }

        let mut open: BinaryHeap<NodeRef> = BinaryHeap::new();
        open.push(NodeRef {
            idx: start_idx,
            f: self.astar_nodes[start_idx].f,
        });

        let mut nbuf = std::mem::take(&mut self.nbuf);

        let found = 'search: loop {
            let Some(current) = open.pop() else {
                break 'search false;
            };

            let ci = current.idx;

            // Skip stale entries.
            if self.astar_nodes[ci].generation != cur_gen || !self.astar_nodes[ci].open {
                continue;
            }

            if ci == goal_idx {
                break 'search true;
            }

            self.astar_nodes[ci].open = false;
            let current_g = self.astar_nodes[ci].g;
            let current_point = self.point(ci);

            nbuf.clear();
            pather.neighbors(current_point, &mut nbuf);

            for &np in nbuf.iter() {
                let Some(ni) = self.idx(np) else {
                    continue;
                };
                let tentative_g = current_g + pather.cost(current_point, np);

                let n = &mut self.astar_nodes[ni];
                if n.generation == cur_gen {
                    // Already visited this generation.
                    if tentative_g >= n.g {
                        continue;
                    }
                } else {
                    n.generation = cur_gen;
                    n.g = UNREACHABLE;
                }

                n.g = tentative_g;
                n.f = tentative_g + pather.estimate(np, to);
                n.parent = ci;
                n.open = true;

                open.push(NodeRef { idx: ni, f: n.f });
            }
        };

        self.nbuf = nbuf;

        if !found {
            return None;
        }

        // Reconstruct path.
        let mut path = Vec::new();
        let mut ci = goal_idx;
        while ci != usize::MAX {
            path.push(self.point(ci));
            ci = self.astar_nodes[ci].parent;
        }
        path.reverse();
        Some(path)
    }
}
