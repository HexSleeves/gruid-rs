# AGENTS.md — Agent Guidelines for gruid-rs

This file provides guidance for agentic coding agents working on the gruid-rs codebase.

---

## Project Overview

**gruid-rs** is a Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) —
a cross-platform grid-based UI and game framework using the Elm architecture
(Model-View-Update). The project targets roguelike games but is general-purpose.

- **Location:** `/home/exedev/gruid-rs`
- **Go original:** `/home/exedev/gruid/` (for reference)
- **Edition:** Rust 2024 (requires Rust 1.85+)
- **Workspace:** 7 crates + examples

---

## Build Commands

```bash
# Check entire workspace compiles (no warnings)
cargo check --workspace

# Build debug mode
cargo build

# Build release mode
cargo build --release

# Build with all features
cargo check --workspace --all-features

# Run the terminal demo
cargo run --bin roguelike

# Run the graphical demo (winit)
cargo run --bin roguelike-winit
```

---

## Test Commands

```bash
# Run all tests (skip winit — pulls too many deps for test build)
cargo test -p gruid-core -p gruid-paths -p gruid-rl -p gruid-ui -p gruid-crossterm

# Run tests for a specific crate
cargo test -p gruid-core
cargo test -p gruid-paths
cargo test -p gruid-rl
cargo test -p gruid-ui

# Run a single test by name
cargo test -p gruid-core test_name
cargo test -p gruid-paths jps_path
cargo test -p gruid-rl fov
cargo test -p gruid-ui menu

# Run tests with output
cargo test -p gruid-core -- --nocapture
```

---

## Linting

```bash
# Run clippy on workspace
cargo clippy --workspace --all-features -- -D warnings

# Run clippy on a specific crate
cargo clippy -p gruid-core

# Format code
cargo fmt --check  # Check without modifying
cargo fmt          # Format in place
```

---

## Code Style Guidelines

### Formatting

- Use **Rust 2024 edition** (set in Cargo.toml)
- Run `cargo fmt` before committing
- 4-space indentation (Rust default)
- Maximum line length: 100 characters (soft guideline)
- Use trailing commas in match arms and function calls

### Imports

- Use absolute imports within a crate: `use crate::module::Item`
- Use external crate imports: `use gruid_core::{Point, Grid}`
- Group std imports before crate imports, then external deps
- Use wildcard sparingly: `pub use messages::*` is acceptable for re-exports

### Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Types | PascalCase | `Point`, `Grid`, `PathRange` |
| Functions | snake_case | `astar_path()`, `dijkstra_map()` |
| Constants | SCREAMING_SNAKE | `UNREACHABLE`, `DEFAULT` |
| Modules | snake_case | `mod geom`, `mod neighbors` |
| Traits | PascalCase | `Pather`, `Model`, `Driver` |
| Enums | PascalCase | `Msg::Tick`, `Key::Char` |

**Reserved words:** `Box` is reserved → use `box_.rs` for file, `BoxDecor` for type.

### Types

- Use `i32` for grid coordinates (matches Go gruid)
- Use `u32` for RGB colors (`Color(r,g,b)` packed as u32)
- Prefer `&[T]` slices over `&Vec<T>` for function parameters
- Use `Rc<RefCell<>>` for shared mutable state (Grid buffer)
- Use `bitflags!` or manual bitflag impls for flag types (`AttrMask`, `ModMask`)

### Error Handling

- Use `Box<dyn std::error::Error>` throughout (no custom error types yet)
- Driver trait methods return `Result<(), Box<dyn std::error::Error>>`
- Use `?` operator for propagation
- Consider `anyhow` for application code if needed

### Documentation

- Use doc comments (`///` or `//!`) for public API
- Include examples in doc comments where helpful
- Use "`[`Type`]`" for linking types in documentation

### Derives

Common derives for types:
```rust
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
```

Only add derives as needed. Not all types need `Copy`, `Hash`, or `Serialize`.

### Interior Mutability

Grid uses `Rc<RefCell<GridBuffer>>` for slice semantics. Methods like `set()` and
`fill()` take `&self` not `&mut self`. This matches Go's pass-by-value semantics.

### Feature Flags

- Gate optional features behind Cargo features
- Serde support: `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`
- Example: `gruid-core/Cargo.toml` has `serde` feature

### Testing

- Place tests in `tests/` module or inline with `#[cfg(test)]` blocks
- Match Go gruid test coverage where possible
- Use property-based tests for algorithm verification (JPS == A* path lengths)

---

## Architecture Notes

### Coordinate System

Both `gruid_core::Grid` and `gruid_rl::Grid` use **relative** coordinates.
After `grid.slice(Range::new(5,5,10,10))`, `grid.set(Point::new(0,0), c)` writes
to position (5,5) in the underlying buffer. All public methods work with relative
coordinates.

### Two Driver Models

1. **Poll-based** (`Driver`): App calls `poll_msgs()` in a loop (crossterm)
2. **Event-loop-based** (`EventLoopDriver`): Driver owns main thread, calls into
   `AppRunner` (winit)

### Effect System

- `Effect::Cmd(f)` — spawns thread, runs `f()`, sends optional result msg
- `Effect::Sub(f)` — long-running subscription
- `Effect::Batch(vec)` — multiple effects
- `Effect::End` — signal quit

---

## Crate Dependency Graph

```
gruid-core (no deps)
    ├── gruid-paths (depends on gruid-core)
    │       └── gruid-rl (depends on gruid-core, gruid-paths, rand)
    ├── gruid-ui (depends on gruid-core)
    ├── gruid-crossterm (depends on gruid-core, crossterm)
    └── gruid-winit (depends on gruid-core, winit, softbuffer, fontdue)
```

---

## Common Tasks

### Adding a New Crate

1. Create `crates/gruid-newcrate/`
2. Add `[package]` section with workspace fields
3. Add to `workspace.members` in root `Cargo.toml`
4. Add dependencies to `workspace.dependencies` and use in crate

### Running the Roguelike Demo

```bash
# Terminal version
cargo run --bin roguelike

# Graphical version (requires display)
cargo run --bin roguelike-winit
```

Controls: arrows to move, `x` look mode, `p` path overlay, `d` Dijkstra, `?` help

---

## Pre-Commit Checklist (MANDATORY)

Every agent MUST complete ALL of these steps before finishing work:

### 1. Format
```bash
cargo fmt --all
```
Fix any formatting issues. Do NOT commit unformatted code.

### 2. Lint
```bash
cargo clippy --workspace -- -D warnings
```
Fix ALL clippy warnings and errors. Zero warnings policy.

### 3. Test
```bash
cargo test --workspace
```
ALL tests must pass. Zero failures.

### 4. Update TODO.md
Mark completed items as ✅. Add new items discovered during work.
Keep priorities accurate (P0 > P1 > P2).

### 5. Update CONTEXT.md
Update "Known Working State" and "Known Incomplete / Missing" sections.
Update test counts, LOC counts, and feature status.

### 6. Commit
```bash
git add <changed files explicitly>
git commit -m "descriptive message

- bullet points of what changed
- test count: X passed"
```
Use conventional commit style. Never use `git add -A` or `git add .`.

### 7. Push
```bash
git push
```
Always push after committing. Don't leave unpushed commits.

---

## Reference

- Go original: cloned as needed from `https://codeberg.org/anaseto/gruid`
- TODO: See `TODO.md` for prioritized task list
- Context: See `CONTEXT.md` for architecture deep-dive
