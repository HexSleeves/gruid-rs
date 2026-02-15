use std::collections::VecDeque;

use gruid_core::Point;

use crate::PathRange;
use crate::pathrange::{PathNode, UNREACHABLE};
use crate::traits::Pather;

impl PathRange {
    /// Compute a multi-source breadth-first search distance map.
    ///
    /// Each step has cost 1. Expansion stops when the distance exceeds
    /// `max_dist`. Returns a slice of all reached nodes.
    pub fn bfs_map<P: Pather>(
        &mut self,
        pather: &P,
        sources: &[Point],
        max_dist: i32,
    ) -> &[PathNode] {
        // Reset.
        for v in self.bfs_map.iter_mut() {
            *v = UNREACHABLE;
        }
        self.bfs_results.clear();

        let mut queue: VecDeque<usize> = VecDeque::new();

        for &src in sources {
            if let Some(si) = self.idx(src) {
                if self.bfs_map[si] != UNREACHABLE {
                    continue;
                }
                self.bfs_map[si] = 0;
                queue.push_back(si);
                self.bfs_results.push(PathNode { pos: src, cost: 0 });
            }
        }

        let mut nbuf = std::mem::take(&mut self.nbuf);

        while let Some(ci) = queue.pop_front() {
            let current_dist = self.bfs_map[ci];
            let cp = self.point(ci);

            nbuf.clear();
            pather.neighbors(cp, &mut nbuf);

            for &np in nbuf.iter() {
                let Some(ni) = self.idx(np) else {
                    continue;
                };
                if self.bfs_map[ni] != UNREACHABLE {
                    continue;
                }
                let nd = current_dist + 1;
                if nd > max_dist {
                    continue;
                }
                self.bfs_map[ni] = nd;
                queue.push_back(ni);
                self.bfs_results.push(PathNode { pos: np, cost: nd });
            }
        }

        self.nbuf = nbuf;
        &self.bfs_results
    }

    /// Query the BFS distance at a specific point.
    ///
    /// Returns [`UNREACHABLE`] if the point is outside the range or was not
    /// reached by the last `bfs_map` call.
    pub fn bfs_at(&self, p: Point) -> i32 {
        match self.idx(p) {
            Some(i) => self.bfs_map[i],
            None => UNREACHABLE,
        }
    }
}
