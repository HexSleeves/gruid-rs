# Shamogu Port — Agent Handoff Prompt

You are porting **shamogu** (a Go roguelike game) to Rust, building on the **gruid-rs** framework.

---

## Context

### gruid-rs (the framework)
- **Location:** `/home/exedev/gruid-rs`
- **Repo:** `https://github.com/HexSleeves/gruid-rs.git`, branch `main`, HEAD `9865c19`
- **Stats:** 14,225 LOC, 8 crates, 228 tests, ~99% Go API parity with gruid v0.25.0
- **Docs:** `CONTEXT.md` has full API reference, `AGENTS.md` has coding standards
- **Rust edition:** 2024 (requires 1.85+)

### shamogu (the game to port)
- **Source:** `https://codeberg.org/anaseto/shamogu` — clone fresh (not on disk)
- **Stats:** ~17,200 LOC Go across 41 files, uses gruid v0.25.0
- **What it is:** A turn-based roguelike with cave levels, spirit abilities, monsters with AI, items, status effects, animations, save/load, multiple UI modes

### Disk constraints
- `/dev/root` is 19G, ~96% used. Run `cargo clean` after builds. Delete Go clone after studying.

---

## Goal

Create `shamogu-rs` as a new binary crate inside the gruid-rs workspace at `/home/exedev/gruid-rs/shamogu/`. It should:
1. Compile and run in the terminal via `cargo run --bin shamogu`
2. Use gruid-rs crates (gruid-core, gruid-paths, gruid-rl, gruid-ui, gruid-crossterm)
3. Be a playable roguelike that demonstrates the framework

---

## Shamogu Architecture (Go)

The Go game has these major components:

### Core State (`game.go`, 568 LOC)
```go
type Game struct {
    Entities []*Entity        // all entities (inventory slots + map entities)
    Map      *Map             // current level map
    PR       *paths.PathRange // pathfinding cache
    Turn     int
    Logs     *Logs
    Stats    *Stats
    rand     *rand.Rand
}
```
- Entity IDs: 0..InventorySize-1 are inventory, FirstMapID onward are map entities
- PlayerID = FirstMapID (always the first map entity)

### Entity System (`entities.go`, 568 LOC)
```go
type Entity struct {
    Name   string
    Rune   rune
    P      gruid.Point  // position
    KnownP gruid.Point  // last known position
    Seen   bool
    Role   any          // *Actor, *Item, *Comestible, etc.
}
```
- Uses `any` (interface{}) for polymorphic roles — in Rust, use an enum:
```rust
enum EntityRole {
    Actor(Actor),
    Spirit(Spirit),
    Comestible(Comestible),
    // etc.
}
```

### Actor/Combat (`actor.go` 1054 LOC, `combat.go` 890 LOC)
- Actor has HP, MaxHP, Attack, Defense, Statuses, Traits, Behavior
- ~20 status effects (StatusConfused, StatusBerserk, StatusFire, etc.)
- Traits are bitflags (~50 traits)
- Behavior has State (Wandering/Hunting/Resting) for monster AI
- Combat: bump-to-attack, damage = attack - defense + dice, death handling

### Map (`map.go` 249 LOC, `mapgen.go` 912 LOC)
- Terrain stored as `rl.Grid` (Cell = i32): Wall=0, Floor=1, Foliage=2, Rubble=3, TranslucentWall=4
- Map has Terrain, KnownTerrain, FOV, Clouds, Noise sources, Waypoints
- Generation: cellular automata + random walk, vault placement, tunnel carving, entity/item spawning
- `keep_connected` ensures walkability

### FOV (`fov.go`, 358 LOC)
- Uses both VisionMap (ray-based, for lighting costs through foliage) and SSCVisionMap (for boolean visibility)
- MaxFOVRange = 8, adjustable per traits
- Lighter trait impl considers terrain and flying status

### Pathfinding (`paths.go`, 248 LOC)
- Two PathRanges: `PR` (general) and `PRnoise` (sound propagation)
- A* for monster chase, Dijkstra for auto-explore, BFS for noise
- Custom Pather impls for different movement rules (flying, digging, etc.)

### Model/UI (`model.go` 312 LOC, `update.go` 402 LOC, `draw.go` 635 LOC)
- Model has modes: Normal, Pager, Menu, NewGame, End, etc.
- Update dispatches by mode, returns Effects
- Draw: log (2 lines top), map (middle), status bar (bottom), overlays
- Grid layout: 80×24, log at lines 0-1, map at 2-22, status at 23

### Actions (`actions.go`, 1880 LOC)
- Action trait: `Handle(*model) -> (Effect, bool)` where bool = ends turn
- ~40 action types: Wait, Bump, Move, UseItem, Sprint, Examine, etc.
- Actions are created in update, handled separately

### Animation (`animation.go`, 466 LOC)
- Queue of AnimFrames, each with cell changes + duration
- Uses Effect::Cmd with sleep timers for frame timing
- Custom `msgAnim` message type for animation ticks

### Items/Effects (`items.go` 415 LOC, `effects.go` 1315 LOC)
- Spirits (3 slots): abilities with cooldowns
- Comestibles (5 slots): food items with effects
- ~30 status effects with per-turn processing

### Monster Generation (`entgen.go`, 1479 LOC)
- ~25 monster types with stats, traits, behaviors
- Spawn tables per dungeon level
- Item generation tables

### Save/Load (`io.go` 196 LOC, `encoding.go` 160 LOC)
- Uses Go's gob encoding — in Rust, use serde + bincode/json

---

## Porting Strategy

### Phase 1: Scaffold + Map (get something on screen)
1. Add `shamogu/` to workspace members in root `Cargo.toml`
2. Create `shamogu/Cargo.toml` depending on gruid-core, gruid-paths, gruid-rl, gruid-ui, gruid-crossterm, rand
3. Create model struct implementing `Model` trait
4. Port terrain constants (Wall/Floor/Foliage/Rubble as i32 constants)
5. Port basic Map struct with Terrain (rl::Grid) and KnownTerrain
6. Port map generation (start with cellular automata cave, `MapGen::cellular_automata_cave`)
7. Draw the map — wall/floor chars with basic colors
8. Run with `cargo run --bin shamogu`

### Phase 2: Player + FOV
1. Port Entity struct with enum Role
2. Add player entity, movement (arrow keys)
3. Port FOV using `FOV::ssc_vision_map` (simpler than ray-based)
4. Implement Lighter trait for terrain
5. Draw with lit/seen/unseen states
6. Status bar showing position, turn count

### Phase 3: Monsters + Combat
1. Port Actor struct (HP, Attack, Defense, Statuses as bitflags)
2. Port monster kinds enum (~10 to start, not all 25)
3. Monster spawn during mapgen
4. Monster AI: simple A* chase when player visible
5. Bump-to-attack combat with damage formula
6. Death handling
7. Message log (top 2 lines)

### Phase 4: Items + Abilities
1. Port item system (Spirit abilities, Comestibles)
2. Inventory UI using Menu widget
3. Item use/equip actions
4. Status effect processing per turn

### Phase 5: UI Polish
1. Full menu system (game menu, help, settings)
2. Pager for logs/help/lore
3. Targeting mode
4. Auto-explore
5. Animations using Effect::Cmd

### Phase 6: Save/Load + Extras
1. Serde serialization for Game state
2. Save on quit, load on start
3. Multiple dungeon levels
4. Remaining monster types and effects

---

## Key Mapping: Go gruid → Rust gruid-rs

| Go | Rust |
|----|------|
| `gruid.Point{X, Y}` | `Point::new(x, y)` |
| `gruid.NewRange(x0,y0,x1,y1)` | `Range::new(x0, y0, x1, y1)` |
| `gruid.NewGrid(w, h)` | `Grid::new(w, h)` |
| `grid.Slice(rg)` | `grid.slice(rg)` |
| `grid.Fill(cell)` | `grid.fill(cell)` |
| `grid.Set(p, c)` | `grid.set(p, c)` |
| `grid.At(p)` | `grid.at(p)` |
| `grid.Range()` | `grid.range_()` (underscore to avoid keyword) |
| `grid.Size()` | `grid.size()` |
| `rg.Lines(y0, y1)` | `rg.lines(y0, y1)` |
| `rg.Shift(dx0,dy0,dx1,dy1)` | `rg.shift(dx0, dy0, dx1, dy1)` |
| `rg.Add(p)` / `rg.Sub(p)` | `rg + p` / `rg - p` (or `rg.add(p)`) |
| `rg.Intersect(r2)` | `rg.intersect(r2)` |
| `p.In(rg)` | `rg.contains(p)` |
| `p.Add(q)` | `p + q` |
| `p.Sub(q)` | `p - q` |
| `p.Shift(dx, dy)` | `p.shift(dx, dy)` |
| `rl.NewGrid(w, h)` | `rl::Grid::new(w, h)` |
| `rl.Cell(n)` | `n as i32` (rl::Cell is i32) |
| `gruid.Cell{Rune: r, Style: s}` | `Cell::default().with_char(r).with_style(s)` |
| `gruid.Style{Fg: c, Bg: b}` | `Style { fg: c, bg: b, attrs: AttrMask::default() }` or builder |
| `gruid.Color(n)` | `Color::from_rgb(r, g, b)` |
| `gruid.MsgKeyDown{Key: k}` | `Msg::KeyDown { key: k, .. }` |
| `gruid.MsgMouse{Action: a, P: p}` | `Msg::Mouse { action: a, pos: p, .. }` |
| `gruid.End()` | `Effect::End` |
| `gruid.Batch(e1, e2)` | `Effect::Batch(vec![e1, e2])` |
| `key.In(keys)` | `keys.contains(&key)` |
| `paths.NewPathRange(rg)` | `PathRange::new(rg)` |
| `pr.AstarPath(pather, from, to)` | `pr.astar_path(&pather, from, to)` |
| `pr.JPSPath(path, from, to, f, diags)` | `pr.jps_path(from, to, f, diags)` or `jps_path_into(buf, ..)` |
| `pr.DijkstraMap(pather, srcs, max)` | `pr.dijkstra_map(&pather, &srcs, max)` |
| `pr.DijkstraAt(p)` | `pr.dijkstra_at(p)` |
| `fov.VisionMap(lighter, src)` | `fov.vision_map(&lighter, src)` |
| `fov.SSCVisionMap(src, range, pass, diags)` | `fov.ssc_vision_map(&lighter, src, diags)` |
| `ui.NewMenu(cfg)` | `Menu::new(cfg)` |
| `ui.NewPager(cfg)` | `Pager::new(cfg)` |
| `ui.StyledText{}.WithText(s)` | `StyledText::text(s)` |
| `stt.WithMarkup(r, style)` | `stt.with_markup(r, style)` |
| `stt.Format(width)` | `stt.format(width)` |
| `interface{}` for Msg | `Msg::Custom(Arc<dyn Any>)` / `Msg::custom(val)` / `msg.downcast_ref::<T>()` |
| `gob.Encode/Decode` | `serde::Serialize/Deserialize` (behind `serde` feature) |
| `rand.IntN(n)` | `rng.random_range(0..n)` (rand 0.9) |

---

## gruid-rs API Quick Reference

See `CONTEXT.md` section "Public API Quick Reference" for the complete API.

Key patterns:
```rust
// Create app and run
let game = ShamoguModel::new();
let driver = CrosstermDriver::new();
let mut app = App::new(AppConfig {
    model: game,
    driver,
    width: 80,
    height: 24,
    frame_writer: None,
});
app.run().unwrap();

// Model impl
impl Model for ShamoguModel {
    fn update(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::Init => { /* setup */ None }
            Msg::KeyDown { key, .. } => match key {
                Key::ArrowUp => { self.move_player(0, -1); None }
                Key::Escape => Some(Effect::End),
                _ => None,
            },
            _ => None,
        }
    }

    fn draw(&self, grid: &mut Grid) {
        grid.fill(Cell::default());
        // Draw map
        for p in self.map.terrain.range_().iter() {
            let terrain = self.map.terrain.at(p).unwrap_or(0);
            let (ch, fg) = match terrain {
                WALL => ('#', Color::from_rgb(128, 128, 128)),
                FLOOR => ('.', Color::from_rgb(80, 80, 80)),
                _ => (' ', Color::default()),
            };
            let style = Style::default().with_fg(fg);
            grid.set(p, Cell::default().with_char(ch).with_style(style));
        }
        // Draw player
        let ps = Style::default().with_fg(Color::from_rgb(0, 128, 255));
        grid.set(self.player_pos, Cell::default().with_char('@').with_style(ps));
    }
}

// Pathfinding
struct MyPather { map: ... }
impl Pather for MyPather {
    fn neighbors(&self, p: Point) -> Vec<Point> {
        Neighbors::cardinal(p, |q| self.map.passable(q))
    }
}
impl WeightedPather for MyPather {
    fn cost(&self, _from: Point, _to: Point) -> i32 { 1 }
}
impl AstarPather for MyPather {
    fn estimation(&self, from: Point, to: Point) -> i32 {
        manhattan(from, to)
    }
}
let path = pr.astar_path(&pather, monster_pos, player_pos);

// FOV
struct MyLighter { terrain: ... }
impl Lighter for MyLighter {
    fn cost(&self, _from: Point, to: Point) -> i32 {
        match self.terrain.at(to).unwrap_or(WALL) {
            WALL | RUBBLE => -1,  // blocks
            FOLIAGE => 3,         // partial block
            _ => 1,               // normal
        }
    }
    fn light(&self, _from: Point, _to: Point) -> i32 { 1 }
}
let visible = fov.ssc_vision_map(&lighter, player_pos, false);

// Custom message for animation ticks
#[derive(Debug, Clone, Copy)]
struct AnimTick(usize);
let effect = Effect::Cmd(Box::new(move || {
    std::thread::sleep(Duration::from_millis(40));
    Some(Msg::custom(AnimTick(frame_idx)))
}));
// In update:
if let Some(tick) = msg.downcast_ref::<AnimTick>() { ... }
```

---

## Things That Don't Port 1:1

1. **Go interfaces → Rust enums**: Entity roles, Actions, Items all use `interface{}` in Go. Use enums with variant data in Rust.
2. **Go slices → Rust Vec**: Go's `[]*Entity` becomes `Vec<Entity>` or `Vec<Option<Entity>>` for sparse ID-indexed access.
3. **Mutable model in draw**: Go's `Draw()` returns a `Grid`; Rust's `draw(&self, grid)` is immutable. Store computed draw state in the model during `update()`.
4. **Go's `any` type assertions → Rust `match` on enums**: Pattern match on entity roles instead of type assertions.
5. **Global vars → struct fields**: Go shamogu uses several global vars (ColorMode, CustomKeys, etc.). Move these into the model struct.
6. **Go error handling → Rust Result**: Propagate with `?` operator.
7. **rand API**: Go's `rand.IntN(n)` → Rust's `rng.random_range(0..n)` (rand 0.9 crate).

---

## Mandatory Pre-Commit Checklist

See `AGENTS.md`. Always:
1. `cargo fmt --all`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`
4. Update `TODO.md`
5. `git add <files>` (never `git add -A`)
6. `git commit` with descriptive message
7. `git push`

---

## Success Criteria

Minimum viable port (Phase 1-3):
- [ ] Player can move through a generated cave
- [ ] FOV with lit/dark/seen states
- [ ] Monsters chase player using A*
- [ ] Bump-to-attack combat with HP
- [ ] Message log
- [ ] Status bar
- [ ] Game over on death
- [ ] Runs in terminal via `cargo run --bin shamogu`

Full port (Phase 1-6):
- [ ] All the above plus items, abilities, status effects
- [ ] Menu system, help pager
- [ ] Animations
- [ ] Save/load
- [ ] Multiple dungeon levels
