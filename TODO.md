# gruid-rs TODO — Full Port Gap Analysis

Comprehensive task list for completing the Rust port of [gruid](https://codeberg.org/anaseto/gruid).
The Go original lives at `/home/exedev/gruid/` for reference.

Current state: ~9,200 LOC across 7 crates, 85 tests passing.
All P0 (critical bugs) and P1 (major features) are complete.
Remaining: P2 (minor methods/polish) and enhancements.

---

## ✅ Completed: Critical Bugs (P0)

All P0 items are resolved.

### ~~C1. Grid coordinate system — relative vs absolute~~ ✅
- Both gruid-core::Grid and gruid-rl::Grid now use relative coordinates
- `slice()` takes relative range, `at()`/`set()`/`contains()` translate internally
- All UI widgets updated for relative coords

### ~~C2. JPS 4-way (no-diags) mode~~ ✅
- Complete rewrite porting Go's jps.go faithfully
- Fixed: forced-neighbor detection, diagonal handler, dirnorm, path interpolation
- 5 new tests (8-way, 4-way, around walls, manhattan, no-path)

### ~~C3. FOV algorithm divergence~~ ✅
- VisionMap: ported Go's octant-parent ray propagation
- Lighter trait: now `cost(src, from, to)` + `max_cost(src)` matching Go
- SSC: ported Go's algorithm with `diags` parameter
- Added: From, Ray, LightMap, SSCLightMap

### ~~C4. Cellular automata countWalls off-by-one~~ ✅
- countWalls now includes center cell, uses Range intersection
- RandomWalkCave improved: random start positions, outDigs escape logic

---

## ✅ Completed: Major Features (P1)

All P1 items are resolved.

### ~~M1. Vault system (rl/mapgen)~~ ✅
- New `crates/gruid-rl/src/vault.rs` with full Vault type
- parse, iter, draw, reflect, rotate (90/180/270°)
- 9 tests

### ~~M2. `KeepCC` — ensure cave connectivity~~ ✅
- `MapGen::keep_connected()` fills unreachable cells with walls
- Uses PathRange CC labels
- 1 test

### M3. Multi-source FOV lighting — DONE (included in C3 fix)
- `FOV::light_map()` and `FOV::ssc_light_map()` are implemented.

### M4. FOV ray traceback — DONE (included in C3 fix)
- `FOV::from()` and `FOV::ray()` are implemented.

### ~~M5. Replay widget~~ ✅
- New `crates/gruid-ui/src/replay.rs`
- Auto-play with speed control (1x-64x), pause, frame stepping, seeking
- Undo stack for backward navigation
- 3 tests

### ~~M6. Menu widget — complete implementation~~ ✅
- Mouse support (hover/click), pagination, disabled-entry skip
- set_entries(), set_box(), active_bounds(), bounds()
- 7 tests
- Note: Multi-column/table layout not yet implemented

### ~~M7. Pager widget — complete implementation~~ ✅
- Horizontal scroll, half-page nav, top/bottom jump, mouse wheel
- set_lines(), set_cursor(), set_box(), view(), action() getters
- 6 tests

### ~~M8. TextInput widget — complete implementation~~ ✅
- Prompt text, mouse click-to-position, cursor rendering
- set_cursor(), set_box(), set_prompt(), action() getters
- 6 tests

### ~~M9. Sub effects — background thread spawning~~ ✅
- Effect::Cmd and Effect::Sub spawn background threads
- AppRunner::process_pending_msgs() for event-loop drivers
- Winit driver updated to call process_pending_msgs()

### ~~M10. Frame recording — real serialization~~ ✅
- Compact binary encoder/decoder (length-prefixed frames)
- Frame.time_ms field for replay timing
- 4 tests (round-trip for empty, styled, multiple, Unicode)

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
- Core algorithm now matches Go (random start, outDigs).
- Minor: Go's walk has `AtU` (unchecked) for perf; Rust uses checked access.
- Low priority; consider adding `AtU` to rl::Grid first (P3).

### P10. Msg extensibility
- **File:** `crates/gruid-core/src/messages.rs`
- Go's `Msg` is `interface{}` — users can define custom message types.
  Rust's `Msg` is a closed enum. Consider adding `Msg::Custom(Box<dyn Any + Send>)`
  variant to allow user-defined messages.

---

## 🟦 Enhancement: Beyond the Go Original

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

## Recommended Next Steps

All P0 and P1 tasks are complete. Suggested priority for remaining work:

1. **P2 items** — Range methods, Grid Resize, Display impls, serde derives
2. **E4** — Port Go test suite, property-based tests for pathfinding
3. **E5** — Crate-level rustdoc, additional examples
4. **E2/E3** — WASM or GPU drivers (new crate work)
