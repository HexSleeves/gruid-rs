# gruid-rs TODO — Full Port Gap Analysis

Comprehensive task list for completing the Rust port of [gruid](https://codeberg.org/anaseto/gruid).
The Go original lives at `/home/exedev/gruid/` for reference.

Current state: ~9,800 LOC across 7 crates, 85 tests passing.
All P0 (critical bugs) and P1 (major features) are complete.
Remaining: P2 (minor methods/polish) and enhancements.

---

## ✅ Completed: Critical Bugs (P0)

All P0 items are resolved.

### ~~C1. Grid coordinate system — relative vs absolute~~ ✅
### ~~C2. JPS 4-way (no-diags) mode~~ ✅
### ~~C3. FOV algorithm divergence~~ ✅
### ~~C4. Cellular automata countWalls off-by-one~~ ✅

---

## ✅ Completed: Major Features (P1)

All P1 items are resolved.

### ~~M1. Vault system~~ ✅ — parse, iter, draw, reflect, rotate (9 tests)
### ~~M2. KeepCC~~ ✅ — keep largest connected component (1 test)
### ~~M3. Multi-source FOV lighting~~ ✅ — LightMap, SSCLightMap
### ~~M4. FOV ray traceback~~ ✅ — From, Ray
### ~~M5. Replay widget~~ ✅ — auto-play, speed, pause, seek, undo (3 tests)
### ~~M6. Menu widget~~ ✅ — mouse, pagination, disabled skip (7 tests)
### ~~M7. Pager widget~~ ✅ — h/v scroll, half-page, top/bottom, mouse wheel (6 tests)
### ~~M8. TextInput widget~~ ✅ — prompt, mouse click-to-cursor (6 tests)
### ~~M9. Sub effects~~ ✅ — Cmd/Sub spawn threads, AppRunner::process_pending_msgs()
### ~~M10. Frame recording~~ ✅ — binary encoder/decoder, Frame.time_ms (4 tests)

---

## ✅ Completed: Demo

### ~~D1. Enhanced roguelike demo~~ ✅
- Monsters with A* AI, bump combat, HP
- A* path overlay (`p`), Dijkstra heatmap (`d`)
- Look mode (`x`) with tile/monster inspection
- Mouse click-to-move with A* auto-pathing
- Status bar (HP, position, turn, mode indicators)
- Message log (combat + system)
- Help pager (`?`) with BoxDecor
- Exercises: gruid-core, gruid-paths, gruid-rl, gruid-ui

---

## 🟢 Minor: Missing Methods & Polish (P2)

### P1. Range missing methods
- **File:** `crates/gruid-core/src/geom.rs`
- `sub(Point)` — translate range by subtracting a point
- `add(Point)` — translate range by adding a point
- `in_range(Range)` — whether this range is fully contained in another
- `rel_msg(msg)` — translate mouse positions in a Msg relative to a range

### P2. Grid Resize
- **File:** `crates/gruid-core/src/grid.rs` and `crates/gruid-rl/src/grid.rs`
- `resize(w, h)` — grow or shrink, preserving content
- Go's `Grid.Resize` returns a new grid with copied content

### P3. Grid Display + Iterator improvements
- **File:** `crates/gruid-core/src/grid.rs`
- `Display` impl (renders grid as text)
- `GridIterator` with `set_p()`, `set_cell()`, `reset()` for mutable traversal
- Same for `crates/gruid-rl/src/grid.rs`

### P4. Key/ModMask helpers
- **File:** `crates/gruid-core/src/messages.rs`
- `Key::is_rune()` — true if `Key::Char(_)`
- `Key::in_keys(&[Key])` — membership test
- Better `Display` for `ModMask` — e.g. "Ctrl+Shift" instead of bitfield

### P5. StyledText @r markup
- **File:** `crates/gruid-ui/src/styled_text.rs`
- Verify `@r` markup matches Go's exact syntax
  (`@` + markup rune, `@@` for literal `@`, `@N` resets to default)
- `with_textf(format_string)` convenience

### P6. Label auto-sizing
- **File:** `crates/gruid-ui/src/label.rs`
- Go's `Label.Draw` auto-sizes from content dimensions
- Add box border support, optional width shrink

### P7. Neighbors::diagonal()
- **File:** `crates/gruid-paths/src/neighbors.rs`
- Return only the 4 inter-cardinal neighbors

### P8. Serde derives on remaining types
- `EventQueue<E>`, `rl::Grid`, `FOV`, `PathRange`
- Gate behind `serde` feature flag

### P9. rl::Grid unchecked access
- `at_unchecked(p)` for performance-critical inner loops
- Low priority

### P10. Msg extensibility
- `Msg::Custom(Box<dyn Any + Send>)` variant for user-defined messages
- Go's `Msg` is `interface{}` so any type works

### P11. Menu multi-column layout
- `MenuStyle.layout` columns > 1 for table-style menus
- **Ref:** Go `ui/menu.go` table layout

---

## 🟦 Enhancement: Beyond the Go Original

### E1. Async effect processing
- Optional `tokio`/`smol` runtime for `Effect::Sub` behind a feature flag

### E2. WASM driver
- New `gruid-web` crate using `wasm-bindgen` + `web-sys` Canvas 2D
- Port of Go's `gruid-js` targeting `wasm-pack`

### E3. GPU-accelerated driver
- New `gruid-wgpu` crate using `wgpu` for GPU tile rendering

### E4. Comprehensive test suite
- Port Go's test files (~2,000 lines)
- Property-based tests: verify JPS == A* path lengths
- Fuzzing for mapgen, FOV symmetry checks

### E5. Documentation & examples
- Crate-level rustdoc with examples for each module
- Standalone examples: menu demo, pathfinding visualizer, replay player

### E6. Error types
- Replace `Box<dyn Error>` with typed errors per crate
- Consider `thiserror` or manual impls

---

## Recommended Next Steps

All P0 and P1 tasks are complete. Suggested priority:

1. **P2/P4** — `Key::in_keys()`, `Range::add/sub` (small, high utility)
2. **P2** — Grid Resize (needed for window resize correctness)
3. **P5** — StyledText @r markup completion
4. **P8** — Serde derives (enables save/load)
5. **E4** — Port Go test suite, property-based tests
6. **E5** — Rustdoc, more examples
7. **E2** — WASM driver (biggest impact for reach)
