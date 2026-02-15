# gruid-rs TODO — Full Port Gap Analysis

Comprehensive task list for completing the Rust port of [gruid](https://codeberg.org/anaseto/gruid).
The Go original lives at `/home/exedev/gruid/` for reference.

Current state: ~7,400 LOC across 7 crates, 48 tests passing.
Estimated remaining: ~1,500 LOC.

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

## 🟡 Major: Missing Features (P1)

Recommended order: M9 → M10 → M1 → M2 → M5 → M6–M8

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

### M3. Multi-source FOV lighting — DONE (included in C3 fix)
- `FOV::light_map()` and `FOV::ssc_light_map()` are implemented.

### M4. FOV ray traceback — DONE (included in C3 fix)
- `FOV::from()` and `FOV::ray()` are implemented.

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

## Dependency Graph

```
M9 (Sub effects) ── standalone fix
M10 (recording) ──→ M5 (replay widget)

M1 (vaults) ── standalone
M2 (KeepCC) ── depends on gruid-paths CC (already works)

M6-M8 (UI widgets) ── standalone, can be done in parallel
```

**Recommended order:** M9 → M10 → M1 → M2 → M5 → M6–M8 → P1–P10
