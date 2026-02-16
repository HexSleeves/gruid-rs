# Agent Context — gruid-rs

Read this before making changes. See also `AGENTS.md` for coding standards
and the **mandatory pre-commit checklist** (fmt, clippy, test, update docs, commit, push).

---

## What This Project Is

A Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) — a Go
cross-platform grid-based UI and game framework using the Elm architecture
(Model-View-Update). Designed for roguelike games but general-purpose.

**Go original:** `https://codeberg.org/anaseto/gruid` (clone as needed for reference)
**Shamogu (reference game):** `https://codeberg.org/anaseto/shamogu` (Go roguelike using gruid)
**Rust port:** `/home/exedev/gruid-rs/` — 13,500 LOC across 8 crates + examples
**Repo:** `https://github.com/HexSleeves/gruid-rs`

---

## Current State (as of last update)

- **211 tests pass** (`cargo test --workspace --all-features`, zero failures)
- **Clippy clean** (`cargo clippy --workspace -- -D warnings`, zero warnings)
- **~94% Go API parity** — all P0 blockers and P1 items closed except Replay polish
- **3 backends:** terminal (crossterm), native window (winit), browser (gruid-web/WASM)
- **Serde:** all key types serializable behind `serde` feature flag

---

## Workspace Structure

```
gruid-rs/
├── Cargo.toml              # Workspace root
├── README.md               # Project overview + quick start
├── AGENTS.md               # Agent coding standards + pre-commit checklist
├── CONTEXT.md              # This file — architecture context
├── TODO.md                 # Prioritized task list
├── GAP_ANALYSIS.md         # Original Go→Rust gap audit (31 items)
├── crates/
│   ├── gruid-core/         # Core types (in workspace)
│   │   └── src/
│   │       ├── lib.rs      # Re-exports: Grid, Cell, Point, Range, Style, Msg, etc.
│   │       ├── geom.rs     # Point (i32 x,y), Range (half-open rect) — RELATIVE coords
│   │       │               # Range: add/sub, line/lines/column/columns (relative),
│   │       │               # rel_msg, in_range, shift (empty-safe), union, intersect
│   │       │               # PartialEq normalizes empties. Add<Point>/Sub<Point> impls.
│   │       ├── style.rs    # Color (u32 RGB), AttrMask (bitflags), Style (fg/bg/attrs)
│   │       ├── cell.rs     # Cell { ch: char, style: Style }
│   │       ├── grid.rs     # Grid with Rc<RefCell<GridBuffer>> shared storage
│   │       │               # Slice semantics, relative coords, resize, Display impl
│   │       ├── messages.rs # Key enum, ModMask, MouseAction, Msg enum
│   │       │               # Msg::Init/KeyDown/Mouse/Screen/Quit/Tick/Custom
│   │       ├── app.rs      # Model/Driver/EventLoopDriver traits, App, AppRunner, Effect
│   │       │               # Effect::Cmd/Sub/Batch/End, Context (cancellation token)
│   │       └── recording.rs # FrameEncoder/FrameDecoder — binary frame serialization
│   │
│   ├── gruid-paths/        # Pathfinding algorithms (in workspace)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs   # Pather, WeightedPather, AstarPather trait hierarchy
│   │       ├── neighbors.rs # Neighbors: cardinal (4), all (8), diagonal (4)
│   │       ├── distance.rs # manhattan(), chebyshev()
│   │       ├── pathrange.rs # PathRange (cache owner) + serde support
│   │       ├── astar.rs    # PathRange::astar_path()
│   │       ├── dijkstra.rs # PathRange::dijkstra_map/at()
│   │       ├── bfs.rs      # PathRange::bfs_map/at()
│   │       ├── jps.rs      # PathRange::jps_path() — 8-way AND 4-way
│   │       └── cc.rs       # PathRange::cc_map_all/cc_map/cc_at()
│   │
│   ├── gruid-rl/           # Roguelike utilities (in workspace)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grid.rs     # rl::Grid (Cell=i32) — relative coords, for_each_mut,
│   │       │               # map_cells_mut, at_unchecked, resize, serde support
│   │       ├── fov.rs      # FOV: VisionMap + SSC, LightMap, From/Ray, serde
│   │       ├── mapgen.rs   # MapGen: cellular automata + random walk + KeepCC
│   │       ├── vault.rs    # Vault: ASCII art room prefabs with transforms
│   │       └── events.rs   # EventQueue<E>: priority queue with serde
│   │
│   ├── gruid-ui/           # UI widgets (in workspace)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── styled_text.rs # StyledText: @-prefix markup (@X switch, @N reset,
│   │       │                  # @@ escape), format, lines (cross-line state)
│   │       ├── box_.rs     # BoxDecor: Unicode box drawing, markup-aware title/footer
│   │       ├── label.rs    # Label: background fill, AdjustWidth
│   │       ├── menu.rs     # Menu: 2D grid layout, active_invokable, mouse,
│   │       │               # pagination with page numbers, disabled skip
│   │       ├── pager.rs    # Pager: v/h scroll (8-col step), half-page, top/bottom,
│   │       │               # mouse click page up/down, lines(), set_cursor(Point),
│   │       │               # PagerKeys::start, view()->Range
│   │       ├── replay.rs   # Replay: auto-play, speed, pause, seek, undo
│   │       │               # MISSING: help overlay, mouse interaction, grid auto-resize
│   │       └── text_input.rs # TextInput: prompt, cursor, mouse click-to-position
│   │
│   ├── gruid-crossterm/    # Terminal driver (in workspace)
│   │   └── src/lib.rs      # CrosstermDriver implements Driver trait (poll-based)
│   │
│   ├── gruid-winit/        # Native window driver (in workspace)
│   │   └── src/
│   │       ├── lib.rs      # WinitDriver implements EventLoopDriver
│   │       ├── input.rs    # winit event → gruid Msg translation
│   │       └── renderer.rs # GridRenderer: fontdue rasterizer + pixel buffer
│   │
│   ├── gruid-web/          # WASM browser driver (EXCLUDED — wasm32 only)
│   │   └── src/lib.rs      # WebDriver implements EventLoopDriver
│   │                       # Canvas 2D text rendering, keyboard/mouse events
│   │
│   ├── gruid-tiles/        # Font-to-tile rendering (EXCLUDED — heavy deps)
│   │   └── src/            # Drawer using rusttype + image crate
│   │
│   └── gruid-sdl/          # SDL driver (EMPTY — placeholder)
│
└── examples/
    ├── Cargo.toml
    ├── src/lib.rs          # Shared Game model — roguelike demo
    ├── roguelike.rs        # Terminal entry point (crossterm)
    └── roguelike_winit.rs  # Graphical entry point (winit)
```

---

## Key Architecture Decisions

### Coordinate System — RELATIVE

Both `gruid_core::Grid` and `gruid_rl::Grid` use **relative** coordinates.
After `grid.slice(Range::new(5,5,10,10))`, `grid.set(Point::new(0,0), c)` writes
to position (5,5) in the underlying buffer. All public methods work with relative
coordinates. Internal storage uses absolute coords in the shared buffer.

### Two Driver Models

```rust
// Poll-based (crossterm): App calls poll_msgs() in a loop
pub trait Driver {
    fn init(&mut self) -> Result<()>;
    fn poll_msgs(&mut self, ctx: &Context, tx: Sender<Msg>) -> Result<()>;
    fn flush(&mut self, frame: Frame) -> Result<()>;
    fn close(&mut self);
}

// Event-loop-based (winit, web): Driver owns the main thread
pub trait EventLoopDriver {
    fn run(self, runner: AppRunner) -> Result<()>;
}
```

`AppRunner` is the state machine for event-loop drivers:
- `runner.init()` → sends Msg::Init
- `runner.handle_msg(msg)` → feeds input to Model::update()
- `runner.process_pending_msgs()` → drains background Cmd/Sub messages
- `runner.draw_frame() -> Option<Frame>` → Model::draw(), diff, returns changes
- `runner.should_quit()` → checks Effect::End
- `runner.resize(w, h)` → reallocates grids

### Effect System

- `Effect::Cmd(f)` — spawns thread, runs `f()`, sends result msg (NOT in WASM)
- `Effect::Sub(f)` — long-running subscription thread (NOT in WASM)
- `Effect::Batch(vec)` — multiple effects
- `Effect::End` — signals quit

### Grid Shared Storage

Both Grid types use `Rc<RefCell<GridBuffer>>` for slice semantics (like Go's
slice-of-underlying-array). `Clone` shares the buffer. `slice()` returns a
new Grid with narrower bounds but same buffer pointer.

### StyledText `@`-Prefix Markup

- `@X` (X is a markup key) → switch to that style
- `@N` → reset to base style
- `@@` → literal `@` character
- `@` + unknown char → consumed, no output
- `\r` → stripped
- `lines()` preserves markup state across line breaks

### FOV Algorithms

1. **VisionMap** (ray-based): Octant-parent ray propagation with cost accumulation
   via `Lighter` trait. Supports `From`/`Ray` traceback and multi-source `LightMap`.
2. **SSCVisionMap** (symmetric shadow casting): Albert Ford's algorithm.
   Binary visibility with `diags` parameter. Multi-source `SSCLightMap`.

### PathRange Cache Pattern

`PathRange` owns all cached data structures. All algorithms are methods on
`&mut PathRange`. Uses generation-based cache invalidation (increment counter
instead of clearing O(n) arrays).

### Frame Diffing

`compute_frame(prev, curr)` compares two grids cell-by-cell, returns only
changed cells as `Frame { cells, width, height, time_ms }`. Drivers only
render the diff.

---

## Go Reference Porting Status

| Go File | Rust Status |
|---------|-------------|
| `grid.go` | ✅ Fully ported (Resize, Display, relative coords) |
| `ui.go` | ✅ Ported (App, Effects, Driver traits) |
| `messages.go` | ✅ Ported |
| `recording.go` | ✅ Ported (binary format) |
| `paths/pathrange.go` | ✅ Ported + serde |
| `paths/jps.go` | ✅ Ported (8-way + 4-way) |
| `paths/neighbors.go` | ✅ Ported (cardinal + all + diagonal) |
| `rl/fov.go` | ✅ Ported (from() bug fixed) + serde |
| `rl/mapgen.go` | ✅ Ported (incl. Vault, KeepCC, with_grid) |
| `rl/grid.go` | ✅ Ported (for_each_mut, resize, at_unchecked) + serde |
| `rl/events.go` | ✅ Ported + serde |
| `ui/styledtext.go` | ✅ Ported (@-prefix markup) |
| `ui/menu.go` | ✅ Ported (2D layout, active_invokable, mouse) |
| `ui/pager.go` | ✅ Ported (all features) |
| `ui/textinput.go` | ✅ Ported |
| `ui/label.go` | ✅ Ported (bg fill, AdjustWidth) |
| `ui/box.go` | ✅ Ported (markup-aware title/footer) |
| `ui/replay.go` | ⚠️ Partial — missing help overlay, mouse, grid auto-resize |

---

## What's Still Missing (3 items)

### Functional Gaps
1. **Replay widget** — help overlay (embedded Pager), mouse interaction, grid auto-resize
2. **Pager line number in footer** — Go shows "Line X/Y" in box footer

### Performance Gaps
3. **PathRange SetRange capacity** — Go preserves caches when new size ≤ old capacity
4. **JPS buffer reuse** — Go accepts pre-allocated `path []Point`

### Testing Gap
Go has 3,124 lines of tests across 14 files. Rust has 211 tests.
Biggest untested areas: Grid slice edge cases, StyledText format edge cases.

---

## Build & Test Commands

```bash
# Full check cycle (the pre-commit checklist)
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace                    # 195 tests (without serde)
cargo test --workspace --all-features     # 204 tests (with serde)

# Run demos
cargo run --bin roguelike                 # terminal
cargo run --bin roguelike-winit           # native window

# WASM driver (excluded from workspace)
cargo check -p gruid-web --target wasm32-unknown-unknown \
  --manifest-path crates/gruid-web/Cargo.toml

# Tiles (excluded from workspace)
cargo check -p gruid-tiles --manifest-path crates/gruid-tiles/Cargo.toml
```

---

## Crate Dependency Graph

```
gruid-core (no deps)
    │
    ├── gruid-paths (gruid-core)
    │       │
    │       └── gruid-rl (gruid-core, gruid-paths, rand)
    │
    ├── gruid-ui (gruid-core)
    │
    ├── gruid-crossterm (gruid-core, crossterm)
    │
    ├── gruid-winit (gruid-core, winit, softbuffer, fontdue)
    │
    ├── gruid-web (gruid-core, wasm-bindgen, web-sys) [excluded, wasm32 only]
    │
    └── gruid-tiles (gruid-core, image, rusttype) [excluded]
```

Optional feature: `serde` on gruid-core, gruid-paths, gruid-rl — enables
Serialize/Deserialize on Point, Range, Cell, Style, PathNode, PathRange,
EventQueue, rl::Grid, FOV.

---

## Style Notes

- **Rust 2024 edition** (1.85+). `gen` is reserved — use `cur_gen`.
- **Builder pattern:** `with_*()` methods return `Self` by value.
- **Interior mutability:** Grid uses `Rc<RefCell<>>`. `set()`/`fill()` take `&self`.
- **Serde:** `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]`
- **Error type:** `Box<dyn std::error::Error>` throughout.
- **Naming:** `Box` is reserved → file is `box_.rs`, type is `BoxDecor`.
- **Coordinates:** always relative to the view's origin.

---

## Next Step: Port shamogu

The framework is ready to port [shamogu](https://codeberg.org/anaseto/shamogu)
— a Go roguelike game built on gruid. Key shamogu files:

| Go File | What it does |
|---------|-------------|
| `model.go` | Model struct, init, key bindings |
| `update.go` | `Update(msg)` — mode-based dispatch + Action system |
| `draw.go` | `Draw()` — renders map, log, status bar |
| `actions.go` | Action interface + all action handlers |
| `game.go` | Core game state |
| `map.go` / `mapgen.go` | Map generation |
| `monsters.go` / `actor.go` | Entity system |
| `fov.go` | FOV integration |
| `paths.go` | Pathfinding integration |
| `combat.go` | Combat system |
| `animation.go` | Animation queue using Cmd timers |
| `effects.go` | Status effects (berserk, poison, etc.) |
| `items.go` | Item/consumable system |
| `menu.go` / `pager.go` | UI widget usage |
