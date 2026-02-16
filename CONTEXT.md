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
**Rust port:** `/home/exedev/gruid-rs/` — 14,225 LOC across 8 crates + examples
**Repo:** `https://github.com/HexSleeves/gruid-rs`
**HEAD:** `9865c19` on `main`

---

## Current State (as of last update)

- **228 tests pass** (`cargo test --workspace --all-features`, zero failures)
- **Clippy clean** (`cargo clippy --workspace -- -D warnings`, zero warnings)
- **~99% Go API parity** — all P0, P1, and P2 items closed
- **3 backends:** terminal (crossterm), native window (winit), browser (gruid-web/WASM)
- **Serde:** all key types serializable behind `serde` feature flag
- **All Go gruid packages fully ported** — grid, messages, app, recording, paths, rl, ui

---

## Workspace Structure

```
gruid-rs/
├── Cargo.toml              # Workspace root (Rust 2024 edition, resolver 2)
├── README.md               # Project overview + quick start
├── AGENTS.md               # Agent coding standards + pre-commit checklist
├── CONTEXT.md              # This file — architecture context
├── TODO.md                 # Prioritized task list
├── GAP_ANALYSIS.md         # Original Go→Rust gap audit (31 items, all closed)
├── crates/
│   ├── gruid-core/         # 2,761 LOC — Core types (in workspace)
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
│   │       ├── messages.rs # Key enum, ModMask ("Ctrl+Shift" Display), MouseAction, Msg enum
│   │       │               # Msg::Init/KeyDown/Mouse/Screen/Quit/Custom(Arc<dyn Any>)
│   │       ├── app.rs      # Model/Driver/EventLoopDriver traits, App, AppRunner, Effect
│   │       │               # Effect::Cmd/Sub/Batch/End, Context (cancellation token)
│   │       └── recording.rs # FrameEncoder/FrameDecoder — binary frame serialization
│   │
│   ├── gruid-paths/        # 1,641 LOC — Pathfinding algorithms (in workspace)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs   # Pather, WeightedPather, AstarPather trait hierarchy
│   │       ├── neighbors.rs # Neighbors: cardinal (4), all (8), diagonal (4)
│   │       ├── distance.rs # manhattan(), chebyshev()
│   │       ├── pathrange.rs # PathRange (cache owner) + serde + capacity-preserving set_range
│   │       ├── astar.rs    # PathRange::astar_path()
│   │       ├── dijkstra.rs # PathRange::dijkstra_map/at()
│   │       ├── bfs.rs      # PathRange::bfs_map/at()
│   │       ├── jps.rs      # PathRange::jps_path() + jps_path_into() — 8-way AND 4-way
│   │       └── cc.rs       # PathRange::cc_map_all/cc_map/cc_at()
│   │
│   ├── gruid-rl/           # 2,920 LOC — Roguelike utilities (in workspace)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── grid.rs     # rl::Grid (Cell=i32) — relative coords, for_each_mut,
│   │       │               # map_cells_mut, at_unchecked, resize, copy_from, serde
│   │       ├── fov.rs      # FOV: VisionMap (ray-based) + SSC (symmetric shadow casting)
│   │       │               # LightMap, SSCLightMap, From/Ray, CircularLighter, serde
│   │       ├── mapgen.rs   # MapGen: cellular automata + random walk + KeepCC
│   │       ├── vault.rs    # Vault: ASCII art room prefabs with reflect/rotate
│   │       └── events.rs   # EventQueue<E>: priority queue with serde
│   │
│   ├── gruid-ui/           # 3,420 LOC — UI widgets (in workspace)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── styled_text.rs # StyledText: @-prefix markup, format, lines, with/with_textf
│   │       ├── box_.rs     # BoxDecor: Unicode box drawing, markup-aware title/footer
│   │       ├── label.rs    # Label: background fill, AdjustWidth
│   │       ├── menu.rs     # Menu: 2D grid layout, active_invokable, mouse, page numbers
│   │       ├── pager.rs    # Pager: v/h scroll, mouse, line-number footer, view()
│   │       ├── replay.rs   # Replay: auto-play, speed, pause, seek, undo, help overlay,
│   │       │               # mouse interaction, grid auto-resize
│   │       └── text_input.rs # TextInput: prompt, cursor auto-reverse, mouse click
│   │
│   ├── gruid-crossterm/    # 261 LOC — Terminal driver (in workspace)
│   │   └── src/lib.rs      # CrosstermDriver implements Driver trait (poll-based)
│   │
│   ├── gruid-winit/        # 752 LOC — Native window driver (in workspace)
│   │   └── src/
│   │       ├── lib.rs      # WinitDriver implements EventLoopDriver
│   │       ├── input.rs    # winit event → gruid Msg translation
│   │       └── renderer.rs # GridRenderer: fontdue rasterizer + pixel buffer
│   │
│   ├── gruid-web/          # 539 LOC — WASM browser driver (EXCLUDED — wasm32 only)
│   │   └── src/lib.rs      # WebDriver implements EventLoopDriver
│   │                       # Canvas 2D text rendering, keyboard/mouse events
│   │
│   ├── gruid-tiles/        # 107 LOC — Font-to-tile rendering (EXCLUDED — heavy deps)
│   │   └── src/            # Drawer using rusttype + image crate
│   │
│   └── gruid-sdl/          # EMPTY — placeholder
│
└── examples/               # 935 LOC
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

### Model Trait

```rust
pub trait Model {
    fn update(&mut self, msg: Msg) -> Option<Effect>;
    fn draw(&self, grid: &mut Grid); // &self — immutable
}
```

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
3. **CircularLighter**: Wrapper that enforces circular FOV shape.

### PathRange Cache Pattern

`PathRange` owns all cached data structures. All algorithms are methods on
`&mut PathRange`. Uses generation-based cache invalidation (increment counter
instead of clearing O(n) arrays). `set_range` preserves caches when new size
≤ old capacity.

### Frame Diffing

`compute_frame(prev, curr)` compares two grids cell-by-cell, returns only
changed cells as `Frame { cells, width, height, time_ms }`. Drivers only
render the diff.

---

## Complete Go Reference Porting Status

| Go File | Rust Status |
|---------|-------------|
| `grid.go` | ✅ Fully ported (Resize, Display, relative coords) |
| `ui.go` | ✅ Ported (App, Effects, Driver traits) |
| `messages.go` | ✅ Ported (ModMask combos, Custom msgs) |
| `recording.go` | ✅ Ported (binary format) |
| `paths/pathrange.go` | ✅ Ported + serde + capacity-preserving SetRange |
| `paths/jps.go` | ✅ Ported (8-way + 4-way, buffer reuse via jps_path_into) |
| `paths/neighbors.go` | ✅ Ported (cardinal + all + diagonal) |
| `rl/fov.go` | ✅ Ported (from() bug fixed, circular FOV) + serde |
| `rl/mapgen.go` | ✅ Ported (incl. Vault, KeepCC, with_grid) |
| `rl/grid.go` | ✅ Ported (for_each_mut, resize, at_unchecked) + serde |
| `rl/events.go` | ✅ Ported + serde |
| `ui/styledtext.go` | ✅ Ported (@-markup, with/with_textf) |
| `ui/menu.go` | ✅ Ported (2D layout, active_invokable, mouse) |
| `ui/pager.go` | ✅ Ported (line-number footer, all features) |
| `ui/textinput.go` | ✅ Ported (cursor auto-reverse) |
| `ui/label.go` | ✅ Ported (bg fill, AdjustWidth) |
| `ui/box.go` | ✅ Ported (markup-aware title/footer) |
| `ui/replay.go` | ✅ Fully ported (help overlay, mouse, grid auto-resize) |

---

## What's Still Missing

All functional and performance gaps are now closed.

### Testing Gap
Go has 3,124 lines of tests across 14 files. Rust has 228 tests.
Biggest untested areas: Grid slice edge cases, StyledText format edge cases.

---

## Build & Test Commands

```bash
# Full check cycle (the pre-commit checklist)
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace                    # ~220 tests (without serde)
cargo test --workspace --all-features     # 228 tests (with serde)

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

## Public API Quick Reference

### gruid-core
```rust
// Model trait
trait Model {
    fn update(&mut self, msg: Msg) -> Option<Effect>;
    fn draw(&self, grid: &mut Grid);
}

// Point / Range
Point::new(x, y), Point::ZERO, p.shift(dx, dy), p.neighbors_4(), p.neighbors_8()
Range::new(x0,y0,x1,y1), r.size(), r.line(y), r.lines(y0,y1), r.column(x),
r.columns(x0,x1), r.shift(dx0,dy0,dx1,dy1), r.contains(p), r.in_range(r2),
r.intersect(r2), r.union(r2), r.rel_msg(msg), r.iter()
Range + Point, Range - Point (via Add/Sub traits)

// Grid
Grid::new(w, h), g.slice(range), g.at(p), g.set(p, cell), g.fill(cell),
g.map_cells(f), g.copy_from(src), g.resize(w,h), g.contains(p),
g.bounds(), g.range_(), g.size(), g.width(), g.height(), g.iter(), g.points()
compute_frame(prev, curr) -> Frame

// Style / Cell
Color::default(), Color::from_rgb(r,g,b)
Style { fg, bg, attrs }, s.with_fg(c).with_bg(c).with_attrs(a)
AttrMask::BOLD | AttrMask::ITALIC | ...
Cell { ch, style }, c.with_char(ch).with_style(s)

// Messages
Msg::Init, Msg::Quit, Msg::KeyDown{key,modifiers,time}, Msg::Mouse{action,pos,modifiers,time},
Msg::Screen{width,height,time}, Msg::Custom(Arc<dyn Any>)
Msg::key(Key), Msg::key_mod(Key, ModMask), Msg::custom(val)
Key::ArrowUp/Down/Left/Right, Key::Escape, Key::Enter, Key::Char(c), ...
ModMask::NONE/SHIFT/CTRL/ALT/META, m.contains(other), m | m2
MouseAction::Main/Auxiliary/Secondary/WheelUp/WheelDown/Release/Move

// Effects
Effect::Cmd(Box<dyn FnOnce() -> Option<Msg>>)
Effect::Sub(Box<dyn FnOnce(Sender<Msg>)>)
Effect::Batch(Vec<Effect>), Effect::End

// App (poll-based)
App::new(AppConfig { model, driver, width, height, frame_writer }).run()

// AppRunner (event-loop-based, used by winit/web drivers)
AppRunner::new(model, w, h)
runner.init(), runner.handle_msg(msg), runner.draw_frame(), runner.should_quit(),
runner.resize(w,h), runner.process_pending_msgs()
```

### gruid-paths
```rust
PathRange::new(range), pr.set_range(range), pr.range()
pr.astar_path(pather, from, to) -> Option<Vec<Point>>
pr.dijkstra_map(pather, sources, max_cost), pr.dijkstra_at(p) -> i32
pr.bfs_map(pather, sources, max_cost), pr.bfs_at(p) -> i32
pr.jps_path(from, to, passable_fn, diags) -> Option<Vec<Point>>
pr.jps_path_into(buf, from, to, passable_fn, diags) -> bool
pr.cc_map_all(pather), pr.cc_map(pather, p), pr.cc_at(p)
trait Pather { fn neighbors(&self, p) -> Vec<Point> }
trait WeightedPather: Pather { fn cost(&self, from, to) -> i32 }
trait AstarPather: WeightedPather { fn estimation(&self, from, to) -> i32 }
Neighbors::cardinal(p, passable) / Neighbors::all(p, passable) / Neighbors::diagonal(p, passable)
manhattan(a,b), chebyshev(a,b)
UNREACHABLE = i32::MAX
```

### gruid-rl
```rust
// FOV
FOV::new(range), fov.set_range(range)
fov.vision_map(lighter, src) -> &[LightNode]
fov.ssc_vision_map(lighter, src, diags) -> &[LightNode]
fov.light_map(lighter, sources), fov.ssc_light_map(lighter, sources, diags)
fov.at(p), fov.from(lighter, to), fov.ray(lighter, to)
fov.retain_circular(center, radius)
trait Lighter { fn cost(&self, from, to) -> i32; fn light(&self, from, to) -> i32 }
CircularLighter::new(inner)

// rl::Grid (Cell = i32)
Grid::new(w,h), g.slice(range), g.at(p) -> Option<Cell>, g.set(p, cell),
g.fill(cell), g.for_each_mut(f), g.map_cells_mut(f), g.at_unchecked(p),
g.resize(w,h), g.copy_from(other), g.count(cell), g.count_fn(f)

// MapGen
MapGen::with_grid(grid, rng)
mg.random_walk_cave(params), mg.cellular_automata_cave(params)
mg.keep_connected(pr, start, wall_cell)

// Vault
Vault::new(ascii_str), v.draw(grid, cell_fn), v.iter(f), v.reflect(), v.rotate(n)

// EventQueue<E>
EventQueue::new(), eq.push(event, rank), eq.push_first(event, rank),
eq.pop(), eq.pop_with_rank(), eq.filter(predicate), eq.len(), eq.is_empty()
```

### gruid-ui
```rust
// StyledText
StyledText::text(s), StyledText::textf(s), StyledText::new(text, style)
stt.with_text(s), stt.with_textf(s), stt.with(text, style), stt.with_style(s)
stt.with_markup(marker, style), stt.with_markups(map)
stt.content(), stt.style(), stt.size(), stt.iter(callback), stt.format(width)
stt.lines(), stt.draw(grid)

// Menu
Menu::new(MenuConfig { grid, entries, keys, box_, style, columns })
menu.update(msg), menu.draw(grid), menu.action(), menu.active(),
menu.set_active(i), menu.active_invokable(), menu.set_active_invokable(i)

// Pager
Pager::new(PagerConfig { content, grid, keys, box_, style })
pager.update(msg), pager.draw(grid), pager.action(), pager.lines(),
pager.set_lines(lines), pager.set_cursor(Point), pager.view(), pager.set_box(b)

// TextInput
TextInput::new(TextInputConfig { grid, content, prompt, keys, box_, style })
ti.update(msg), ti.draw(grid), ti.action(), ti.content(), ti.set_content(s)

// Label
Label { content: StyledText, adjust_width: AdjustWidth, grid }
label.draw(grid)

// BoxDecor
BoxDecor::new(), bd.title, bd.footer, bd.draw(grid) -> inner_range

// Replay
Replay::new(ReplayConfig { grid, decoder, keys })
replay.update(msg), replay.draw(grid), replay.set_frame(n), replay.seek_ms(delta),
replay.frame_index(), replay.is_auto_play(), replay.speed(), replay.is_help()
```

### Drivers
```rust
// Terminal (poll-based)
CrosstermDriver::new() // implements Driver

// Native window (event-loop-based)
WinitDriver::new(tile_size, font_bytes) // implements EventLoopDriver

// Browser WASM (event-loop-based, excluded from workspace)
WebDriver::new(canvas_id, font_family, font_size) // implements EventLoopDriver
```

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

## Disk Space

`/dev/root` is 19G, ~96% used. Run `cargo clean` after builds to free ~1.5G.
The `.cargo/registry` takes ~400M. Delete Go toolchain at `/home/exedev/go` if still present.

---

## Next Step: Port shamogu

See `SHAMOGU_PORT_PROMPT.md` for a detailed handoff prompt.
