# gruid-rs TODO — Full Port Gap Analysis

Comprehensive task list for completing the Rust port of [gruid](https://codeberg.org/anaseto/gruid).
The Go original can be cloned from `https://codeberg.org/anaseto/gruid` for reference.

Current state: ~12,600 LOC across 7 crates, 195 tests passing.
All P0 (critical bugs/blockers) and P1 (major features) are complete.
Remaining: P2 (minor methods/polish) and enhancements.

---

## ✅ Completed: P0 Blockers (Go Parity)

All P0 items resolved in parallel agent batch.

### ~~P0-1. StyledText `@`-prefix markup~~ ✅
Full `@X` switch, `@N` reset, `@@` escape, `\r` stripping, cross-line state preservation.

### ~~P0-2. Menu 2D grid layout~~ ✅
`MenuStyle.layout` with multi-column/row pagination. Mouse support (click/wheel/hover/outside-quit).

### ~~P0-3. Menu ActiveInvokable~~ ✅
`active_invokable()` and `set_active_invokable()` for indexing past disabled entries.

### ~~P0-4. Range relative Line/Lines/Column/Columns~~ ✅
Switched to relative coordinates with intersection clamping, matching Go.

### ~~P0-5. Range Add/Sub translation~~ ✅
`Range::add(p)` / `Range::sub(p)` + `Add<Point>`/`Sub<Point>` trait impls.

### ~~P0-6. Range RelMsg~~ ✅
`Range::rel_msg(msg)` adjusts mouse positions relative to sub-grid.

### ~~P0-7. Grid Resize~~ ✅
Both `gruid-core::Grid` and `gruid-rl::Grid`. Content-preserving resize.

### ~~P0-8. rl::Grid mutable iteration~~ ✅
`for_each_mut()`, `map_cells_mut()`, `at_unchecked()`, `copy_from` returns Point.

---

## ✅ Completed: P1 Important (Feature Parity)

### ~~P1-1. FOV from() bug~~ ✅
Removed double-counting of `lt.cost()` in `from()` method.

### ~~P1-2. Neighbors::diagonal()~~ ✅
4 diagonal neighbors, matching Go order.

### ~~P1-3. Label background fill~~ ✅
Fills area with base style before drawing content.

### ~~P1-4. Label AdjustWidth~~ ✅
Functional — shrinks returned drawing area to content width.

### ~~P1-5. BoxDecor markup title/footer~~ ✅
Uses `StyledText::draw()` for markup-aware rendering.

### ~~P1-6. Pager enhancements~~ ✅
`lines()`, `set_cursor(Point)`, `PagerKeys::start`, 8-col horizontal scroll, `view()->Range`, mouse click page up/down.

### ~~P1-7. Menu enhancements~~ ✅
Page numbers in footer, mouse click outside quit, wheel paging.

### ~~P1-8. Grid Display~~ ✅
`impl Display for Grid` — renders as text for debugging.

### ~~P1-9. Grid points() iterator~~ ✅
Convenience `points()` method for Point-only iteration.

### ~~P1-10. Range shift empty safety~~ ✅
Returns empty range when result would be empty.

### ~~P1-11. Range PartialEq empty normalization~~ ✅
All empty ranges compare equal.

### ~~P1-12. Range In(r) containment~~ ✅
Check if range is fully within another.

---

## ✅ Completed: Earlier Work

### Critical Bugs (all resolved)
- Grid coordinate system — relative vs absolute ✅
- JPS 4-way (no-diags) mode ✅
- FOV algorithm divergence ✅
- Cellular automata countWalls off-by-one ✅

### Major Features (all resolved)
- Vault system ✅
- KeepCC ✅
- Multi-source FOV lighting ✅
- FOV ray traceback ✅
- Replay widget ✅
- Menu widget ✅
- Pager widget ✅
- TextInput widget ✅
- Sub effects ✅
- Frame recording ✅
- Enhanced roguelike demo ✅

---

## 🟡 Remaining P1 (3 items)

### P1-A. StyledText `lines()` markup state preservation
- **File:** `crates/gruid-ui/src/styled_text.rs`
- Go preserves inter-line markup state with `@r` prefix on continuation lines.
- **Status:** Implemented but needs thorough verification against Go edge cases.

### P1-B. Replay missing features
- **File:** `crates/gruid-ui/src/replay.rs`
- Help overlay (embedded Pager)
- Mouse interaction (toggle pause, step)
- Grid auto-resize on larger frames

### P1-C. Serde derives on remaining types
- `EventQueue<E>`, `rl::Grid`, `FOV`, `PathRange`
- Gate behind `serde` feature flag
- Required for save/load in real games

---

## 🟢 Remaining P2 (minor polish)

### P2-1. Key/ModMask helpers
- `Key::in_keys(&[Key])` — membership test
- Better `Display` for `ModMask` — "Ctrl+Shift" combos

### P2-2. StyledText convenience
- `with_textf(format_string)` — formatted text constructor
- `with(text, style)` — combined text+style setter

### P2-3. PathRange capacity optimization
- Preserve caches when new size ≤ old capacity (Go behavior)

### P2-4. JPS path buffer reuse
- Accept pre-allocated `&mut Vec<Point>` for zero-allocation reuse

### P2-5. MapGen::with_grid()
- Create derived MapGen sharing a grid

### P2-6. TextInput cursor auto-reverse
- Auto-swap fg/bg for cursor style when no explicit style set

### P2-7. App::CatchPanics
- Configurable panic recovery flag

### P2-8. `\r` handling in StyledText
- Strip carriage returns (partially done, verify completeness)

---

## 🟦 Enhancement: Beyond Go Original

### E1. WASM driver
- New `gruid-web` crate using `wasm-bindgen` + Canvas 2D

### E2. GPU-accelerated driver
- New `gruid-wgpu` crate using `wgpu`

### E3. Async effect processing
- Optional `tokio`/`smol` runtime behind feature flag

### E4. Comprehensive test suite
- Port remaining Go test cases (~2,000 lines)
- Property-based tests, fuzzing

### E5. Documentation & examples
- Crate-level rustdoc with examples
- Standalone examples: menu demo, pathfinding visualizer

### E6. Typed errors
- Replace `Box<dyn Error>` with per-crate error types

---

## Recommended Next Steps

1. **P1-C** — Serde derives (enables save/load for game ports)
2. **P1-B** — Replay polish (help, mouse, auto-resize)
3. **E4** — Port Go test suite for full confidence
4. **E1** — WASM driver (biggest impact for reach)
5. **P2** — Minor polish as needed during game porting
