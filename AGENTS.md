# AGENTS.md — Agent Guidelines for gruid-rs

---

## Project Overview

**gruid-rs** is a Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) —
a cross-platform grid-based UI and game framework using the Elm architecture.
Targets roguelike games but is general-purpose.

- **Location:** `/home/exedev/gruid-rs`
- **Go original:** `/home/exedev/gruid/` (clone as needed for reference)
- **Edition:** Rust 2024 (requires Rust 1.85+)
- **Workspace:** 9 crates + examples + shamogu game

---

## Build Commands

```bash
cargo check --workspace              # Check everything compiles
cargo build                           # Debug build
cargo build --release                 # Release build
cargo run --bin roguelike             # Terminal demo
cargo run --bin roguelike-winit       # Graphical demo (softbuffer)
cargo run --bin roguelike-wgpu        # Graphical demo (GPU/wgpu)
cargo run --bin shamogu               # Shamogu terminal
cargo run -p shamogu --bin shamogu-winit --features shamogu/winit  # Shamogu graphical
cargo run -p shamogu --bin shamogu-wgpu --features shamogu/wgpu    # Shamogu GPU
```

---

## Test Commands

```bash
# Run all tests (skip winit/wgpu — need display)
cargo test -p gruid-core -p gruid-paths -p gruid-rl -p gruid-ui -p gruid-crossterm

# Single crate
cargo test -p gruid-core

# Single test
cargo test -p gruid-core test_name

# With output
cargo test -p gruid-core -- --nocapture
```

---

## Linting

```bash
cargo clippy --workspace -- -D warnings   # Zero warnings policy
cargo fmt --check                          # Check formatting
cargo fmt --all                            # Auto-format
```

---

## Code Style

### Formatting
- Rust 2024 edition, 4-space indentation
- `cargo fmt` before every commit
- Trailing commas in match arms and function calls

### Imports
- Group: std → crate → external deps
- Absolute within crate: `use crate::module::Item`
- External: `use gruid_core::{Point, Grid}`

### Naming
| Element | Convention | Example |
|---------|------------|---------|
| Types | PascalCase | `Point`, `Grid`, `PathRange` |
| Functions | snake_case | `astar_path()`, `dijkstra_map()` |
| Constants | SCREAMING_SNAKE | `UNREACHABLE`, `DEFAULT` |
| Traits | PascalCase | `Pather`, `Model`, `Driver` |

**Reserved:** `Box` → `box_.rs` / `BoxDecor`. `gen` → `cur_gen`.

### Types
- `i32` for grid coordinates
- `u32` for RGB colors
- `&[T]` over `&Vec<T>` for parameters
- `Rc<RefCell<>>` for shared mutable Grid state
- `Box<dyn std::error::Error>` for errors

### Derives
```rust
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
```
Only add derives as needed.

### Interior Mutability
Grid uses `Rc<RefCell<GridBuffer>>`. Methods like `set()` take `&self` not `&mut self`.

---

## Architecture Notes

### Coordinate System
All grid coordinates are **relative** to the slice origin.
`grid.slice(Range::new(5,5,10,10))` → `grid.set(Point::new(0,0), c)` writes to (5,5).

### Two Driver Models
1. **Poll-based** (`Driver`): App calls `poll_msgs()` in a loop (crossterm)
2. **Event-loop** (`EventLoopDriver`): Driver owns main thread (winit, wgpu, web)

### TileManager
Defined once in `gruid-core::tiles`. Re-exported by gruid-winit and gruid-wgpu.
Maps `Cell → Option<&[u8]>` (monochrome alpha bitmap).

### Effect System
- `Effect::Cmd(f)` — one-shot thread, sends optional result msg
- `Effect::Sub(f)` — long-running subscription
- `Effect::Batch(vec)` — multiple effects
- `Effect::End` — signal quit

---

## Crate Dependency Graph

```
gruid-core (no deps)
    ├── gruid-paths
    │       └── gruid-rl (+ rand)
    ├── gruid-ui
    ├── gruid-crossterm (+ crossterm)
    ├── gruid-winit (+ winit, softbuffer, fontdue)
    ├── gruid-wgpu (+ winit, wgpu, fontdue, bytemuck)
    └── gruid-web (+ wasm-bindgen, web-sys) [excluded]
```

---

## Pre-Commit Checklist (MANDATORY)

Every agent MUST complete ALL steps before finishing:

### 1. Format
```bash
cargo fmt --all
```

### 2. Lint
```bash
cargo clippy --workspace -- -D warnings
```
Zero warnings. Fix all issues.

### 3. Test
```bash
cargo test -p gruid-core -p gruid-paths -p gruid-rl -p gruid-ui -p gruid-crossterm
```
All tests must pass.

### 4. Commit
```bash
git add <changed files explicitly>
git commit -m "descriptive message"
```
Conventional commit style. Never `git add -A` or `git add .`.

### 5. Push
```bash
git push
```
Always push after committing.
