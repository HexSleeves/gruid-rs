use gruid_core::{Point, Range};

/// A position with an associated cost, returned from Dijkstra / BFS map queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PathNode {
    pub pos: Point,
    pub cost: i32,
}

// ---------------------------------------------------------------------------
// Internal node for A*/Dijkstra priority-queue searches
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct Node {
    pub(crate) g: i32,
    pub(crate) f: i32,
    pub(crate) parent: usize,
    pub(crate) generation: u32,
    pub(crate) open: bool,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            g: 0,
            f: 0,
            parent: usize::MAX,
            generation: 0,
            open: false,
        }
    }
}

/// Reference into the node array, ordered by `f` for use in `BinaryHeap`.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) struct NodeRef {
    pub(crate) idx: usize,
    pub(crate) f: i32,
}

impl Ord for NodeRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse so BinaryHeap (max-heap) pops smallest f first.
        other.f.cmp(&self.f)
    }
}

impl PartialOrd for NodeRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Sentinel value meaning "unreachable" in BFS / Dijkstra maps.
pub const UNREACHABLE: i32 = i32::MAX;

// ---------------------------------------------------------------------------
// PathRange
// ---------------------------------------------------------------------------

/// Central coordinator for pathfinding on a grid rectangle.
///
/// `PathRange` owns all internal caches (open lists, node arrays, BFS maps,
/// connected-component labels, etc.) so that repeated queries incur no
/// allocations after the first use.
pub struct PathRange {
    pub(crate) rng: Range,
    pub(crate) width: usize,
    // A* / JPS caches
    pub(crate) astar_nodes: Vec<Node>,
    pub(crate) astar_generation: u32,
    // Dijkstra caches
    pub(crate) dijkstra_nodes: Vec<Node>,
    pub(crate) dijkstra_generation: u32,
    pub(crate) dijkstra_results: Vec<PathNode>,
    pub(crate) dijkstra_map: Vec<i32>,
    // BFS caches
    pub(crate) bfs_map: Vec<i32>,
    pub(crate) bfs_queue: Vec<usize>,
    pub(crate) bfs_results: Vec<PathNode>,
    // CC caches
    pub(crate) cc_labels: Vec<i32>,
    pub(crate) cc_stack: Vec<usize>,
    // shared scratch buffer for neighbor queries
    pub(crate) nbuf: Vec<Point>,
}

impl PathRange {
    /// Create a new `PathRange` for the given grid rectangle.
    pub fn new(rng: Range) -> Self {
        let w = rng.width().max(0) as usize;
        let len = rng.len();
        Self {
            rng,
            width: w,
            astar_nodes: vec![Node::default(); len],
            astar_generation: 0,
            dijkstra_nodes: vec![Node::default(); len],
            dijkstra_generation: 0,
            dijkstra_results: Vec::new(),
            dijkstra_map: vec![UNREACHABLE; len],
            bfs_map: vec![UNREACHABLE; len],
            bfs_queue: Vec::new(),
            bfs_results: Vec::new(),
            cc_labels: vec![-1; len],
            cc_stack: Vec::new(),
            nbuf: Vec::with_capacity(8),
        }
    }

    /// Replace the underlying range, reallocating caches as needed.
    ///
    /// If the new size fits within existing capacity, caches are preserved
    /// and only generation counters are bumped (matching Go gruid's
    /// `SetRange` behaviour). Otherwise caches are reallocated.
    pub fn set_range(&mut self, rng: Range) {
        let new_len = rng.len();
        let old_capacity = self.astar_nodes.len();
        self.rng = rng;
        self.width = rng.width().max(0) as usize;

        if new_len <= old_capacity {
            // Fits within existing capacity — just bump generations so
            // stale entries are ignored, no reallocation needed.
            self.astar_generation = self.astar_generation.wrapping_add(1);
            self.dijkstra_generation = self.dijkstra_generation.wrapping_add(1);
            // Clear result vectors (they hold variable-length query output).
            self.dijkstra_results.clear();
            self.bfs_results.clear();
            return;
        }

        // New size exceeds capacity — reallocate everything.
        self.astar_nodes.clear();
        self.astar_nodes.resize(new_len, Node::default());
        self.astar_generation = 0;

        self.dijkstra_nodes.clear();
        self.dijkstra_nodes.resize(new_len, Node::default());
        self.dijkstra_generation = 0;
        self.dijkstra_results.clear();
        self.dijkstra_map.clear();
        self.dijkstra_map.resize(new_len, UNREACHABLE);

        self.bfs_map.clear();
        self.bfs_map.resize(new_len, UNREACHABLE);
        self.bfs_queue.clear();
        self.bfs_results.clear();

        self.cc_labels.clear();
        self.cc_labels.resize(new_len, -1);
        self.cc_stack.clear();
    }

    /// The grid rectangle being used.
    #[inline]
    pub fn range(&self) -> Range {
        self.rng
    }

    // -----------------------------------------------------------------------
    // Coordinate helpers
    // -----------------------------------------------------------------------

    /// Convert a `Point` to a flat index. Returns `None` if out of range.
    #[inline]
    pub(crate) fn idx(&self, p: Point) -> Option<usize> {
        if !self.rng.contains(p) {
            return None;
        }
        let x = (p.x - self.rng.min.x) as usize;
        let y = (p.y - self.rng.min.y) as usize;
        Some(y * self.width + x)
    }

    /// Convert a flat index back to a `Point`.
    #[inline]
    pub(crate) fn point(&self, idx: usize) -> Point {
        let x = (idx % self.width) as i32 + self.rng.min.x;
        let y = (idx / self.width) as i32 + self.rng.min.y;
        Point::new(x, y)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PathRange {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rng.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PathRange {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let range = gruid_core::Range::deserialize(deserializer)?;
        Ok(PathRange::new(range))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gruid_core::Range;

    #[test]
    fn set_range_smaller_preserves_capacity() {
        let rng = Range::new(0, 0, 20, 20);
        let mut pr = PathRange::new(rng);
        let original_cap = pr.astar_nodes.len(); // 400

        // Shrink to a smaller range — should NOT reallocate.
        let small = Range::new(0, 0, 5, 5);
        pr.set_range(small);
        assert_eq!(pr.range(), small);
        assert_eq!(pr.astar_nodes.len(), original_cap); // still 400
        assert_eq!(pr.width, 5);
        // Generations should have bumped (stale entries ignored).
        assert!(pr.astar_generation > 0 || pr.dijkstra_generation > 0);
    }

    #[test]
    fn set_range_larger_reallocates() {
        let rng = Range::new(0, 0, 5, 5);
        let mut pr = PathRange::new(rng);
        let old_cap = pr.astar_nodes.len(); // 25

        // Grow beyond capacity — must reallocate.
        let big = Range::new(0, 0, 20, 20);
        pr.set_range(big);
        assert_eq!(pr.range(), big);
        assert!(pr.astar_nodes.len() > old_cap);
        assert_eq!(pr.astar_nodes.len(), 400);
    }

    #[test]
    fn set_range_equal_preserves_capacity() {
        let rng = Range::new(0, 0, 10, 10);
        let mut pr = PathRange::new(rng);
        let cap = pr.astar_nodes.len();

        // Same size but different origin — should preserve.
        let shifted = Range::new(5, 5, 15, 15);
        pr.set_range(shifted);
        assert_eq!(pr.astar_nodes.len(), cap);
        assert_eq!(pr.range(), shifted);
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use gruid_core::{Point, Range};

    #[test]
    fn pathnode_round_trip() {
        let node = PathNode {
            pos: Point::new(3, 7),
            cost: 42,
        };
        let json = serde_json::to_string(&node).unwrap();
        let back: PathNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, back);
    }

    #[test]
    fn pathrange_round_trip() {
        let rng = Range {
            min: Point::new(1, 2),
            max: Point::new(10, 20),
        };
        let pr = PathRange::new(rng);
        let json = serde_json::to_string(&pr).unwrap();
        let back: PathRange = serde_json::from_str(&json).unwrap();
        assert_eq!(back.range(), rng);
        // Caches are freshly initialized (not serialized).
        assert_eq!(back.astar_generation, 0);
        assert_eq!(back.dijkstra_generation, 0);
        assert_eq!(back.bfs_map.len(), rng.len());
    }
}
