use std::collections::BinaryHeap;

use gruid_core::Point;

use crate::PathRange;
use crate::pathrange::{NodeRef, PathNode, UNREACHABLE};
use crate::traits::WeightedPather;

impl PathRange {
    /// Compute a multi-source Dijkstra distance map.
    ///
    /// Every source starts at cost 0. Expansion stops when the cumulative
    /// cost exceeds `max_cost`. Returns a slice of all reached nodes.
    pub fn dijkstra_map<P: WeightedPather>(
        &mut self,
        pather: &P,
        sources: &[Point],
        max_cost: i32,
    ) -> &[PathNode] {
        // Reset the flat cost map.
        for v in self.dijkstra_map.iter_mut() {
            *v = UNREACHABLE;
        }
        self.dijkstra_results.clear();

        self.dijkstra_generation = self.dijkstra_generation.wrapping_add(1);
        let cur_gen = self.dijkstra_generation;

        let mut open: BinaryHeap<NodeRef> = BinaryHeap::new();

        // Seed sources.
        for &src in sources {
            if let Some(si) = self.idx(src) {
                let n = &mut self.dijkstra_nodes[si];
                n.g = 0;
                n.f = 0;
                n.generation = cur_gen;
                n.open = true;
                self.dijkstra_map[si] = 0;
                open.push(NodeRef { idx: si, f: 0 });
            }
        }

        let mut nbuf = std::mem::take(&mut self.nbuf);

        while let Some(current) = open.pop() {
            let ci = current.idx;
            let cn = &self.dijkstra_nodes[ci];
            if cn.generation != cur_gen || !cn.open {
                continue;
            }
            let current_g = cn.g;
            self.dijkstra_nodes[ci].open = false;

            let cp = self.point(ci);
            self.dijkstra_results.push(PathNode {
                pos: cp,
                cost: current_g,
            });

            nbuf.clear();
            pather.neighbors(cp, &mut nbuf);

            for &np in nbuf.iter() {
                let Some(ni) = self.idx(np) else {
                    continue;
                };
                let tentative = current_g + pather.cost(cp, np);
                if tentative > max_cost {
                    continue;
                }

                let n = &mut self.dijkstra_nodes[ni];
                if n.generation == cur_gen {
                    if tentative >= n.g {
                        continue;
                    }
                } else {
                    n.generation = cur_gen;
                    n.g = UNREACHABLE;
                }

                n.g = tentative;
                n.f = tentative;
                n.open = true;
                self.dijkstra_map[ni] = tentative;
                open.push(NodeRef {
                    idx: ni,
                    f: tentative,
                });
            }
        }

        self.nbuf = nbuf;
        &self.dijkstra_results
    }

    /// Query the Dijkstra cost at a specific point.
    ///
    /// Returns [`UNREACHABLE`] if the point is outside the range or was not
    /// reached by the last `dijkstra_map` call.
    pub fn dijkstra_at(&self, p: Point) -> i32 {
        match self.idx(p) {
            Some(i) => self.dijkstra_map[i],
            None => UNREACHABLE,
        }
    }
}
