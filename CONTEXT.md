# Agent Context Dump — gruid-rs

This file contains full context for continuing development of gruid-rs.
Read this before making changes.

---

## What This Project Is

A Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) — a Go
cross-platform grid-based UI and game framework using the Elm architecture
(Model-View-Update). Designed for roguelike games but general-purpose.

**Go original:** cloned as needed from `https://codeberg.org/anaseto/gruid`
**Rust port:** `/home/exedev/gruid-rs/` (~13,400 LOC across 8 crates + examples)
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
│   │       ├── messages.rs # Key, ModMask, MouseAction, Msg enum (incl. Msg::Tick)
│   │       ├── app.rs      # Model/Driver/EventLoopDriver traits, App, AppRunner, Effect
│   │       └── recording.rs # FrameEncoder/FrameDecoder — binary frame serialization
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
│   │       ├── mapgen.rs   # MapGen — cellular automata + random walk + KeepCC
│   │       ├── vault.rs    # Vault — ASCII art room prefabs with transforms
│   │       └── events.rs   # EventQueue<E> — works
│   ├── gruid-ui/           # UI widgets
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── styled_text.rs # StyledText with markup — partial (@r incomplete)
│   │       ├── box_.rs     # BoxDecor (Unicode box drawing) — works
│   │       ├── label.rs    # Label — partial (no auto-sizing)
│   │       ├── menu.rs     # Menu — keyboard, mouse, pagination, disabled skip
│   │       ├── pager.rs    # Pager — vertical/horizontal scroll, half-page, mouse wheel
│   │       ├── replay.rs   # Replay — frame playback with speed/pause/seek/undo
│   │       └── text_input.rs # TextInput — prompt, cursor, mouse click-to-position
│   ├── gruid-tiles/        # Font-to-tile rendering (EXCLUDED from workspace build)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── drawer.rs   # Drawer using rusttype + image crate
│   ├── gruid-crossterm/    # Terminal driver — works
│   │   └── src/lib.rs      # CrosstermDriver implements Driver trait
│   └── gruid-winit/        # Native window driver — works
│       └── src/
│           ├── lib.rs      # WinitDriver implements EventLoopDriver trait
│           ├── input.rs    # winit event → gruid Msg translation
│           ├── renderer.rs # GridRenderer: fontdue rasterizer + pixel buffer
│           └── builtin_font.ttf # DejaVu Sans Mono
└── examples/
    ├── Cargo.toml
    ├── src/lib.rs          # Shared Game model — full-featured roguelike demo
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
- `runner.process_pending_msgs()` — drains background Cmd/Sub messages
- `runner.draw_frame() -> Option<Frame>` — calls Model::draw(), diffs, returns changes
- `runner.should_quit()` — checks if Effect::End was returned
- `runner.resize(w, h)` — reallocates grids on window resize

### Effect System

- `Effect::Cmd(f)` — spawns a thread, runs `f()`, sends result msg back
- `Effect::Sub(f)` — spawns a thread, runs `f(ctx, tx)` for long-running subscriptions
- `Effect::Batch(vec)` — processes multiple effects
- `Effect::End` — signals quit

Both `App` (poll-based) and `AppRunner` (event-loop) spawn real threads
for Cmd/Sub effects and feed messages back via channels.

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

### Frame Diffing & Recording

`compute_frame(prev, curr)` compares two same-sized grids cell-by-cell and
returns only changed cells as a `Frame { cells, width, height, time_ms }`.
Positions in the frame are relative (0-based). Drivers only render the diff.

`FrameEncoder`/`FrameDecoder` serialize frames to a compact binary wire format
(length-prefixed, no external deps). The `time_ms` field supports replay timing.

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

| Go File | Status |
|---------|--------|
| `grid.go` | ✅ Ported (except Resize) |
| `ui.go` | ✅ Ported (App, Effects, Driver traits) |
| `messages.go` | ✅ Ported |
| `recording.go` | ✅ Ported (binary format, not gob+gzip) |
| `paths/jps.go` | ✅ Ported |
| `paths/pathrange.go` | ✅ Ported |
| `rl/fov.go` | ✅ Ported |
| `rl/mapgen.go` | ✅ Ported (incl. Vault, KeepCC) |
| `rl/grid.go` | ✅ Ported |
| `ui/menu.go` | ✅ Ported (except multi-column layout) |
| `ui/pager.go` | ✅ Ported |
| `ui/textinput.go` | ✅ Ported |
| `ui/replay.go` | ✅ Ported |
| `ui/styledtext.go` | ⚠️ Partial (@r markup incomplete) |

---

## Known Working State

- **195 tests pass** (`cargo test --workspace`, zero failures)
- **Workspace compiles clean** (`cargo clippy --workspace -- -D warnings`, zero warnings)
- **Code formatted** (`cargo fmt --all`)
- **Grid relative coordinates** — both gruid-core and gruid-rl match Go semantics
- **Grid Resize** — both gruid-core and gruid-rl, content-preserving
- **Grid Display** — `impl Display for Grid` for debugging
- **Range** — relative Line/Lines/Column/Columns, Add/Sub translation, In containment,
  RelMsg for mouse position adjustment, empty normalization in PartialEq
- **JPS 4-way and 8-way** — both work, faithfully ported from Go
- **FOV VisionMap + SSC** — both work, match Go algorithms, with From/Ray/LightMap
- **FOV from()** — bug fixed (no longer double-counts lt.cost())
- **Vault system** — parse, draw, reflect, rotate — matches Go
- **KeepCC** — uses PathRange CC labels to keep largest connected component
- **Neighbors** — all (8-way), cardinal (4-way), diagonal (4-way)
- **Sub effects** — Cmd/Sub spawn background threads, messages fed back via channel
- **Frame recording** — real binary encoder/decoder with time_ms for replay
- **Replay widget** — auto-play, speed control, pause, stepping, seeking, undo
- **Menu widget** — 2D grid layout, keyboard, mouse (click/wheel/hover/outside-quit),
  pagination with page numbers, disabled-entry skip, active_invokable
- **Pager widget** — vertical/horizontal scroll (8-col step), half-page, top/bottom,
  mouse wheel + click page up/down, lines(), set_cursor(Point), view()->Range, start key
- **TextInput widget** — prompt, cursor, mouse click-to-position
- **StyledText** — full `@`-prefix markup protocol: `@X` switch, `@N` reset, `@@` escape,
  `\r` stripping, cross-line state preservation, format with zero-width markup
- **Label** — background fill, AdjustWidth functional
- **BoxDecor** — markup-aware title/footer rendering
- **rl::Grid** — for_each_mut, map_cells_mut, at_unchecked, resize, copy_from returns Point
- **Roguelike demo** — cave gen, FOV, monsters with A* AI, combat, Dijkstra heatmap,
  A* path overlay, look mode, mouse click-to-move, status bar, message log, help pager
- **Winit DPI scaling** works on Retina displays

## Known Incomplete / Missing

- **Serde derives** — only on core types, not on FOV/PathRange/EventQueue/rl::Grid
- **Replay** — missing help overlay, mouse interaction, grid auto-resize
- **StyledText lines()** — markup state preservation needs edge case verification
- **P2 minor methods** — see TODO.md (Key helpers, PathRange capacity opt, etc.)

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

# gruid-web is excluded (requires wasm32 target)
# Build: cargo check -p gruid-web --target wasm32-unknown-unknown --manifest-path crates/gruid-web/Cargo.toml
```

**Disk space warning:** This VM has limited disk. The full workspace test build
with winit can exhaust it. Use targeted test commands.

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
    ├── gruid-tiles (depends on gruid-core, image, rusttype) [excluded from workspace]
    │
    └── gruid-web (depends on gruid-core, wasm-bindgen, web-sys) [excluded, wasm32 only]
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
