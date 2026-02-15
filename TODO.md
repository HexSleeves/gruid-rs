# gruid-rs TODO — Full Port Gap Analysis

Comprehensive task list for completing the Rust port of [gruid](https://codeberg.org/anaseto/gruid).
The Go original lives at `/home/exedev/gruid/` for reference.

Current state: ~6,500 LOC across 7 crates. Estimated remaining: ~2,150 LOC.

---

## 🔴 Critical: Semantic Bugs (P0)

These are things that exist but behave incorrectly.

### C1. Grid coordinate system — relative vs absolute
- **File:** `crates/gruid-rl/src/grid.rs`, `crates/gruid-core/src/grid.rs`
- **Bug:** Go `Set`/`At`/`Contains` use **relative** coordinates (slice-local). After
  `grid.Slice(Range(5,5,10,10))`, Go's `Set(Point{0,0}, c)` writes to position (5,5)
  in the underlying buffer. Our Rust rl::Grid uses **absolute** coordinates, so
  `set(Point(0,0), c)` is out of bounds on a sliced grid.
- **Impact:** Breaks all downstream consumers — FOV, mapgen, UI widgets all assume
  relative coordinates after slicing.
- **Fix:** In both `Grid` types, `set(p)` / `at(p)` must add `self.bounds.min` to `p`
  internally. `contains(p)` must check `p + bounds.min` is within bounds. `Slice(rg)`
  must take a **relative** range and offset by current `bounds.min`.
- **Ref:** Go `grid.go` lines 200-230 (Set/At), lines 155-175 (Slice)

### C2. JPS 4-way (no-diags) mode is broken
- **File:** `crates/gruid-paths/src/jps.rs`
- **Bugs (4):**
  1. Forced-neighbor detection is a logical contradiction: `!passable(X) && passable(X)`
  2. Diagonal jumps fall into the horizontal branch instead of a separate no-diags handler
  3. Direction normalization uses `signum` instead of Go's `dirnorm` (misclassifies
     non-clean directions after diagonal jumps)
  4. Path interpolation doesn't insert cardinal intermediates for diagonal steps
- **Impact:** 4-way JPS produces wrong/no paths. 8-way mode works fine.
- **Fix:** Port Go's `jumpStraightNoDiags`, `jumpDiagonalNoDiags`, `neighborsNoDiags`,
  `jumpPathNoDiags` functions faithfully.
- **Ref:** Go `paths/jps.go` lines 200-616

### C3. FOV algorithm divergence
- **File:** `crates/gruid-rl/src/fov.rs`
- **Bugs:**
  1. `vision_map` uses a different algorithm than Go's octant-parent ray propagation
  2. `Lighter` trait missing the `src` (source point) parameter: Go has
     `Cost(src, from, to Point) int`; Rust has `cost(from, to) -> i32`
  3. `Lighter` trait missing `MaxCost(src Point) int` method
  4. SSC algorithm missing `diags` parameter (Go supports 4-way mode)
- **Fix:** Port Go's `visionMap` octant traversal from `rl/fov.go` lines 130-250.
  Update `Lighter` trait to include source and max_cost.
- **Ref:** Go `rl/fov.go`

### C4. Cellular automata `countWalls` off-by-one
- **File:** `crates/gruid-rl/src/mapgen.rs`, `count_walls_ring()` method
- **Bug:** Go includes the center cell `(0,0)` in the wall count. Rust has
  `if dx == 0 && dy == 0 { continue; }` which skips it.
- **Impact:** Changes threshold behavior — maps generate differently.
- **Fix:** Remove the `(0,0)` skip. One-line change.
- **Ref:** Go `rl/mapgen.go` `countWalls` function

---

## 🟡 Major: Missing Features (P1)

### M1. Vault system (rl/mapgen)
- **File:** New `crates/gruid-rl/src/vault.rs`
- **Missing:** Entire Vault type — parsing ASCII art maps into Grid overlays.
- **Methods needed:** `new(content)`, `parse(text)`, `content()`, `size()`,
  `set_runes(map)`, `runes()`, `iter()`, `draw(grid, pos)`, `reflect()`, `rotate()`
- **Ref:** Go `rl/mapgen.go` Vault section (~100 lines)

### M2. `KeepCC` — ensure cave connectivity
- **File:** `crates/gruid-rl/src/mapgen.rs`
- **Missing:** `MapGen::keep_connected(cell, wall)` — uses `gruid-paths` CC to find
  the largest connected component and fill the rest with walls.
- **Ref:** Go `rl/mapgen.go` `KeepCC` function

### M3. Multi-source FOV lighting
- **File:** `crates/gruid-rl/src/fov.rs`
- **Missing:** `FOV::light_map(lighter, sources, max_cost)` — ray-based FOV from
  multiple sources. `FOV::ssc_light_map(sources, max_range, passable)` — SSC from
  multiple sources.
- **Ref:** Go `rl/fov.go` `LightMap`/`SSCLightMap`

### M4. FOV ray traceback
- **File:** `crates/gruid-rl/src/fov.rs`
- **Missing:** `FOV::from(lighter, to) -> Option<LightNode>` — return the previous
  position in the light ray. `FOV::ray(lighter, to) -> Vec<LightNode>` — return the
  full ray path from source to target.
- **Ref:** Go `rl/fov.go` `From`/`Ray` methods

### M5. Replay widget
- **File:** New `crates/gruid-ui/src/replay.rs`
- **Missing:** Entire `Replay` widget — frame playback with speed control, pause,
  seeking, undo, help overlay.
- **Types:** `Replay`, `ReplayConfig`, `ReplayKeys`, `ReplayAction`
- **Methods:** `new(config)`, `update(msg)`, `draw(grid)`, `set_frame(idx)`,
  `seek(offset)`
- **Depends on:** Working frame recording/decoding (see M10)
- **Ref:** Go `ui/replay.go` (~390 lines)

### M6. Menu widget — complete implementation
- **File:** `crates/gruid-ui/src/menu.rs`
- **Missing:**
  - Mouse event handling (hover to activate, click to invoke)
  - Pagination (PageUp/PageDown across pages)
  - Multi-column / table layout (`MenuStyle.layout`)
  - Disabled-entry skipping during keyboard navigation
  - `set_entries()`, `set_box()`, `active_bounds()`, `bounds()`
  - Page number display in box footer
- **Ref:** Go `ui/menu.go` (~720 lines — the largest UI widget)

### M7. Pager widget — complete implementation
- **File:** `crates/gruid-ui/src/pager.rs`
- **Missing:**
  - Horizontal scroll (Left/Right/Start keys)
  - Half-page navigation (HalfPageDown/HalfPageUp)
  - Top/Bottom jump (Home/End)
  - Mouse wheel scrolling
  - Standalone `MsgInit` mode (pager as main app model)
  - `set_cursor()`, `set_box()`, `set_lines()`, `view()`, `action()` getters
  - Line number display in box
- **Ref:** Go `ui/pager.go` (~400 lines)

### M8. TextInput widget — complete implementation
- **File:** `crates/gruid-ui/src/text_input.rs`
- **Missing:**
  - Prompt text support (`Prompt StyledText`)
  - Mouse click-to-position cursor
  - `set_cursor()`, `set_box()`, `action()` getters
  - Cursor rendering with style swap
- **Ref:** Go `ui/textinput.go` (~260 lines)

### M9. Sub effects — background thread spawning
- **File:** `crates/gruid-core/src/app.rs`
- **Missing:** `Effect::Sub` is silently dropped. Need to spawn a thread and
  feed messages back via the channel.
- **Fix:** In `App::handle_effect` for `Effect::Sub(f)`, spawn
  `std::thread::spawn(move || f(ctx, tx))`. In `AppRunner::handle_effect`,
  same but store join handles.
- **Ref:** Go `ui.go` subscription dispatch

### M10. Frame recording — real serialization
- **File:** `crates/gruid-core/src/recording.rs`
- **Missing:** Current implementation is a stub. Need real frame encode/decode.
- **Fix:** Use `bincode` + `flate2` (Rust equivalents of Go's gob+gzip).
  Gate behind `serde` feature. `FrameEncoder::encode(frame)`,
  `FrameDecoder::decode() -> Option<Frame>`.
- **Also:** Add `time: Instant` field to `Frame` struct for replay timing.
- **Ref:** Go `recording.go`

---

## 🟢 Minor: Missing Methods & Polish (P2)

### P1. Range missing methods
- **File:** `crates/gruid-core/src/geom.rs`
- `Sub(Point)` — translate range by subtracting a point
- `Add(Point)` — translate range by adding a point
- `In(Range)` — whether this range is fully contained in another
- `Eq()` — already have `PartialEq`, but explicit method for Go compat
- `RelMsg(msg)` — translate mouse positions in a Msg relative to a range

### P2. Grid missing methods
- **File:** `crates/gruid-core/src/grid.rs`
- `Resize(w, h)` — grow or shrink, preserving content
- `String()` / `Display` impl
- Full `GridIterator` with `set_p()`, `set_cell()`, `reset()` for efficient
  mutable traversal without borrow issues

### P3. rl::Grid missing methods
- **File:** `crates/gruid-rl/src/grid.rs`
- `Resize(w, h)` — grow underlying buffer
- `Range()` — return relative range (min at 0,0)
- `AtU(p)` — unchecked access (skip bounds check)
- Full `GridIterator` matching core Grid

### P4. Key/ModMask/MouseAction display
- **File:** `crates/gruid-core/src/messages.rs`
- `Key::is_rune()` — true if `Key::Char(_)`
- `Key::in_keys(&[Key])` — membership test
- `Display` for `ModMask` — e.g. "Ctrl+Shift"
- `Display` for `MouseAction`

### P5. StyledText missing methods
- **File:** `crates/gruid-ui/src/styled_text.rs`
- `with_textf!()` or `with_textf(format, args)` — format string variant
- `with(text, style)` — set both at once
- Verify `@r` markup prefix system matches Go's exact two-character syntax
  (`@` followed by markup rune, `@@` for literal `@`, `@N` resets to default)

### P6. Label auto-sizing
- **File:** `crates/gruid-ui/src/label.rs`
- Go's `Label.Draw` auto-sizes the grid slice from content dimensions.
  Rust should match: compute content size, optionally add box borders,
  optionally shrink width.

### P7. Neighbors::diagonal()
- **File:** `crates/gruid-paths/src/neighbors.rs`
- Add `diagonal()` method returning only the 4 inter-cardinal neighbors.
- **Ref:** Go `paths/neighbors.go` `Diagonal` method

### P8. Serde derives on remaining types
- Add `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` to:
  - `EventQueue<E>` (where `E: Serialize`)
  - `rl::Grid`
  - `FOV`
  - `PathRange`
  - `Frame` (already has it) — verify `time` field is handled

### P9. RandomWalkCave algorithm fidelity
- **File:** `crates/gruid-rl/src/mapgen.rs`
- Go starts each walk from a **random** position; Rust always starts from center.
- Go's walk has `outDigs` escape logic for out-of-range wandering; Rust doesn't.
- Port the Go logic more faithfully.
- **Ref:** Go `rl/mapgen.go` `RandomWalkCave`

### P10. Msg extensibility
- **File:** `crates/gruid-core/src/messages.rs`
- Go's `Msg` is `interface{}` — users can define custom message types.
  Rust's `Msg` is a closed enum. Consider adding `Msg::Custom(Box<dyn Any + Send>)`
  variant to allow user-defined messages.

---

## 🔵 Enhancement: Beyond the Go Original

These are improvements for the "modern take" that go beyond a 1:1 port.

### E1. Async effect processing
- Replace `std::thread::spawn` with optional `tokio` or `smol` runtime
  for `Effect::Sub` — gated behind a feature flag.

### E2. WASM driver
- New `gruid-web` crate using `wasm-bindgen` + `web-sys` Canvas 2D.
- Port of Go's `gruid-js` but targeting `wasm-pack`.

### E3. GPU-accelerated driver
- New `gruid-wgpu` crate using `wgpu` for GPU tile rendering.
- Would replace softbuffer for better performance at high resolutions.

### E4. Comprehensive test suite
- Port Go's test files (currently ~2,000 lines of tests).
- Add property-based tests for pathfinding (verify JPS == A* path lengths).
- Add fuzzing for mapgen, FOV symmetry checks.

### E5. Documentation & examples
- Crate-level rustdoc with examples for each module.
- Additional examples: menu demo, pager demo, pathfinding visualizer.

---

## Dependency Graph

```
C1 (grid coords) ──→ C3 (FOV) ──→ M3 (multi-source FOV)
       │                │
       ├──→ C4 (countWalls) ──→ M1 (vaults)
       │                        │
       └──→ M6-M8 (UI widgets)  └──→ M2 (KeepCC)

C2 (JPS 4-way) ── standalone fix

M9 (Sub effects) ── standalone fix
M10 (recording) ──→ M5 (replay widget)
```

**Recommended order:** C1 → C4 → C3 → C2 → M9 → M10 → M1 → M2 → M3-M4 → M5 → M6-M8 → P1-P10
