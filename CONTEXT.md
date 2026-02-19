# Agent Context — gruid-rs

Read this before making changes. See also `AGENTS.md` for coding standards
and the **mandatory pre-commit checklist**.

---

## What This Project Is

A Rust reimplementation of [gruid](https://codeberg.org/anaseto/gruid) — a Go
cross-platform grid-based UI and game framework using the Elm architecture.

- **Repo:** `https://github.com/HexSleeves/gruid-rs`
- **Go original:** `https://codeberg.org/anaseto/gruid`
- **Shamogu (Go game):** `https://codeberg.org/anaseto/shamogu`

---

## Current State

| Metric | Value |
|--------|-------|
| Crates | 9 (7 framework + shamogu + examples) |
| LOC | ~18,700 Rust |
| Tests | 219 passing, 0 failures |
| Clippy | Zero warnings |
| Go API parity | ~99% |
| Backends | 3 (crossterm, winit, wgpu) + 1 excluded (web/WASM) |

---

## Workspace Layout

```
gruid-rs/
├── Cargo.toml              # Workspace root (Rust 2024, resolver 2)
├── crates/
│   ├── gruid-core/         # 2,787 LOC — Grid, Cell, Point, Range, Style, Msg, Model, Driver, TileManager
│   ├── gruid-paths/        # 1,755 LOC — A*, Dijkstra, BFS, JPS, Connected Components
│   ├── gruid-rl/           # 2,919 LOC — FOV, MapGen, Vault, EventQueue
│   ├── gruid-ui/           # 4,195 LOC — Menu, Pager, TextInput, Label, BoxDecor, StyledText, Replay
│   ├── gruid-crossterm/    # 261 LOC  — Terminal driver (poll-based)
│   ├── gruid-winit/        # 862 LOC  — Native window driver (softbuffer + fontdue)
│   ├── gruid-wgpu/         # 1,386 LOC — GPU driver (wgpu + instanced quads + glyph atlas)
│   ├── gruid-web/          # 539 LOC  — WASM browser driver (excluded, wasm32-only)
│   └── gruid-tiles/        # excluded — font-to-tile (rusttype + image)
├── examples/               # 960 LOC  — Roguelike demo (crossterm + winit + wgpu)
├── shamogu/                # 2,970 LOC — Shamanic Mountain Guardian game
└── docs/                   # architecture.html
```

---

## Crate Dependency Graph

```
gruid-core (no deps)
    ├── gruid-paths (gruid-core)
    │       └── gruid-rl (gruid-core, gruid-paths, rand)
    ├── gruid-ui (gruid-core)
    ├── gruid-crossterm (gruid-core, crossterm)
    ├── gruid-winit (gruid-core, winit, softbuffer, fontdue)
    ├── gruid-wgpu (gruid-core, winit, wgpu, fontdue, bytemuck)
    └── gruid-web (gruid-core, wasm-bindgen, web-sys) [excluded]
```

Optional: `serde` feature on gruid-core, gruid-paths, gruid-rl.

---

## Architecture

### Elm Architecture
```
Msg → Model::update() → Effect → Model::draw() → Grid → diff → Frame → Driver::flush()
```

### Two Driver Models
- **Poll-based** (`Driver`): App owns main thread, calls `poll_msgs()` (crossterm)
- **Event-loop** (`EventLoopDriver`): Driver owns main thread, pushes into `AppRunner` (winit, wgpu, web)

### Grid System
`Grid` uses `Rc<RefCell<GridBuffer>>` for Go-like slice semantics. All coordinates
are relative to slice origin. `set()`/`fill()` take `&self` (interior mutability).

### TileManager
Defined in `gruid-core::tiles`, re-exported by gruid-winit and gruid-wgpu.
Maps `Cell → Option<&[u8]>` (monochrome alpha bitmap). Backends colorize at render time.

### GPU Rendering (gruid-wgpu)
Instanced quads — one per grid cell. WGSL shader samples glyph atlas (R8 texture)
for fg/bg color blending. Atlas dynamically grown via fontdue rasterization.

---

## Key Design Decisions vs Go

| Aspect | Go | Rust |
|--------|-----|------|
| Grid memory | Slice of array | `Rc<RefCell<GridBuffer>>` |
| Driver model | Single interface | `Driver` + `EventLoopDriver` |
| Pathfinding | `interface{}` | Trait hierarchy (`Pather` → `WeightedPather` → `AstarPather`) |
| Cache reset | Manual memset | Generation-based O(1) invalidation |
| Terminal | tcell (CGo) | crossterm (pure Rust) |
| Graphics | SDL2 (CGo) | winit+softbuffer / wgpu (pure Rust) |
| Serialization | gob+zlib | serde (opt-in feature) |
| Tiles | Built into SDL2 | `TileManager` trait in core |

---

## Style Notes

- **Rust 2024 edition** (1.85+). `gen` is reserved — use `cur_gen`.
- **Builder pattern:** `with_*()` methods return `Self` by value.
- **Error type:** `Box<dyn std::error::Error>` throughout.
- **Naming:** `Box` is reserved → file is `box_.rs`, type is `BoxDecor`.
- **Coordinates:** always relative to the view's origin.
