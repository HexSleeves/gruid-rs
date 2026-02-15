//! Connected-component labelling.

use gruid_core::Point;

use crate::traits::Pather;
use crate::PathRange;

impl PathRange {
    /// Label every cell in the range with a connected-component ID.
    ///
    /// Two cells belong to the same component if there is a path of
    /// neighbours (as defined by `pather`) between them.  After this call
    /// use [`cc_at`](Self::cc_at) to query the label of a given point.
    pub fn cc_map_all<P: Pather>(&mut self, pather: &P) {
        let len = self.rng.len();
        // Reset labels.
        for v in self.cc_labels.iter_mut() {
            *v = -1;
        }

        let mut label: i32 = 0;
        let mut nbuf = std::mem::take(&mut self.nbuf);

        for start in 0..len {
            if self.cc_labels[start] >= 0 {
                continue;
            }

            // Iterative DFS from `start`.
            self.cc_stack.clear();
            self.cc_stack.push(start);
            self.cc_labels[start] = label;

            while let Some(ci) = self.cc_stack.pop() {
                let cp = self.point(ci);
                nbuf.clear();
                pather.neighbors(cp, &mut nbuf);

                for i in 0..nbuf.len() {
                    let np = nbuf[i];
                    if let Some(ni) = self.idx(np) {
                        if self.cc_labels[ni] < 0 {
                            self.cc_labels[ni] = label;
                            self.cc_stack.push(ni);
                        }
                    }
                }
            }

            label += 1;
        }

        self.nbuf = nbuf;
    }

    /// Flood-fill from a single point and return the set of connected cells.
    ///
    /// Internally this also populates the `cc_labels` array, but only cells
    /// reachable from `p` will have a meaningful label.
    pub fn cc_map<P: Pather>(&mut self, pather: &P, p: Point) -> Vec<Point> {
        // Reset labels.
        for v in self.cc_labels.iter_mut() {
            *v = -1;
        }

        let mut result = Vec::new();
        let Some(si) = self.idx(p) else {
            return result;
        };

        let mut nbuf = std::mem::take(&mut self.nbuf);

        self.cc_stack.clear();
        self.cc_stack.push(si);
        self.cc_labels[si] = 0;
        result.push(p);

        while let Some(ci) = self.cc_stack.pop() {
            let cp = self.point(ci);
            nbuf.clear();
            pather.neighbors(cp, &mut nbuf);

            for i in 0..nbuf.len() {
                let np = nbuf[i];
                if let Some(ni) = self.idx(np) {
                    if self.cc_labels[ni] < 0 {
                        self.cc_labels[ni] = 0;
                        self.cc_stack.push(ni);
                        result.push(np);
                    }
                }
            }
        }

        self.nbuf = nbuf;
        result
    }

    /// Query the connected-component label of a point.
    ///
    /// Returns `None` if the point is outside the range or was not labelled
    /// (i.e. `cc_map_all` or `cc_map` has not been called yet, or the point
    /// had no neighbours).
    pub fn cc_at(&self, p: Point) -> Option<usize> {
        let i = self.idx(p)?;
        let label = self.cc_labels[i];
        if label < 0 {
            None
        } else {
            Some(label as usize)
        }
    }
}
