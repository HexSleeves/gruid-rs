# gruid-rs Gap Analysis: What's Needed to Match Go gruid

Complete comparison of Go [gruid](https://codeberg.org/anaseto/gruid) v0.25.0 against gruid-rs.
Derived from line-by-line API audit across all packages.

Legend: 🔴 = blocker for porting shamogu, 🟡 = important, 🟢 = nice-to-have

---

## P0 — Blockers (must fix to port a real game like shamogu)

### 1. 🔴 StyledText Markup Protocol Mismatch
**Crate:** `gruid-ui` · **Files:** `styled_text.rs`

Go uses a `@`-prefix markup system: `@r` switches to markup `r`, `@@` produces literal `@`, `@N` resets to default style. Rust uses single marker chars as style switches with no escape mechanism and no reset.

Shamogu uses `@` markup everywhere — log messages, menus, descriptions, status bar. This is **the** highest-impact gap.

**Fix:** Rewrite `StyledText::iter()` / `StyledText::draw()` to implement the `@`-prefix protocol.

### 2. 🔴 Menu 2D Grid Layout
**Crate:** `gruid-ui` · **Files:** `menu.rs`

Go Menu supports `MenuStyle.Layout` as a `Point{X,Y}` where X=columns and Y=rows-per-page, enabling table layouts. Rust Menu is a flat 1D vertical list only.

Shamogu uses 2D menus for inventory, spirit selection, status bar.

**Fix:** Implement `layout` field with multi-column/row pagination logic.

### 3. 🔴 Menu `ActiveInvokable` / `SetActiveInvokable`
**Crate:** `gruid-ui` · **Files:** `menu.rs`

Go can index entries skipping disabled ones. Shamogu calls `SetActiveInvokable(0)` in spirit selection, inventory, etc.

**Fix:** Add `active_invokable() -> usize` and `set_active_invokable(i: usize)`.

### 4. 🔴 Range `Line`/`Lines`/`Column`/`Columns` — Relative Coordinates
**Crate:** `gruid-core` · **Files:** `geom.rs`

Go `Range.Line(y)` uses *relative* y (0 = first line of the range) with OOB → empty-range safety. Rust uses *absolute* coordinates with no bounds check.

This matters everywhere slicing grids for UI layout: log area, map area, status bar.

**Fix:** Change to relative indexing with intersection/clamping.

### 5. 🔴 Range `Add(p)` / `Sub(p)` — Translation
**Crate:** `gruid-core` · **Files:** `geom.rs`

Go `Range.Add(p)` translates the entire range by a point. Used extensively for positioning UI elements.

**Fix:** Add `Range::add(p: Point) -> Range` and `Range::sub(p: Point) -> Range`.

### 6. 🔴 Range `RelMsg(msg)` — Make Mouse Positions Relative
**Crate:** `gruid-core` · **Files:** `geom.rs`

Go `Range.RelMsg(msg)` adjusts mouse coordinates to be relative to a sub-grid. Every widget uses this to translate mouse input into local coordinates.

**Fix:** Add `Range::rel_msg(msg: Msg) -> Msg`.

### 7. 🔴 Grid `Resize(w, h)`
**Crate:** `gruid-core` · **Files:** `grid.rs`

Go `Grid.Resize(w, h)` grows the underlying buffer, preserving existing content. Used for dynamic UI (terminal resize, pager content changes).

**Fix:** Add `Grid::resize(w: i32, h: i32)`.

### 8. 🔴 rl::Grid Mutable Iterator / `SetCell`
**Crate:** `gruid-rl` · **Files:** `grid.rs`

Go `GridIterator.SetCell(c)` mutates cells during iteration. Critical for map generation and FOV loops.

Rust's `iter()` returns a snapshot — no mutation.

**Fix:** Add `iter_mut()` returning `&mut Cell` or a `for_each_mut(FnMut(Point, &mut Cell))` method.

---

## P1 — Important (needed for full feature parity)

### 9. 🟡 Pager Missing Features
**Crate:** `gruid-ui` · **Files:** `pager.rs`

- `Pager.Lines()` — get total line count
- `Pager.SetCursor(Point)` — set both x and y (Rust only sets y)
- `PagerKeys.Start` — go to column 0
- Line number display in box footer
- Mouse click top/bottom half → page up/down
- Horizontal scroll by 8 columns (Go) vs 1 (Rust)

### 10. 🟡 Menu Missing Features
**Crate:** `gruid-ui` · **Files:** `menu.rs`

- Page number display in box footer
- Mouse: click outside → Quit
- Mouse: WheelUp/WheelDown for page navigation
- Multi-page X,Y tracking (Go tracks 2D page position)

### 11. 🟡 Label `AdjustWidth` Not Functional
**Crate:** `gruid-ui` · **Files:** `label.rs`

Field exists but isn't used in `draw()`. Go uses it to auto-shrink the label grid to content width.

### 12. 🟡 Label Background Fill
**Crate:** `gruid-ui` · **Files:** `label.rs`

Go fills the label area with the style's background before drawing text. Rust doesn't.

### 13. 🟡 BoxDecor Title/Footer Markup Styles
**Crate:** `gruid-ui` · **Files:** `box_.rs`

Rust draws title/footer char-by-char ignoring StyledText markup styles. Go applies markup.

### 14. 🟡 FOV `from()` Behavioral Bug
**Crate:** `gruid-rl` · **Files:** `fov.rs`

Rust adds an extra `lt.cost()` to the return value that Go doesn't. This would cause incorrect FOV cost calculations.

### 15. 🟡 Neighbors `diagonal()` 
**Crate:** `gruid-paths` · **Files:** `neighbors.rs`

Go has `Neighbors.Diagonal()` returning 4 diagonal neighbors. Rust only has `all()` (8) and `cardinal()` (4).

### 16. 🟡 Replay Missing Features
**Crate:** `gruid-ui` · **Files:** `replay.rs`

- Help overlay (embedded Pager)
- Mouse interaction (toggle pause, step)
- Grid auto-resize on larger frames
- `ReplayKeys.Help`

### 17. 🟡 StyledText `Lines()` Markup State
**Crate:** `gruid-ui` · **Files:** `styled_text.rs`

Go preserves inter-line markup state (prefixes continuation lines with active `@r` marker). Rust does not.

### 18. 🟡 `serde` Support for Pathfinding/FOV/Grid
**Crate:** `gruid-paths`, `gruid-rl`

Go has `GobEncode`/`GobDecode` on `PathRange`, `FOV`, `Grid`, `EventQueue`. Rust has none. Needed for save/load.

---

## P2 — Nice to Have (completeness, polish)

### 19. 🟢 Grid `String()` — Debug Display
**Crate:** `gruid-core` · Textual representation of grid runes for debugging.

### 20. 🟢 Range `In(r)` — Containment Check
**Crate:** `gruid-core` · Check if range is fully within another.

### 21. 🟢 Range `Eq` Empty Normalization  
**Crate:** `gruid-core` · Go treats all empty ranges as equal.

### 22. 🟢 rl::Grid `AtU(p)` — Unchecked Access
**Crate:** `gruid-rl` · Performance-critical unchecked cell access.

### 23. 🟢 rl::Grid `Resize(w, h)`
**Crate:** `gruid-rl` · Same as core Grid resize.

### 24. 🟢 PathRange `SetRange` Capacity Optimization
**Crate:** `gruid-paths` · Go preserves caches when new size ≤ old capacity. Rust always reallocates.

### 25. 🟢 JPS Path Buffer Reuse
**Crate:** `gruid-paths` · Go `JPS` accepts pre-allocated `path []Point`. Rust allocates new Vec each call.

### 26. 🟢 `MapGen::with_grid()`
**Crate:** `gruid-rl` · Create derived MapGen sharing a grid.

### 27. 🟢 TextInput Cursor Auto-Reverse Style
**Crate:** `gruid-ui` · Go auto-swaps fg/bg for cursor if no style set.

### 28. 🟢 `ModMask` Display Combos
**Crate:** `gruid-core` · Go shows "Ctrl+Shift"; Rust doesn't combine.

### 29. 🟢 `App::CatchPanics`
**Crate:** `gruid-core` · Rust uses `catch_unwind` differently; Go has an explicit flag.

### 30. 🟢 `StyledText::with_textf()` / `StyledText::with(text, style)`
**Crate:** `gruid-ui` · Convenience constructors.

### 31. 🟢 `\r` Carriage Return Handling in StyledText
**Crate:** `gruid-ui` · Go strips `\r`; Rust doesn't.

---

## Testing Gap

Go gruid has **3,124 lines** of tests across 14 test files. Rust has **87 tests** total (24 core, 33 paths, 23 rl, 5 ui, 2 ignored). Major untested areas:

- Grid slice semantics and edge cases (Go: 820 lines of grid tests)
- StyledText markup parsing (Go: 327 lines)
- Menu update/draw behavior (Go: 140 lines)  
- Pager behavior (Go: 77 lines)
- rl::Grid operations (Go: 398 lines)

---

## Summary Counts

| Priority | Count | Description |
|----------|-------|-------------|
| 🔴 P0 | 8 | Blockers — must fix for real game port |
| 🟡 P1 | 10 | Important — full feature parity |
| 🟢 P2 | 13 | Nice to have — completeness/polish |
| **Total** | **31** | |

## Recommended Order

1. **StyledText markup** (#1) — everything renders through this
2. **Range relative coords + Add/Sub/RelMsg** (#4, #5, #6) — UI layout depends on it
3. **Grid Resize** (#7) — terminal resize support
4. **Menu 2D layout + ActiveInvokable** (#2, #3) — game menus
5. **FOV bug fix** (#14) — correctness
6. **rl::Grid mutable iteration** (#8) — map generation
7. **Pager/Menu/Label polish** (#9-13) — UI completeness
8. **serde support** (#18) — save/load
9. **Everything else** — as needed during port
