# gruid-rs TODO

Prioritized task list. See `CONTEXT.md` for architecture details.

Current: 13,500 LOC, 8 crates, 204 tests, ~94% Go API parity.

---

## ✅ All P0 Blockers — DONE

1. ✅ StyledText `@`-prefix markup protocol
2. ✅ Menu 2D grid layout + ActiveInvokable
3. ✅ Range relative Line/Lines/Column/Columns
4. ✅ Range Add/Sub translation + RelMsg
5. ✅ Grid Resize (core + rl)
6. ✅ rl::Grid mutable iteration (for_each_mut, map_cells_mut, at_unchecked)
7. ✅ FOV from() bug fixed
8. ✅ Serde on all key types (PathRange, EventQueue, rl::Grid, FOV)

## ✅ All P1 Major Features — DONE

- ✅ Pager: lines(), set_cursor(Point), start key, 8-col scroll, mouse, view()->Range
- ✅ Menu: mouse (click/wheel/hover/outside-quit), page numbers in footer
- ✅ Label: background fill, AdjustWidth
- ✅ BoxDecor: markup-aware title/footer
- ✅ Neighbors: diagonal()
- ✅ StyledText lines() markup state preservation
- ✅ Grid Display, points() iterator
- ✅ Range PartialEq empty normalization, In containment
- ✅ MapGen::with_grid()
- ✅ WASM driver (gruid-web)

---

## 🟡 Remaining Small Gaps (6 items)

### R1. Replay widget polish
- **File:** `crates/gruid-ui/src/replay.rs`
- Help overlay (embedded Pager showing keybindings)
- Mouse interaction (click to toggle pause, etc.)
- Grid auto-resize when frames are larger than current grid
- `ReplayKeys.help` field
- **Effort:** ~2 hours

### R2. Pager line number in footer
- **File:** `crates/gruid-ui/src/pager.rs`
- Go shows "Line X/Y" in box footer when scrolling
- **Effort:** ~30 min

### R3. TextInput cursor auto-reverse style
- **File:** `crates/gruid-ui/src/text_input.rs`
- Go auto-swaps fg/bg for cursor when no explicit cursor style set
- **Effort:** ~15 min

### R4. ModMask Display combos
- **File:** `crates/gruid-core/src/messages.rs`
- Currently shows "SHIFT" or "CTRL" individually
- Go shows "Ctrl+Shift" for combined modifiers
- **Effort:** ~15 min

### R5. StyledText convenience constructors
- **File:** `crates/gruid-ui/src/styled_text.rs`
- `with_textf(String)` — pre-formatted text
- `with(text, style)` — combined text+style
- **Effort:** ~15 min

### R6. PathRange/JPS performance
- **Files:** `crates/gruid-paths/src/pathrange.rs`, `jps.rs`
- PathRange::set_range should preserve caches when new size ≤ old capacity
- JPS should accept `&mut Vec<Point>` for buffer reuse
- **Effort:** ~1 hour

---

## 🟦 Enhancements (beyond Go original)

### E1. Port shamogu ← NEXT STEP
- Port https://codeberg.org/anaseto/shamogu to Rust using gruid-rs
- This proves the framework and surfaces any remaining gaps
- Start with: model struct, update loop, draw, map generation
- Then: monsters, combat, items, animations, menus

### E2. Port Go test suite
- Go has 3,124 lines of tests we haven't ported
- Biggest gaps: Grid slice edge cases (820 lines), StyledText (327 lines)
- Would give high confidence in correctness

### E3. GPU-accelerated driver
- `gruid-wgpu` crate using wgpu for GPU tile rendering

### E4. Typed errors
- Replace `Box<dyn Error>` with per-crate error types

### E5. Documentation
- Crate-level rustdoc with examples for each module
- Standalone examples: menu demo, pathfinding visualizer
