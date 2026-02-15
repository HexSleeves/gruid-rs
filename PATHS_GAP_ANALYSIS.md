# gruid-paths: Go → Rust Gap Analysis

## Interfaces / Traits

| Go Interface | Go Signature | Rust Trait | Status |
|---|---|---|---|
| `Pather` | `Neighbors(gruid.Point) []gruid.Point` | `Pather` | ⚠️ Partial — signature differs: Rust uses `fn neighbors(&self, p: Point, buf: &mut Vec<Point>)` (push into buffer instead of returning slice). Semantically equivalent but API different. |
| `Dijkstra` (extends `Pather`) | `Cost(gruid.Point, gruid.Point) int` | `WeightedPather` (extends `Pather`) | ✅ Present — renamed. `fn cost(&self, from: Point, to: Point) -> i32` |
| `Astar` (extends `Dijkstra`) | `Estimation(gruid.Point, gruid.Point) int` | `AstarPather` (extends `WeightedPather`) | ✅ Present — renamed. `fn estimate(&self, from: Point, to: Point) -> i32` |

## Types

| Go Type | Rust Type | Status |
|---|---|---|
| `PathRange` (struct) | `PathRange` (struct) | ✅ Present |
| `Node` (struct: `P gruid.Point`, `Cost int`) | `PathNode` (struct: `pos: Point`, `cost: i32`) | ✅ Present — renamed |
| `Neighbors` (struct, cached neighbor helper) | `Neighbors` (struct) | ⚠️ Partial — see method table below |
| `nodeMap` (internal) | `Vec<Node>` + generation counter (internal) | ✅ Present — different internal design |
| `priorityQueue` (internal) | `BinaryHeap<NodeRef>` (internal) | ✅ Present — uses std library |

## PathRange Constructor & Methods

| Go Function/Method | Rust Equivalent | Status |
|---|---|---|
| `NewPathRange(rg gruid.Range) *PathRange` | `PathRange::new(rng: Range) -> Self` | ✅ Present |
| `(pr *PathRange) SetRange(rg gruid.Range)` | `PathRange::set_range(&mut self, rng: Range)` | ⚠️ Partial — Go version preserves caches if new size ≤ old capacity; Rust always reallocates |
| `(pr *PathRange) Range() gruid.Range` | `PathRange::range(&self) -> Range` | ✅ Present |
| `GobEncode() ([]byte, error)` | — | ❌ Missing — no serde/serialization support |
| `GobDecode([]byte) error` | — | ❌ Missing — no serde/serialization support |

## A* Pathfinding

| Go | Rust | Status |
|---|---|---|
| `(pr *PathRange) AstarPath(ast Astar, from, to gruid.Point) []gruid.Point` | `PathRange::astar_path<P: AstarPather>(&mut self, pather: &P, from: Point, to: Point) -> Option<Vec<Point>>` | ✅ Present — Rust returns `Option` instead of nil slice; Rust takes `&P` instead of by-value. Same semantics. |

## Dijkstra Map

| Go | Rust | Status |
|---|---|---|
| `(pr *PathRange) DijkstraMap(dij Dijkstra, sources []gruid.Point, maxCost int) []Node` | `PathRange::dijkstra_map<P: WeightedPather>(&mut self, pather: &P, sources: &[Point], max_cost: i32) -> &[PathNode]` | ✅ Present |
| `(pr *PathRange) DijkstraMapAt(p gruid.Point) int` | `PathRange::dijkstra_at(&self, p: Point) -> i32` | ✅ Present — renamed |

## BFS Map

| Go | Rust | Status |
|---|---|---|
| `(pr *PathRange) BreadthFirstMap(nb Pather, sources []gruid.Point, maxCost int) []Node` | `PathRange::bfs_map<P: Pather>(&mut self, pather: &P, sources: &[Point], max_dist: i32) -> &[PathNode]` | ✅ Present — renamed |
| `(pr *PathRange) BreadthFirstMapAt(p gruid.Point) int` | `PathRange::bfs_at(&self, p: Point) -> i32` | ✅ Present — renamed |

## JPS (Jump Point Search)

| Go | Rust | Status |
|---|---|---|
| `(pr *PathRange) JPSPath(path []gruid.Point, from, to gruid.Point, passable func(gruid.Point) bool, diags bool) []gruid.Point` | `PathRange::jps_path(&mut self, from: Point, to: Point, passable: impl Fn(Point) -> bool, diags: bool) -> Option<Vec<Point>>` | ⚠️ Partial — Rust does NOT accept a pre-allocated `path` slice for reuse (Go passes `path []gruid.Point` to avoid allocation). Rust allocates a new `Vec` each call. Returns `Option` instead of nil. |

## Connected Components

| Go | Rust | Status |
|---|---|---|
| `(pr *PathRange) CCMapAll(nb Pather)` | `PathRange::cc_map_all<P: Pather>(&mut self, pather: &P)` | ✅ Present |
| `(pr *PathRange) CCMap(nb Pather, p gruid.Point) []gruid.Point` | `PathRange::cc_map<P: Pather>(&mut self, pather: &P, p: Point) -> Vec<Point>` | ✅ Present |
| `(pr *PathRange) CCMapAt(p gruid.Point) int` | `PathRange::cc_at(&self, p: Point) -> Option<usize>` | ⚠️ Partial — Go returns `-1` for out-of-range/unlabelled; Rust returns `Option<usize>` (more idiomatic but different API). Go returns `int` (signed, 0-based where 0=first component); Rust returns `usize`. |

## Distance Functions

| Go | Rust | Status |
|---|---|---|
| `DistanceManhattan(p, q gruid.Point) int` | `manhattan(a: Point, b: Point) -> i32` | ✅ Present — renamed |
| `DistanceChebyshev(p, q gruid.Point) int` | `chebyshev(a: Point, b: Point) -> i32` | ✅ Present — renamed |

## Neighbors Helper

| Go Method | Rust Method | Status |
|---|---|---|
| `(nb *Neighbors) All(p gruid.Point, keep func(gruid.Point) bool) []gruid.Point` | `Neighbors::all(&mut self, p: Point, keep: impl Fn(Point) -> bool) -> &[Point]` | ✅ Present |
| `(nb *Neighbors) Cardinal(p gruid.Point, keep func(gruid.Point) bool) []gruid.Point` | `Neighbors::cardinal(&mut self, p: Point, keep: impl Fn(Point) -> bool) -> &[Point]` | ✅ Present |
| `(nb *Neighbors) Diagonal(p gruid.Point, keep func(gruid.Point) bool) []gruid.Point` | — | ❌ Missing — `Diagonal()` method not implemented in Rust |

## Internal / Private (not public API, but noteworthy)

| Go | Rust | Status |
|---|---|---|
| `idxToPos(i, w int) gruid.Point` | `PathRange::point(&self, idx: usize) -> Point` | ✅ Present (method instead of free function) |
| `checkNodesIdx(nm *nodeMap)` | Generation-based wrapping (`wrapping_add`) | ✅ Present — handled differently but correctly |
| Custom `heap.go` (manual priority queue) | `std::collections::BinaryHeap` | ✅ Present — uses stdlib |
| `abs(x int) int` helper | `.abs()` method on `i32` | ✅ Present (language built-in) |

## Constants

| Go | Rust | Status |
|---|---|---|
| (no explicit constant; unreachable = `maxCost + 1`) | `pub const UNREACHABLE: i32 = i32::MAX` | ⚠️ Different — Go uses `maxCost + 1` dynamically per call; Rust uses a fixed sentinel `i32::MAX`. Functionally equivalent for most uses but edge-case behavior differs. |

---

## Summary

### ❌ Missing (2 items)
1. **`Neighbors::Diagonal()`** — The Go `Neighbors` struct has a `Diagonal()` method returning 4 inter-cardinal (diagonal) neighbors filtered by a predicate. Not present in Rust.
2. **Serialization (GobEncode/GobDecode)** — Go `PathRange` implements `gob.GobEncoder` and `gob.GobDecoder`. No `serde` support in Rust.

### ⚠️ Partial (5 items)
1. **`Pather` trait signature** — Go returns `[]gruid.Point`; Rust pushes into `&mut Vec<Point>`. Same semantics but callers port differently.
2. **`SetRange` capacity optimization** — Go preserves caches if new size fits; Rust always reallocates.
3. **`CCMapAt` / `cc_at` return type** — Go returns `int` (-1 sentinel); Rust returns `Option<usize>` (idiomatic but different).
4. **`JPSPath` buffer reuse** — Go accepts a pre-allocated path slice; Rust always allocates a fresh `Vec`.
5. **Unreachable sentinel** — Go uses dynamic `maxCost + 1`; Rust uses fixed `i32::MAX`.

### ✅ Present (14 items)
1. `Pather` trait (equivalent to Go `Pather` interface)
2. `WeightedPather` trait (equivalent to Go `Dijkstra` interface)
3. `AstarPather` trait (equivalent to Go `Astar` interface)
4. `PathRange` struct + `new()` + `range()`
5. `PathNode` struct (equivalent to Go `Node`)
6. `Neighbors` struct + `all()` + `cardinal()`
7. `PathRange::astar_path()`
8. `PathRange::dijkstra_map()` + `dijkstra_at()`
9. `PathRange::bfs_map()` + `bfs_at()`
10. `PathRange::jps_path()`
11. `PathRange::cc_map_all()` + `cc_map()` + `cc_at()`
12. `manhattan()` distance function
13. `chebyshev()` distance function
14. `PathRange::set_range()`
