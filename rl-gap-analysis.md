# gruid RL Package: Go → Rust Gap Analysis

Comprehensive comparison of every public type, method, function, constant, and struct field.

---

## 1. EventQueue (`events.go` → `events.rs`)

### Types
| Go | Rust | Status |
|---|---|---|
| `Event` (interface = `interface{}`) | generic `<E>` | ✅ (idiomatic Rust) |
| `EventQueue` struct | `EventQueue<E>` struct | ✅ |

### Constructor
| Go | Rust | Status |
|---|---|---|
| `NewEventQueue() *EventQueue` | `EventQueue::new()` | ✅ |

### Methods
| Go | Rust | Status |
|---|---|---|
| `Push(ev Event, rank int)` | `push(event, rank: i32)` | ✅ |
| `PushFirst(ev Event, rank int)` | `push_first(event, rank: i32)` | ✅ |
| `Pop() Event` | `pop() -> Option<E>` | ✅ |
| `PopR() (Event, int)` | `pop_with_rank() -> Option<(E, i32)>` | ✅ (renamed) |
| `Empty() bool` | `is_empty() -> bool` | ✅ (renamed) |
| `Filter(fn func(Event) bool)` | `filter(predicate: impl Fn(&E) -> bool)` | ✅ |
| `GobDecode([]byte) error` | — | ❌ (no serialization) |
| `GobEncode() ([]byte, error)` | — | ❌ (no serialization) |
| — | `len() -> usize` | ✅ (extra in Rust, not in Go) |

---

## 2. FOV (`fov.go` → `fov.rs`)

### Types
| Go | Rust | Status |
|---|---|---|
| `FOV` struct | `FOV` struct | ✅ |
| `LightNode` struct | `LightNode` struct | ✅ |
| `Lighter` interface | `Lighter` trait | ✅ |

### LightNode Fields
| Go | Rust | Status |
|---|---|---|
| `P gruid.Point` | `pos: Point` | ✅ (renamed `P` → `pos`) |
| `Cost int` | `cost: i32` | ✅ |

### Lighter Interface/Trait
| Go | Rust | Status |
|---|---|---|
| `Cost(src, from, to Point) int` | `cost(src, from, to: Point) -> i32` | ✅ |
| `MaxCost(src Point) int` | `max_cost(src: Point) -> i32` | ✅ |

### FOV Constructor
| Go | Rust | Status |
|---|---|---|
| `NewFOV(rg Range) *FOV` | `FOV::new(range: Range)` | ✅ |

### FOV Methods
| Go | Rust | Status |
|---|---|---|
| `SetRange(rg Range)` | `set_range(range: Range)` | ✅ |
| `Range() Range` | `range_() -> Range` | ✅ (renamed due to keyword) |
| `At(p Point) (int, bool)` | `at(p: Point) -> Option<i32>` | ✅ |
| `Visible(p Point) bool` | `visible(p: Point) -> bool` | ✅ |
| `VisionMap(lt Lighter, src Point) []LightNode` | `vision_map(lt, src) -> &[LightNode]` | ✅ |
| `LightMap(lt Lighter, srcs []Point) []LightNode` | `light_map(lt, srcs) -> &[LightNode]` | ✅ |
| `SSCVisionMap(src, maxDepth, passable, diags) []Point` | `ssc_vision_map(src, max_depth, passable, diags) -> &[Point]` | ✅ |
| `SSCLightMap(srcs, maxDepth, passable, diags) []Point` | `ssc_light_map(srcs, max_depth, passable, diags) -> &[Point]` | ✅ |
| `Ray(lt Lighter, to Point) []LightNode` | `ray(lt, to) -> Option<&[LightNode]>` | ✅ |
| `From(lt Lighter, to Point) (LightNode, bool)` | `from(lt, to) -> Option<LightNode>` | ⚠️ see note |
| `Iter(fn func(LightNode))` | `iter_lighted() -> impl Iterator` | ✅ (idiomatic) |
| `IterSSC(fn func(Point))` | `iter_visible() -> impl Iterator` | ✅ (idiomatic) |
| `GobDecode([]byte) error` | — | ❌ (no serialization) |
| `GobEncode() ([]byte, error)` | — | ❌ (no serialization) |

**Note on `From`**: The Rust `from()` method exists but its return value computation looks different from the Go version. In Go, `From` returns `LightNode{P: n.P, Cost: n.Cost}` where cost is the stored internal cost from `from_internal`. The Rust version adds an extra `lt.cost()` call (`ln.cost - 1 + lt.cost(...)`) which doesn't match the Go source. The Go version's `From` simply returns the parent node with its stored cost. **This is a behavioral bug in the Rust `from()` method.**

---

## 3. Grid (`grid.go` → `grid.rs`)

### Types
| Go | Rust | Status |
|---|---|---|
| `Cell` (type alias `int`) | `Cell` (newtype `i32`) | ✅ |
| `Grid` struct | `Grid` struct | ✅ |
| `GridIterator` struct | `GridIter` struct | ⚠️ (different API) |

### Grid Constructor
| Go | Rust | Status |
|---|---|---|
| `NewGrid(w, h int) Grid` | `Grid::new(width, height: i32)` | ✅ |

### Grid Methods
| Go | Rust | Status |
|---|---|---|
| `Bounds() Range` | `bounds() -> Range` | ✅ |
| `Range() Range` | `range_() -> Range` | ✅ |
| `Size() Point` | `size() -> Point` | ✅ |
| `Contains(p Point) bool` | `contains(p: Point) -> bool` | ✅ |
| `At(p Point) Cell` | `at(p: Point) -> Option<Cell>` | ✅ (returns Option instead of zero) |
| `AtU(p Point) Cell` | — | ❌ unchecked access not ported |
| `Set(p Point, c Cell)` | `set(p: Point, cell: Cell)` | ✅ |
| `Fill(c Cell)` | `fill(cell: Cell)` | ✅ |
| `FillFunc(fn func() Cell)` | `fill_fn(f: impl FnMut() -> Cell)` | ✅ |
| `Slice(rg Range) Grid` | `slice(rng: Range) -> Grid` | ✅ |
| `Copy(src Grid) Point` | `copy_from(other: &Grid)` | ⚠️ does not return the copied size |
| `Map(fn func(Point, Cell) Cell)` | `map_cells(f: impl FnMut(Point, Cell) -> Cell)` | ✅ |
| `Iter(fn func(Point, Cell))` | `iter() -> GridIter` | ✅ (deprecated in Go, iterator in Rust) |
| `Count(c Cell) int` | `count(cell: Cell) -> usize` | ✅ |
| `CountFunc(fn func(Cell) bool) int` | `count_fn(f) -> usize` | ⚠️ Rust signature is `(Point, Cell) -> bool` vs Go `(Cell) -> bool` |
| `Resize(w, h int) Grid` | — | ❌ not ported |
| `Iterator() GridIterator` | `iter()` returns snapshot-based iter | ⚠️ different semantics |
| `All() iter.Seq2[Point, Cell]` | — | ❌ Go 1.23 range-over-func, N/A |
| `Points() iter.Seq[Point]` | — | ❌ Go 1.23 range-over-func, N/A |
| `GobDecode([]byte) error` | — | ❌ |
| `GobEncode() ([]byte, error)` | — | ❌ |
| — | `width() -> i32` | ✅ extra |
| — | `height() -> i32` | ✅ extra |

### GridIterator (Go) vs GridIter (Rust)
| Go | Rust | Status |
|---|---|---|
| `Iterator() GridIterator` | `iter() -> GridIter` | ⚠️ |
| `Next() bool` | Rust `Iterator::next()` | ⚠️ different pattern |
| `P() Point` | built into iterator item `(Point, Cell)` | ⚠️ |
| `Cell() Cell` | built into iterator item | ⚠️ |
| `SetP(p Point)` | — | ❌ |
| `SetCell(c Cell)` | — | ❌ (Rust iter is a snapshot, can't mutate) |
| `Reset()` | — | ❌ |

**Architecture note**: The Go `GridIterator` gives mutable access to the underlying cells (`SetCell`). The Rust `GridIter` takes a snapshot of the data (borrows and copies into a Vec), so it cannot mutate the grid. This is a fundamental design difference.

---

## 4. MapGen (`mapgen.go` → `mapgen.rs`)

### Types
| Go | Rust | Status |
|---|---|---|
| `MapGen` struct | `MapGen<R: Rng>` struct | ✅ |
| `RandomWalker` interface | `RandomWalker` trait | ⚠️ see below |
| `CellularAutomataRule` struct | `CellularAutomataRule` struct | ✅ |
| `Vault` struct | `Vault` struct | ✅ (separate file) |

### MapGen Fields
| Go | Rust | Status |
|---|---|---|
| `Rand *rand.Rand` | `rng: R` | ✅ |
| `Grid Grid` | `grid: Grid` | ✅ |

### MapGen Constructor
| Go | Rust | Status |
|---|---|---|
| Direct struct literal `MapGen{Rand: rd, Grid: gd}` | `MapGen::with_grid(grid, rng)` | ✅ |

### MapGen Methods
| Go | Rust | Status |
|---|---|---|
| `WithGrid(gd Grid) MapGen` | — | ❌ not ported |
| `RandomWalkCave(walker, c, fillp, walks) int` | `random_walk_cave(walker, cell, fill_pct, walks) -> usize` | ✅ |
| `CellularAutomataCave(wall, ground, winit, rules) int` | `cellular_automata_cave(wall, ground, wall_init_pct, rules) -> usize` | ✅ |
| `KeepCC(pr *PathRange, p Point, wall Cell) int` | `keep_connected(pr, p, wall) -> usize` | ✅ (renamed) |

### RandomWalker Interface/Trait
| Go | Rust | Status |
|---|---|---|
| `Neighbor(Point) Point` | `neighbor(p: Point, rng: &mut impl Rng) -> Point` | ⚠️ different signature |

The Go `RandomWalker.Neighbor` takes only the point; the walker is expected to carry its own RNG. The Rust trait passes the RNG explicitly. This is a design choice difference.

### CellularAutomataRule Fields
| Go | Rust | Status |
|---|---|---|
| `WCutoff1 int` | `w_cutoff1: i32` | ✅ |
| `WCutoff2 int` | `w_cutoff2: i32` | ✅ |
| `WallsOutOfRange bool` | `walls_out_of_range: bool` | ✅ |
| `Reps int` | `reps: usize` | ✅ |

### Extra in Rust
| Rust | Status |
|---|---|
| `FourDirectionWalker` struct + impl | ✅ extra convenience type |
| `CellularAutomataRule::default()` | ✅ extra |

---

## 5. Vault (`mapgen.go` → `vault.rs`)

### Types
| Go | Rust | Status |
|---|---|---|
| `Vault` struct | `Vault` struct | ✅ |
| — | `VaultError` enum | ✅ (Go returns `error`) |

### Constructor
| Go | Rust | Status |
|---|---|---|
| `NewVault(s string) (*Vault, error)` | `Vault::new(s: &str) -> Result<Self, VaultError>` | ✅ |

### Methods
| Go | Rust | Status |
|---|---|---|
| `Content() string` | `content() -> &str` | ✅ |
| `Size() Point` | `size() -> Point` | ✅ |
| `SetRunes(s string)` | `set_runes(s: &str)` | ✅ |
| `Runes() string` | `runes() -> &str` | ✅ |
| `Parse(s string) error` | `parse(s: &str) -> Result<(), VaultError>` | ✅ |
| `Iter(fn func(Point, rune))` | `iter(f: impl FnMut(Point, char))` | ✅ |
| `Draw(gd Grid, fn func(rune) Cell) Grid` | `draw(grid: &Grid, f: impl Fn(char) -> Cell) -> Grid` | ✅ |
| `Reflect()` | `reflect()` | ✅ |
| `Rotate(n int)` | `rotate(n: i32)` | ✅ |

---

## Summary of Gaps

### ❌ Missing (not present in Rust at all)

| Category | Item | Importance |
|---|---|---|
| EventQueue | `GobDecode`/`GobEncode` (serialization) | Medium — serde could be added |
| FOV | `GobDecode`/`GobEncode` (serialization) | Medium |
| Grid | `GobDecode`/`GobEncode` (serialization) | Medium |
| Grid | `AtU(p Point) Cell` — unchecked access | Low (perf optimization) |
| Grid | `Resize(w, h int) Grid` — grow underlying buffer | Medium |
| Grid | `GridIterator.SetP(p)` — repositioning | Low |
| Grid | `GridIterator.SetCell(c)` — mutable iteration | **High** — enables in-place map ops |
| Grid | `GridIterator.Reset()` — reuse iterator | Low |
| Grid | `All()` / `Points()` — Go 1.23 range iterators | Low (N/A in Rust) |
| MapGen | `WithGrid(gd Grid) MapGen` — derived mapgen | Low |

### ⚠️ Partial / Behavioral Differences

| Category | Item | Issue |
|---|---|---|
| FOV `from()` | Return value | Rust adds extra `lt.cost()` not present in Go |
| Grid `copy_from` | Return type | Go returns `Point` (copied size); Rust returns nothing |
| Grid `count_fn` | Callback signature | Go: `func(Cell) bool`; Rust: `(Point, Cell) -> bool` |
| Grid `iter()` | Semantics | Rust takes a snapshot (copies data); Go iterates live (mutable) |
| RandomWalker | Signature | Go: `Neighbor(Point) Point`; Rust: `neighbor(Point, &mut Rng) -> Point` |

### ✅ Fully Ported
- EventQueue core API (Push, PushFirst, Pop, PopR, Filter, Empty)
- FOV VisionMap algorithm (ray-based, octant parents)
- FOV SSC algorithm (symmetric shadow casting with diags flag)
- FOV LightMap (multi-source)
- FOV SSCLightMap (multi-source SSC)
- FOV Ray traceback
- FOV At / Visible queries
- Grid core API (New, At, Set, Fill, FillFunc, Slice, Copy, Map, Count, Contains, Bounds, Range, Size)
- MapGen RandomWalkCave
- MapGen CellularAutomataCave
- MapGen KeepCC (as keep_connected)
- CellularAutomataRule (all fields)
- Vault (Parse, Iter, Draw, Reflect, Rotate, Content, Size, SetRunes, Runes)
