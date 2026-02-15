# Agent Context Dump — gruid-rs

This file contains full context for continuing development of gruid-rs.
Read this before making changes.

---

## What This Project Is

A Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) — a Go
cross-platform grid-based UI and game framework using the Elm architecture
(Model-View-Update). Designed for roguelike games but general-purpose.

**Go original:** `/home/exedev/gruid/` (10,290 LOC across 5 packages + 3 driver repos)
**Rust port:** `/home/exedev/gruid-rs/` (~7,400 LOC across 7 crates)
**Repo:** https://github.com/HexSleeves/gruid-rs

---

## Workspace Structure

```
gruid-rs/
├── Cargo.toml              # Workspace root
├── README.md
├── TODO.md                 # Full task list with priorities (READ THIS)
├── CONTEXT.md              # This file
├── crates/
│   ├── gruid-core/         # Core types: Grid, Cell, Point, Range, Style, Msg, App
│   │   └── src/
│   │       ├── lib.rs      # Re-exports everything
│   │       ├── geom.rs     # Point (i32 x,y), Range (half-open rect), iterators
│   │       ├── style.rs    # Color (u32 RGB), AttrMask (bitflags), Style (fg/bg/attrs)
│   │       ├── cell.rs     # Cell { ch: char, style: Style }
│   │       ├── grid.rs     # Grid with Rc<RefCell<GridBuffer>> shared storage, RELATIVE coords
│   │       ├── messages.rs # Key, ModMask, MouseAction, Msg enum
│   │       ├── app.rs      # Model/Driver/EventLoopDriver traits, App, AppRunner, Effect
│   │       └── recording.rs # FrameEncoder/FrameDecoder (STUB — not implemented)
│   ├── gruid-paths/        # Pathfinding algorithms
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs   # Pather, WeightedPather, AstarPather trait hierarchy
│   │       ├── neighbors.rs # Neighbors helper (cardinal + all)
│   │       ├── distance.rs # manhattan(), chebyshev()
│   │       ├── pathrange.rs # PathRange (cache owner), PathNode, Node internals
│   │       ├── astar.rs    # PathRange::astar_path() — works
│   │       ├── dijkstra.rs # PathRange::dijkstra_map/at() — works
│   │       ├── bfs.rs      # PathRange::bfs_map/at() — works
│   │       ├── jps.rs      # PathRange::jps_path() — works (8-way AND 4-way)
│   │       └── cc.rs       # PathRange::cc_map_all/cc_map/cc_at() — works
│   ├── gruid-rl/           # Roguelike utilities
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grid.rs     # rl::Grid (Cell=i32) — RELATIVE coords, matches Go
│   │       ├── fov.rs      # FOV — VisionMap + SSC, matches Go's algorithms
│   │       ├── mapgen.rs   # MapGen — cellular automata + random walk, matches Go
│   │       └── events.rs   # EventQueue<E> — works
│   ├── gruid-ui/           # UI widgets
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── styled_text.rs # StyledText with markup — partial
│   │       ├── box_.rs     # BoxDecor (Unicode box drawing) — works
│   │       ├── label.rs    # Label — partial (no auto-sizing)
│   │       ├── menu.rs     # Menu — partial (keys only, no mouse/pagination)
│   │       ├── pager.rs    # Pager — partial (basic up/down/quit only)
│   │       └── text_input.rs # TextInput — partial (no prompt, no mouse)
│   ├── gruid-tiles/        # Font-to-tile rendering (EXCLUDED from workspace build)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── drawer.rs   # Drawer using rusttype + image crate
│   ├── gruid-crossterm/    # Terminal driver — works
│   │   └── src/lib.rs  # CrosstermDriver implements Driver trait
│   └── gruid-winit/        # Native window driver — works
│       └── src/
│           ├── lib.rs      # WinitDriver implements EventLoopDriver trait
│           ├── input.rs    # winit event → gruid Msg translation
│           ├── renderer.rs # GridRenderer: fontdue rasterizer + pixel buffer
│           └── builtin_font.ttf # DejaVu Sans Mono
└── examples/
    ├── Cargo.toml
    ├── src/lib.rs          # Shared Game model (cave generation + FOV + movement)
    ├── roguelike.rs        # Terminal entry point (crossterm)
    └── roguelike_winit.rs  # Graphical entry point (winit+softbuffer)
```

---

## Key Architecture Decisions

### Relative Coordinate System

Both `gruid_core::Grid` and `gruid_rl::Grid` use **relative** coordinates
matching Go gruid's semantics. After `grid.slice(Range::new(5,5,10,10))`,
`grid.set(Point::new(0,0), c)` writes to position (5,5) in the underlying
buffer. `slice()` takes a relative range, clamped to the grid's size.

All public methods (`at`, `set`, `contains`, `iter`, `map_cells`, `fill`)
work with relative coordinates. Internal storage uses absolute coords in the
shared buffer.

### Two Driver Models

The core supports both poll-based and event-loop-based backends:

```rust
// Poll-based (crossterm): App calls poll_msgs() in a loop
pub trait Driver {
    fn init(&mut self) -> Result<()>;
    fn poll_msgs(&mut self, ctx: &Context, tx: Sender<Msg>) -> Result<()>;
    fn flush(&mut self, frame: Frame) -> Result<()>;
    fn close(&mut self);
}

// Event-loop-based (winit): Driver owns the main thread
pub trait EventLoopDriver {
    fn run(self, runner: AppRunner) -> Result<()>;
}
```

`AppRunner` is a state machine the event-loop driver calls into:
- `runner.init()` — sends Msg::Init
- `runner.handle_msg(msg)` — feeds input to Model::update()
- `runner.draw_frame() -> Option<Frame>` — calls Model::draw(), diffs, returns changes
- `runner.should_quit()` — checks if Effect::End was returned
- `runner.resize(w, h)` — reallocates grids on window resize

### Grid Shared Storage

Both `gruid_core::Grid` and `gruid_rl::Grid` use `Rc<RefCell<GridBuffer>>` for
slice semantics (like Go's slice-of-underlying-array). `Clone` shares the buffer.
`slice()` returns a new Grid with narrower bounds but same buffer pointer.

### FOV Algorithms

Two FOV algorithms, both matching Go gruid:

1. **VisionMap** (ray-based): Octant-parent ray propagation. Non-binary visibility
   with cost accumulation via `Lighter` trait (`cost(src, from, to)` + `max_cost(src)`).
   Supports `From`/`Ray` traceback and multi-source `LightMap`.

2. **SSCVisionMap** (symmetric shadow casting): Albert Ford's algorithm.
   Binary visibility with `diags` parameter. Multi-source `SSCLightMap`.

### JPS Pathfinding

Faithfully ported from Go gruid. Both 8-way (diagonal) and 4-way (cardinal-only)
modes work. Uses `dirnorm` for direction normalization, `expandOrigin` for initial
successors, `straightMax` for edge optimization, and `jumpPath` with cardinal
intermediates for no-diags mode.

### Frame Diffing

`compute_frame(prev, curr)` compares two same-sized grids cell-by-cell and
returns only changed cells as a `Frame { cells: Vec<FrameCell> }`. Positions
in the frame are relative (0-based). Drivers only render the diff.

### PathRange Cache Pattern

`PathRange` owns all cached data structures for pathfinding. All algorithms are
methods on `&mut PathRange`. Uses generation-based cache invalidation (increment
a counter instead of clearing O(n) arrays).

### Winit DPI Handling

The winit driver queries `monitor.scale_factor()` at startup, multiplies the
logical font size by it, and works entirely in physical pixels. The window is
created with `PhysicalSize`. Handles `ScaleFactorChanged` events by rebuilding
the renderer.

---

## Go Reference Files

The Go original is cloned at `/home/exedev/gruid/`. Key files:

| Go File | What to Reference |
|---------|-------------------|
| `grid.go` | Grid slice semantics, Set/At coordinate handling, Resize, Copy |
| `ui.go` | App loop, Effect dispatch (goroutine spawning), Driver interface |
| `messages.go` | Input event types |
| `recording.go` | gzip+gob frame encoding |
| `paths/jps.go` | JPS algorithm — already ported |
| `paths/pathrange.go` | Epoch-based cache invalidation |
| `rl/fov.go` | Both FOV algorithms — already ported |
| `rl/mapgen.go` | Vault system, KeepCC, countWalls, RandomWalkCave |
| `rl/grid.go` | Integer-cell grid with relative coordinates — already ported |
| `ui/menu.go` | Full menu with mouse, pagination, multi-column layout |
| `ui/pager.go` | Pager with all navigation modes |
| `ui/textinput.go` | Text input with prompt and cursor |
| `ui/replay.go` | Replay widget |
| `ui/styledtext.go` | @r markup system |

---

## Known Working State

- **48 tests pass** (`cargo test -p gruid-core -p gruid-paths -p gruid-rl -p gruid-ui -p gruid-crossterm`)
- **Workspace compiles clean** (`cargo check --workspace`, zero warnings)
- **Grid relative coordinates** — both gruid-core and gruid-rl match Go semantics
- **JPS 4-way and 8-way** — both work, faithfully ported from Go
- **FOV VisionMap + SSC** — both work, match Go algorithms, with From/Ray/LightMap
- **countWalls** — includes center cell, matches Go
- **RandomWalkCave** — random start positions, outDigs logic, matches Go
- **roguelike example works** — cave generation, FOV, movement in both
  crossterm and winit backends
- **Winit DPI scaling** works on Retina displays

## Known Incomplete / Missing

- **Sub effects** — silently dropped (no thread spawning)
- **Recording** — stub only
- **Vault system** — not implemented
- **KeepCC** — not implemented
- **UI widgets** — partial (keys only, no mouse/pagination/prompt)
- **Grid Resize** — not implemented
- **StyledText @r markup** — partial

---

## Build & Test Commands

```bash
# Check everything compiles
cargo check --workspace

# Run tests (skip winit — it pulls too many deps for test build)
cargo test -p gruid-core -p gruid-paths -p gruid-rl -p gruid-ui -p gruid-crossterm

# Run terminal demo
cargo run --bin roguelike

# Run graphical demo
cargo run --bin roguelike-winit

# gruid-tiles is excluded from workspace (huge image crate deps)
# Build separately: cargo check -p gruid-tiles --manifest-path crates/gruid-tiles/Cargo.toml
```

**Disk space warning:** This VM has limited disk (~600MB free). The full
workspace test build with winit can exhaust it. Use targeted test commands.

---

## Crate Dependency Graph

```
gruid-core (no deps)
    │
    ├── gruid-paths (depends on gruid-core)
    │       │
    │       └── gruid-rl (depends on gruid-core, gruid-paths, rand)
    │
    ├── gruid-ui (depends on gruid-core)
    │
    ├── gruid-crossterm (depends on gruid-core, crossterm)
    │
    ├── gruid-winit (depends on gruid-core, winit, softbuffer, fontdue)
    │
    └── gruid-tiles (depends on gruid-core, image, rusttype) [excluded from workspace]
```

---

## Style Notes

- Rust 2024 edition (1.85+). `gen` is reserved — use `cur_gen` for generation counters.
- Builder pattern: `with_*()` methods return `Self` by value (Copy types) or new owned value.
- Interior mutability: Grid uses `Rc<RefCell<>>`. `set()` and `fill()` take `&self` not `&mut self`.
- Optional serde: `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`
- Error type: `Box<dyn std::error::Error>` throughout (no custom error types yet).
- Naming: `Box` is reserved in Rust → file is `box_.rs`, type is `BoxDecor`.
- Grid coordinates: always **relative** to the view's origin. `bounds()` returns absolute range in buffer; `range_()` returns relative range (0,0 to size).
