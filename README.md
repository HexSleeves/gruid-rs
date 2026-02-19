# gruid-rs

A Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) — cross-platform grid-based UI & game framework using the Elm Architecture.

| | |
|---|---|
| **Workspace** | 9 crates + examples + shamogu game |
| **LOC** | ~18,700 Rust |
| **Tests** | 219 passing |
| **Edition** | Rust 2024 (1.85+) |
| **License** | MIT OR Apache-2.0 |

> 📐 **[Interactive Architecture Doc](docs/architecture.html)**

---

## Crates

```
gruid-core ─────┬── gruid-paths ──── gruid-rl
                ├── gruid-ui
                ├── gruid-crossterm
                ├── gruid-winit
                └── gruid-wgpu
```

| Crate | LOC | Description |
|-------|-----|-------------|
| **gruid-core** | 2,787 | Core types: `Grid`, `Cell`, `Point`, `Range`, `Style`, `Msg`, `Model`/`Driver`/`EventLoopDriver` traits, `App`/`AppRunner`, `TileManager`, frame recording |
| **gruid-paths** | 1,755 | A\*, Dijkstra, BFS, Jump Point Search (4+8 way), Connected Components |
| **gruid-rl** | 2,919 | FOV (ray-based & symmetric shadow casting), map generation, vaults, event queue |
| **gruid-ui** | 4,195 | Menu, Pager, TextInput, Label, StyledText (`@r`/`@g`/`@b` markup), BoxDecor, Replay |
| **gruid-crossterm** | 261 | Terminal backend — poll-based `Driver` |
| **gruid-winit** | 862 | Graphical backend — event-loop `EventLoopDriver` (softbuffer + fontdue) |
| **gruid-wgpu** | 1,386 | GPU-accelerated backend — event-loop `EventLoopDriver` (wgpu instanced quads + glyph atlas) |
| **gruid-web** | 539 | Browser WASM backend (excluded from workspace, wasm32-only) |
| **gruid-tiles** | — | Font-to-tile rendering (excluded) |

---

## Architecture

gruid-rs implements the **Elm Architecture** for grid UIs:

```
Msg → Model::update() → Effect → Model::draw() → Grid → diff → Frame → Driver::flush()
```

Your application defines a `Model` that receives messages, returns effects, and draws to a shared `Grid`. The framework diffs the grid and only sends changed cells to the driver.

### Driver Models

| Pattern | Trait | Backend | How it works |
|---------|-------|---------|-------------|
| **Poll-based** | `Driver` | crossterm | App owns the loop, calls `poll_msgs()` |
| **Event-loop** | `EventLoopDriver` | winit, wgpu, web | Driver owns the main thread, pushes events into `AppRunner` |

Both use the same `Model` trait — game logic works unchanged across all backends.

### Grid System

`Grid` uses `Rc<RefCell<GridBuffer>>` for Go-like slice semantics:

```rust
let grid = Grid::new(80, 24);
let sub = grid.slice(range);          // Same buffer, offset view
sub.set(Point::new(0, 0), cell);      // Writes at (range.min.x, range.min.y)
```

All public methods work with **relative coordinates** within the slice.

### TileManager

Defined in `gruid-core`, re-exported by both graphical backends. Maps `Cell → Option<&[u8]>` (monochrome alpha bitmap). Backends colorize at render time using fg/bg colors. Returns `None` to fall back to font rendering.

```rust
pub trait TileManager: Send + 'static {
    fn tile_size(&self) -> (usize, usize);
    fn get_tile(&self, cell: &Cell) -> Option<&[u8]>;
}
```

### GPU Rendering (gruid-wgpu)

Instanced quad rendering: each grid cell = one GPU instance. WGSL shader samples a glyph atlas (R8 texture) and blends fg/bg colors. Atlas dynamically grown via fontdue rasterization. DPI-aware scaling.

---

## Quick Start

```rust
use gruid_core::{app::*, grid::Grid, messages::*, style::*, Cell, Point};
use gruid_crossterm::CrosstermDriver;

struct MyModel;

impl Model for MyModel {
    fn update(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::KeyDown { key: Key::Escape, .. } => Some(Effect::End),
            _ => None,
        }
    }

    fn draw(&self, grid: &mut Grid) {
        let style = Style::default().with_fg(Color::from_rgb(0, 255, 0));
        grid.set(Point::new(0, 0), Cell::default().with_char('@').with_style(style));
    }
}

fn main() {
    let mut app = App::new(AppConfig {
        model: MyModel,
        driver: CrosstermDriver::new(),
        width: 80,
        height: 24,
        frame_writer: None,
    });
    app.run().unwrap();
}
```

---

## Shamogu

**Shamanic Mountain Guardian** — a coffee-break roguelike ported from [the Go original](https://codeberg.org/anaseto/shamogu) (17K LOC). ~15% ported (2,970 LOC).

**Implemented:** map generation (cellular automata + vaults), player movement (8-dir + vi keys + mouse), ray-based FOV, 14/27 monster types with A* AI, bump-to-attack combat, message log, status bar, help pager, 174 custom tile bitmaps, crossterm + winit + wgpu backends.

**Remaining:** 13 monster types, 40-trait bitfield, 19 status effects, ~20 spirits, 7 consumables, inventory, ranged attacks, clouds, runic traps, 10 dungeon levels, noise system, animations, save/load, auto-explore.

```bash
cargo run --bin shamogu                                            # Terminal
cargo run -p shamogu --bin shamogu-winit --features shamogu/winit  # Graphical
cargo run -p shamogu --bin shamogu-wgpu --features shamogu/wgpu    # GPU
```

Controls: Arrow keys / vi keys to move · `x` examine · `?` help

---

## Building

```bash
cargo build --workspace              # Build all
cargo test -p gruid-core -p gruid-paths -p gruid-rl -p gruid-ui -p gruid-crossterm  # Test
cargo clippy --workspace -- -D warnings  # Lint
cargo fmt --all                       # Format
```

### Examples

```bash
cargo run --bin roguelike             # Terminal
cargo run --bin roguelike-winit       # Graphical (softbuffer)
cargo run --bin roguelike-wgpu        # Graphical (GPU)
```

---

## Project Structure

```
gruid-rs/
├── crates/
│   ├── gruid-core/          # Core types and framework
│   ├── gruid-paths/         # Pathfinding algorithms
│   ├── gruid-rl/            # Roguelike utilities
│   ├── gruid-ui/            # UI widgets
│   ├── gruid-crossterm/     # Terminal backend
│   ├── gruid-winit/         # Graphical backend (CPU)
│   ├── gruid-wgpu/          # Graphical backend (GPU)
│   ├── gruid-web/           # Browser WASM backend (excluded)
│   └── gruid-tiles/         # Font-to-tile (excluded)
├── examples/                # Roguelike demo (3 binaries)
├── shamogu/                 # Shamogu game (3 binaries)
├── docs/                    # architecture.html
├── AGENTS.md                # Agent coding guidelines
├── CONTEXT.md               # Architecture context
└── TODO.md                  # Task list
```

---

## Credits

Reimplemented from [gruid](https://codeberg.org/anaseto/gruid) and [shamogu](https://codeberg.org/anaseto/shamogu) by Yon (anaseto).

## License

MIT OR Apache-2.0
