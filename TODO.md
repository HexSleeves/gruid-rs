# gruid-rs TODO

9 crates, ~18,700 LOC, 219 tests. ~99% Go API parity.

---

## ✅ Completed

### Framework (all P0/P1/P2 closed)
- ✅ Core: Grid, Cell, Point, Range, Style, Msg, Model, Driver, AppRunner, FrameEncoder
- ✅ Paths: A*, Dijkstra, BFS, JPS (4+8 way), Connected Components, PathRange
- ✅ RL: FOV (ray + SSC), MapGen (cellular automata + random walk), Vault, EventQueue
- ✅ UI: Menu, Pager, TextInput, Label, BoxDecor, StyledText, Replay
- ✅ Crossterm: poll-based terminal Driver
- ✅ Winit: event-loop graphical Driver (softbuffer + fontdue)
- ✅ Wgpu: GPU-accelerated graphical Driver (instanced quads + glyph atlas)
- ✅ Web: WASM browser Driver (excluded from workspace, wasm32-only)
- ✅ Serde: opt-in on all key types
- ✅ TileManager trait in gruid-core, re-exported by winit + wgpu

### Shamogu (Phase 1–3 MVP)
- ✅ Map generation (cellular automata + vaults + tunnels)
- ✅ Player + FOV + 8-dir movement + vi keys + mouse
- ✅ 14/27 monster types with A* AI
- ✅ Combat (bump-to-attack, HP/ATK/DEF)
- ✅ UI (status bar, message log, help pager)
- ✅ Crossterm + Winit + Wgpu backends
- ✅ 174 custom monochrome tile bitmaps

---

## 🟦 Active: Shamogu port continuation

See README.md Shamogu section for detailed remaining features.

### Phase 4: Items + Spirits + Inventory
- 🔲 Monster traits bitfield (40 traits)
- 🔲 Status effects (19 statuses with durations)
- 🔲 Spirits (~20 totemic spirits with abilities)
- 🔲 Comestibles (7 consumable items)
- 🔲 Inventory (3 spirit slots + 5 item slots, equip/use menus)

### Phase 5: Advanced combat + terrain
- 🔲 Ranged attacks, special abilities, knockback
- 🔲 Clouds (steam, fire, poison with propagation)
- 🔲 Runic traps (5 types)
- 🔲 Terrain: foliage, rubble, translucent walls
- 🔲 13 remaining monster types

### Phase 6: World + progression
- 🔲 10 dungeon levels with stairs
- 🔲 Noise propagation system
- 🔲 Auto-explore + auto-travel
- 🔲 Animations + visual effects

### Phase 7: Polish
- 🔲 Save/load (serde)
- 🔲 Character dump
- 🔲 Game-over screen

---

## 🟦 Enhancements

### E2. Port Go test suite
- Go has 3,124 lines of tests not yet ported
- Biggest gaps: Grid slice edge cases (820 lines), StyledText (327 lines)

### E4. Typed errors
- Replace `Box<dyn Error>` with per-crate error enums
