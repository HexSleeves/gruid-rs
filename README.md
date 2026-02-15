# gruid-rs

A modern Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) — a cross-platform grid-based UI and game framework originally written in Go.

## Architecture

gruid-rs follows the **Elm Architecture (Model-View-Update)**: your application defines a `Model` that processes `Msg` events via `update()` and renders via `draw()` into a shared `Grid`. The framework diffs frames and only sends changed cells to the `Driver` backend.

```
Msg → Model::update() → Effect → Model::draw() → Grid → diff → Frame → Driver::flush()
```

## Crates

| Crate | Description |
|-------|-------------|
| **`gruid-core`** | Core types: `Grid`, `Cell`, `Point`, `Range`, `Style`, `Msg`, `App`, `Model`, `Driver` |
| **`gruid-paths`** | Pathfinding: A\*, Dijkstra, BFS, Jump Point Search, Connected Components |
| **`gruid-rl`** | Roguelike utilities: FOV (ray-based & symmetric shadow casting), map generation (cellular automata, random walk), priority event queue |
| **`gruid-ui`** | UI widgets: Menu, Pager, TextInput, Label, StyledText with markup, Box drawing |
| **`gruid-tiles`** | Font-to-tile rendering for graphical backends (uses `rusttype` + `image`) |
| **`gruid-crossterm`** | Terminal backend using [crossterm](https://docs.rs/crossterm) |

## Key Design Decisions (vs. Go original)

- **Rust trait hierarchy** instead of Go interfaces: `Pather` → `WeightedPather` → `AstarPather`
- **Generic `EventQueue<E>`** instead of `interface{}` events
- **`Rc<RefCell<...>>`** shared grid buffers for slice semantics (like Go's slice-of-underlying-array)
- **Generation-based cache invalidation** in pathfinding (zero-cost resets between queries)
- **Crossterm** replaces tcell as the terminal backend (pure Rust, no CGo)
- **Builder pattern** with `with_*()` methods on immutable value types (`Cell`, `Style`, `Point`)
- **`serde` feature** for serialization (opt-in, replaces Go's `gob`)

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

## Example: Roguelike Demo

The `examples/roguelike.rs` demo generates a cave map using cellular automata, computes FOV with ray casting, and lets you explore with arrow keys / hjkl:

```bash
cargo run --bin roguelike
```

## Building

```bash
cargo build --workspace
cargo test --workspace
```

## Credits

Inspired by and reimplemented from [gruid](https://codeberg.org/anaseto/gruid) by Yon (anaseto). The original Go library is a beautifully designed framework for roguelike development with an Elm-architecture core.

## License

MIT OR Apache-2.0
