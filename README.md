<p align="center">
  <strong>gruid-rs</strong><br>
  <em>A modern Rust reimplementation of <a href="https://codeberg.org/anaseto/gruid">gruid</a> — cross-platform grid-based UI &amp; game framework</em>
</p>

<p align="center">
  <a href="#crates">Crates</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#shamogu">Shamogu</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#building">Building</a> •
  <a href="#license">License</a>
</p>

---

## Overview

gruid-rs is a Rust workspace of 8 crates that together provide everything needed to build grid-based terminal and graphical applications — especially roguelikes. It follows the **Elm Architecture** (Model → Update → View) and includes pathfinding, FOV, map generation, UI widgets, and two rendering backends.

The project also includes **Shamogu** (*Shamanic Mountain Guardian*), a coffee-break roguelike being ported from [the original Go version](https://codeberg.org/anaseto/shamogu) (17K LOC) as both a test harness and a real game.

| | |
|---|---|
| **Workspace** | 8 crates + examples + shamogu game |
| **LOC** | ~17,300 Rust (framework: ~13,400 · shamogu: ~2,970 · examples: ~935) |
| **Tests** | 221 passing |
| **Edition** | Rust 2024 (requires 1.85+) |
| **License** | MIT OR Apache-2.0 |

> 📐 **[Architecture Documentation](docs/architecture.html)** — interactive diagrams, crate deep-dives, and design decisions.

---

## Crates

```
gruid-core ─────┬── gruid-paths ──── gruid-rl
                ├── gruid-ui
                ├── gruid-crossterm
                └── gruid-winit
```

| Crate | LOC | Description |
|-------|-----|-------------|
| **[gruid-core](crates/gruid-core)** | 4,200 | Core types: `Grid`, `Cell`, `Point`, `Range`, `Style`, `Msg`, `App`/`AppRunner`, `Model`, `Driver`/`EventLoopDriver` |
| **[gruid-paths](crates/gruid-paths)** | 2,800 | A\*, Dijkstra, BFS, Jump Point Search, Connected Components |
| **[gruid-rl](crates/gruid-rl)** | 2,500 | FOV (ray-based & shadow casting), map generation (cellular automata, random walk), vaults, event queue |
| **[gruid-ui](crates/gruid-ui)** | 2,400 | Menu, Pager, TextInput, Label, StyledText with `@r`/`@g`/`@b` markup, Box drawing |
| **[gruid-crossterm](crates/gruid-crossterm)** | 600 | Terminal backend — poll-based `Driver` using [crossterm](https://docs.rs/crossterm) |
| **[gruid-winit](crates/gruid-winit)** | 1,000 | Graphical backend — event-loop `EventLoopDriver` using [winit](https://docs.rs/winit) + [softbuffer](https://docs.rs/softbuffer) + [fontdue](https://docs.rs/fontdue). Custom tile rendering via `TileManager` trait |
| **gruid-tiles** | — | Font-to-tile rendering (rusttype + image). Currently excluded from workspace |

---

## Architecture

gruid-rs implements the **Elm Architecture** for grid UIs:

```
Msg → Model::update() → Effect → Model::draw() → Grid → diff → Frame → Driver::flush()
```

Your application defines a `Model` that:
1. Receives `Msg` events (keyboard, mouse, tick, custom)
2. Returns `Effect`s (commands, subscriptions, batch, end)
3. Draws to a shared `Grid` each frame
4. The framework diffs the grid and only sends changed cells to the driver

### Two Driver Models

| Pattern | Trait | Backend | How it works |
|---------|-------|---------|--------------|
| **Poll-based** | `Driver` | crossterm | App owns the loop, calls `poll_msgs()` |
| **Event-loop** | `EventLoopDriver` | winit | Driver owns the main thread, pushes events into `AppRunner` |

Both use the same `Model` trait — game logic works unchanged across backends.

### Grid System

`Grid` uses `Rc<RefCell<GridBuffer>>` for Go-like slice semantics:

```rust
let grid = Grid::new(80, 24);        // Allocates buffer
let sub = grid.slice(range);          // Same buffer, offset view
sub.set(Point::new(0, 0), cell);      // Writes at (range.min.x, range.min.y)
```

All public methods work with **relative coordinates** within the slice.

### Key Design Decisions vs Go

| Aspect | Go gruid | Rust gruid-rs |
|--------|----------|---------------|
| Grid memory | Slice of underlying array | `Rc<RefCell<GridBuffer>>` |
| Driver model | `DriverPollMsg` interface | Separate `Driver` + `EventLoopDriver` traits |
| Pathfinding | `interface{}` | `Pather` → `WeightedPather` → `AstarPather` trait hierarchy |
| Events | `interface{}` | Generic `EventQueue<E>` |
| Terminal | tcell (CGo) | crossterm (pure Rust) |
| Graphics | SDL2 (CGo) | winit + softbuffer + fontdue (pure Rust) |
| Tiles | Built into SDL2 driver | `TileManager` trait + embedded bitmaps |
| Serialization | gob + zlib | serde (opt-in feature flag) |
| Cache resets | Manual | Generation-based zero-cost invalidation |

---

## Shamogu

**Shamanic Mountain Guardian** — a coffee-break roguelike with tactical movement, totemic spirits, and stealth mechanics. Being ported from the [Go original](https://codeberg.org/anaseto/shamogu) (17,210 LOC / 41 files).

### Current State (~15% ported, 2,969 LOC / 14 files)

#### ✅ Implemented

| System | Details |
|--------|---------|
| **Map generation** | Cellular automata + vault loading (small & big vaults) |
| **Player movement** | 8-directional + vi keys + mouse click-to-move |
| **Field of View** | Ray-based FOV with lit/dark/explored states |
| **Monsters** | 14 of 27 types with A\* pathfinding AI (hunt/wander) |
| **Combat** | Bump-to-attack with HP, attack, defense stats |
| **Message log** | Color-coded combat and system messages |
| **Status bar** | HP, position, turn count, mode indicators |
| **Help pager** | Press `?` for keybinding reference |
| **Terminal backend** | Crossterm with full color and mouse support |
| **Graphical backend** | Winit with 174 custom monochrome tile bitmaps, DPI-aware scaling |

<details>
<summary><strong>Implemented Monsters (14)</strong></summary>

BarkingHound · BerserkingSpider · ConfusingEye · FireLlama · FourHeadedHydra · HungryRat · LashingFrog · RampagingBoar · TemporalCat · ThunderPorcupine · VenomousViper · WalkingMushroom · MirrorToad · SatansFrog

</details>

#### 🔲 Not Yet Ported

<details>
<summary><strong>Monsters (13 remaining)</strong></summary>

AcidMound · BlazingGolem · BlinkButterfly · BurningPhoenix · CrazyDruid · EarthDragon · ExplodingNadre · FearsomeLich · MadOctopode · NoisyImp · TotemWasp · UndeadKnight · WalkingTree · WarpingWraith · WindFox

</details>

| System | Go LOC | What's Missing |
|--------|--------|----------------|
| **Monster traits** | ~200 | 40-trait bitfield (PatternRampage, BurningHits, Pushing, Dazzling, Gluttony…) |
| **Status effects** | ~213 | 19 statuses (Berserk, Confusion, Fear, Fire, Poison, Lignification, Daze, Shadow, Sprint, TimeStop, Vampirism…) with duration system |
| **Spirits** | ~415 | ~20 totemic spirits with abilities, charges, level-up, attack/defense bonuses |
| **Comestibles** | ~100 | 7 consumable items (AmbrosiaBerries, BerserkingFlower, ClarityLeaves, FirebreathPepper, FoggySkinOnion, LignificationFruit, TeleportMushroom) |
| **Inventory** | ~500 | 3 spirit slots + 5 item slots, equip/use menus |
| **Combat (advanced)** | ~890 | Ranged attacks, confusion, knockback, special abilities, defense/attack modifiers |
| **Actions** | ~1,880 | Full action system — player actions, monster special behaviors, spirit invocations |
| **Effects** | ~1,315 | Status application, spirit abilities, comestible effects |
| **Clouds** | ~293 | Steam, fire, poison clouds with propagation |
| **Runic traps** | ~156 | 5 types: Berserk, Fire, Lignification, Poison, Warp |
| **Noise system** | ~100 | Sound propagation alerting monsters |
| **Multiple levels** | ~400 | 10 dungeon levels with stairs, ProcInfo level themes |
| **Terrain** | ~50 | Foliage, Rubble, Translucent walls (only Wall/Floor implemented) |
| **Map objects** | ~200 | Menhirs (healing stones), Totems, Portals, Corruption Orbs |
| **Animations** | ~466 | Visual effects for combat, abilities, explosions |
| **UI (advanced)** | ~500+ | Examine mode, targeting, inventory menus, game-over screen |
| **Auto-movement** | ~218 | Auto-explore, auto-travel, run-in-direction |
| **Save/load** | ~160 | Serialization with serde (Go uses gob + zlib) |
| **Character dump** | ~359 | End-of-game statistics export |
| **Pathfinding (game)** | ~248 | Noise-aware pathing, monster waypoints |

### Running Shamogu

```bash
# Terminal (crossterm)
cargo run --bin shamogu

# Graphical (winit + tiles)
cargo run -p shamogu --bin shamogu-winit --features shamogu/winit
```

**Controls:** Arrow keys / vi keys to move · `x` examine · `p` path overlay · `d` Dijkstra heatmap · `?` help

### Tile Rendering

The winit backend uses 174 embedded monochrome 16×24 tile bitmaps (one per game character). Tiles are colorized at render time using each cell's foreground/background colors. The `TileManager` trait allows any application to provide custom tiles — when `get_tile()` returns `None`, the renderer falls back to fontdue glyph rasterization.

```rust
pub trait TileManager {
    fn tile_size(&self) -> (usize, usize);
    fn get_tile(&self, cell: &Cell) -> Option<&[u8]>;
}
```

DPI-aware scaling is handled via `WinitConfig::tile_scale`: `0` = auto-detect from display scale factor, `N` = explicit Nx scaling.

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
        grid.set(
            Point::new(0, 0),
            Cell::default().with_char('@').with_style(style),
        );
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

## Building

```bash
# Build entire workspace
cargo build --workspace

# Run all tests (221 passing)
cargo test --workspace

# Lint
cargo clippy --workspace --all-features -- -D warnings

# Format
cargo fmt --all
```

### Examples

The `examples/` crate contains a small roguelike demo showcasing the framework:

```bash
# Terminal
cargo run --bin roguelike

# Graphical
cargo run --bin roguelike-winit
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
│   ├── gruid-winit/         # Graphical backend
│   └── gruid-tiles/         # Font-to-tile (excluded)
├── examples/                # Roguelike demo
│   ├── src/lib.rs           # Shared game logic
│   ├── roguelike.rs         # Terminal binary
│   └── roguelike_winit.rs   # Graphical binary
├── shamogu/                 # Shamogu game port
│   ├── src/
│   │   ├── lib.rs           # Library root
│   │   ├── game.rs          # Game state and logic
│   │   ├── gamemap.rs       # Map generation (844 LOC)
│   │   ├── model.rs         # Elm architecture Model impl
│   │   ├── entity.rs        # Monsters and player
│   │   ├── tile_data.rs     # 174 embedded tile bitmaps
│   │   ├── tiles.rs         # TileManager implementation
│   │   ├── colors.rs        # Selenized color palette
│   │   ├── combat.rs        # Combat resolution
│   │   ├── log.rs           # Message log
│   │   ├── terrain.rs       # Terrain types
│   │   ├── fov_.rs          # FOV wrapper
│   │   ├── main.rs          # Crossterm binary
│   │   └── main_winit.rs    # Winit binary
│   ├── data/                # Vault definitions
│   └── tiles/               # 174 PNG tile sources
├── docs/                    # Architecture documentation
├── AGENTS.md                # Agent coding guidelines
├── CONTEXT.md               # Architecture deep-dive
└── TODO.md                  # Prioritized task list
```

---

## Credits

Reiplemented from [gruid](https://codeberg.org/anaseto/gruid) and [shamogu](https://codeberg.org/anaseto/shamogu) by Yon (anaseto). The original Go libraries are beautifully designed frameworks for roguelike development.

## License

MIT OR Apache-2.0
