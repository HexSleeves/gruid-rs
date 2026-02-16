# gruid-rs TODO

Prioritized task list. See `CONTEXT.md` for architecture details.

Current: ~14,225 LOC, 8 crates, 228 tests, ~99% Go API parity.

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

- ✅ Pager: lines(), set_cursor(Point), start key, 8-col scroll, mouse, view()->Range, line-number footer
- ✅ Menu: mouse (click/wheel/hover/outside-quit), page numbers in footer
- ✅ Label: background fill, AdjustWidth
- ✅ BoxDecor: markup-aware title/footer
- ✅ Neighbors: diagonal()
- ✅ StyledText lines() markup state preservation
- ✅ Grid Display, points() iterator
- ✅ Range PartialEq empty normalization, In containment
- ✅ MapGen::with_grid()
- ✅ WASM driver (gruid-web)

## ✅ All P2 Polish — DONE

- ✅ R1: Replay widget (help overlay, mouse, grid auto-resize)
- ✅ R2: Pager line number in footer
- ✅ R3: TextInput cursor auto-reverse style
- ✅ R4: ModMask Display combos ("Ctrl+Shift")
- ✅ R5: StyledText with_textf/with convenience constructors
- ✅ R6: PathRange capacity-preserving set_range + JPS jps_path_into buffer reuse

---

## 🟦 Active: Port shamogu (E1)

Port https://codeberg.org/anaseto/shamogu to Rust using gruid-rs.
This proves the framework and surfaces any remaining gaps.
See `SHAMOGU_PORT_PROMPT.md` for the full handoff plan.

### Phases
1. ✅ Clone & study Go shamogu (~17k LOC, ~41 files)
2. ✅ Scaffold: new binary crate `shamogu/`, model struct, main
3. ✅ Map generation: cellular automata + vaults + tunnels + keep_connected
4. ✅ Player + FOV: movement, vision_map + ssc_vision_map, draw loop
5. ✅ Monsters + pathfinding: entity system, A* chase AI, wandering
6. ✅ Combat: bump-to-attack with probability tables, death handling
7. ✅ UI: status bar (HP/A/D/Level/Turn), message log (2 lines), help pager
8. 🔲 Items: spirits, comestibles, inventory menu
9. 🔲 Animations: Effect::Cmd timers
10. 🔲 Save/load: serde

### Current State (Phase 1-3 MVP)
- 9 source files, ~1500 LOC
- Playable: generated cave map, player @, 14 monster types, FOV, combat
- `cargo run --bin shamogu`

---

## 🟦 Enhancements (beyond Go original)

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

### E6. Publish to crates.io
- Add proper metadata, README per crate, license files
- Version 0.1.0 initial publish
