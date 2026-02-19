# Shamogu Source Analysis — Comprehensive Feature Extraction

## 1. game.go — Game State & Core Loop

### Constants
- `Version = "v1.4.1"`
- `MapWidth = 80`, `MapHeight = 21`
- `InventorySize = NSpirits + NComestibles`
- `FirstMapID = InventorySize` (first non-inventory entity ID)
- `PlayerID = FirstMapID`
- `MaxFOVRange = 8` (from fov.go, used here)

### Types/Structs
- **`ID`** (`int32`) — entity identifier
- **`Game`** — core game state struct containing:
  - `Entities []*Entity` — all entities indexed by ID
  - `Map *Map` — current level map
  - `PR`, `PRnoise` — two PathRange objects (one for general pathfinding, one for noise to avoid conflicts)
  - `Dir` — last bump direction
  - `Prev` — previous player position
  - `Turn` — current turn number
  - `CorruptionTurn` — next corruption event turn (for ModCorruptedDungeon)
  - `Logs *Logs` — game log
  - `Mods []bool` — active game mods
  - `ProcInfo *ProcInfo` — procedural generation info
  - `Stats *Stats` — game statistics
  - `Version` — game version string
  - `Wizard` — wizard mode
  - `instant` — whether last action was instant-effect (no turn cost)
  - `snack` — eating a snack flag
  - `win` — game won flag
  - `rand` — RNG
  - `md *model` — UI model reference
- **`NoiseType`** (`int`) — enum for non-movement noise sources

### NoiseType Enum (16 types)
| Constant | Noise Radius | Message |
|---|---|---|
| `NoiseBark` | MaxFOVRange | "WOOF!" |
| `NoiseCombat` | 4 | "Smash!" |
| `NoiseChomp` | 4 | "Chomp!" |
| `NoiseCackle` | MaxFOVRange | "KO-KO-KO!" |
| `NoiseDig` | MaxFOVRange | "CRACK!" |
| `NoiseExplosion` | MaxFOVRange+4 | "POP-BOOM!" |
| `NoiseEarthMenhir` | 2*MaxFOVRange | "RING-RING!" |
| `NoiseFakePortal` | MaxFOVRange+4 | "THRUM!" |
| `NoiseLightning` | MaxFOVRange | "PANG!" |
| `NoiseMusic` | 2*MaxFOVRange | "♫ larilon, larila ♫ ♪" |
| `NoiseRoar` | MaxFOVRange | "ROAR!" |
| `NoiseStomp` | MaxFOVRange | "STOMP!" |
| `NoiseTrumpet` | MaxFOVRange | "TARARA!" |
| `NoiseWind` | MaxFOVRange | "WHIZ!" |
| `NoiseHeavySteps` | 6 | (silent log) |
| `NoiseGrowl` | 6 | "Growl!" |

### Key Methods & Mechanics

#### Initialization
- `Init(spe *Entity)` — initializes game state, entities, map, pathfinding, CorruptionTurn = 50+rand(250)
- `InitLevel()` — generates new map level (increment level, generate map, reset knowledge, update FOV)

#### Turn Processing — `EndTurn()`
1. Clears noise map
2. Checks for early return (instant actions, TimeStop status)
3. Collects all actor IDs, shuffles monster order randomly
4. Processes each actor's turn; if player dies mid-turn, remaining monsters skip
5. Updates FOV when player position changes or on player's own turn
6. Updates clouds after all actors act
7. Final FOV update, noise computation, turn count handling

#### Turn Count & Dungeon Core — `HandleTurnCount()`
- At **turn 950**: warning message ("feel a presence searching for intruders")
- At **turn 1000**: "Found by the dungeon core!" — spawns **2 Blazing Golems**
- If `ModCorruptedDungeon` enabled: triggers corruption events at `CorruptionTurn`

#### Corruption Events — `handleCorruptionEvent()`
- Next event scheduled at: current turn + 25 + rand(300)
- 10% chance (case 0): Foliage→Floor, Rubble→Foliage, Wall→TranslucentWall (rare), Floor gets normal cloud
- 90% chance (default): Foliage→Floor, Rubble→Floor, Wall↔TranslucentWall, Floor→Rubble/Foliage
- Logs a random interjection: "Uh.", "Um.", "Oh.", "Eh."

#### Noise System — `ComputeNoise()`
- Uses BFS from player position with max distance = MaxFOVRange + 5
- Monster hearing modifiers:
  - **GoodHearing** trait: reduces distance by MaxFOVRange/4
  - **BadHearing** trait: can't hear wing flaps or creeping > 2 distance
  - **MonsSilent**: unhearable without GoodHearing; with GoodHearing adds MaxFOVRange/2
  - **MonsHeavyFootsteps**: reduces distance by 3
  - **MonsLightFootsteps/MonsCreep**: increases distance by MaxFOVRange/4
- `StatusClarity` overrides noise (provides superior senses)
- Noise log messages differ by monster type (footsteps, flapping, creep, air movement)

#### Noise Production — `MakeNoise()`
- BFS from noise source with max distance = 1.5× noise intensity
- Player hears if within range (boosted by GoodHearing)
- Monsters update their target if they hear the noise

#### Sensing
- `SmellFood()` — GoodSmell trait: sense comestibles within 2×MaxFOVRange manhattan distance
- `SenseItems(ty)` — Gawalt trait: sense menhirs within 2×MaxFOVRange

#### Mods
- `Mod(m Mod) bool` — checks if a mod is enabled
- `HasMod(mods []bool, m Mod) bool` — static check

#### Time Stop
- `endTurnEarly()` — if `StatusTimeStop`, only player acts (monsters frozen)

---

## 2. map.go — Map Structure & Terrain

### Terrain Types (rl.Cell enum, 7 values)
| Constant | Value | Passable | Blocks Vision | Description |
|---|---|---|---|---|
| `Wall` | 0 | No | Yes | Obstructing pile of rocks |
| `Floor` | 1 | Yes | No | Passable plain ground |
| `Foliage` | 2 | Yes | Partially (fuzzy) | Dense foliage, difficult to see through, **flammable** |
| `Rubble` | 3 | Yes | Yes | Broken rocks, passable but blocks vision |
| `TranslucentWall` | 4 | No | No | Obstructing translucent rocks, **contains poison gas** |
| `UnknownPassable` | 5 | — | — | Known to be passable (knowledge layer) |
| `Unknown` | 6 | — | — | Unknown terrain (knowledge layer) |

### Terrain Runes
| Terrain | Rune |
|---|---|
| Wall | `#` (or space if surrounded by walls) |
| Floor | `.` |
| Foliage | `"` |
| Rubble | `^` |
| TranslucentWall | `◊` |
| UnknownPassable | `♫` |
| Unknown wall (adjacent to known) | `¤` |

### Map Struct
- `Terrain rl.Grid` — actual terrain
- `KnownTerrain rl.Grid` — player's knowledge of terrain
- `FOV *rl.FOV` — player's field of view
- `FOVPts []gruid.Point` — points in SSC field of view
- `Clouds *CloudGrid` — cloud map
- `Level int` — current dungeon level (1-9)
- `Noise map[gruid.Point]NoiseType` — noise sources this turn
- `Waypoints []gruid.Point` — vault patrol waypoints
- `BoolCache CacheGrid[bool]` — boolean cache for algorithms
- `ActorCache CacheGrid[ID]` — actor position cache
- `RuneCache CacheGrid[ID]` — runic trap position cache
- `Orb gruid.Point` — Orb of Corruption position
- `Portal gruid.Point` — true portal position
- `Totem gruid.Point` — totem position

### CacheGrid[T] — Generic Grid
- `At(p)`, `AtU(p)` — read (with/without bounds check)
- `Set(p, v)`, `SetU(p, v)` — write (with/without bounds check)
- `New()` — allocate or clear

### Key Methods
- `Passable(p)` — checks if terrain is passable (not Wall, not TranslucentWall)
- `PassableWithoutTraps(p)` — passable AND no runic trap
- `Connected(p)` — has at least one adjacent passable tile
- `AdjacentNonPassableCount(p)` — count of non-passable neighbors (0-4)
- `NoWallAt(p)` — not a regular Wall
- `Burnable(p)` — is Foliage

---

## 3. mapgen.go — Map Generation

### MapLayout Enum (6 layouts)
| Constant | Description |
|---|---|
| `CellularAutomataCave` | Cellular automata cave |
| `RandomWalkCave` | Random walk cave |
| `RandomWalkTreeCave` | Random walk tree cave (grows from center) |
| `MixedAutomataWalkCave` | Left/right half: automata + walk |
| `MixedAutomataWalkTreeCave` | Left/right half: automata + tree walk |
| `MixedWalkCaveWalkTreeCave` | Left/right half: walk + tree walk |

- `NLayouts = 3` (base layout types, mixed ones combine them)

### mapTheme Enum (5 themes)
| Constant | Effect |
|---|---|
| `ThemeNone` | No special theme |
| `ThemeBerserk` | Affects item/monster generation |
| `ThemeFire` | Fire-themed items/monsters, more foliage |
| `ThemeLignification` | Lignification-themed |
| `ThemeWarp` | Warp-themed, more translucent walls |

### MapGen Struct
- `terrain rl.Grid`
- `theme mapTheme`
- `vaults []*vault`
- `tunnel CacheGrid[bool]` — cells in tunnels
- `xtunnel []gruid.Point` — extra tunnel points
- `vault CacheGrid[bool]` — cells in vaults
- `itemPlace CacheGrid[bool]` — item/static placement cells
- `PR *paths.PathRange`
- `rand *rand.Rand`

### Placement Enum
| Constant | Description |
|---|---|
| `PlacementRandom` | Random position |
| `PlacementCenter` | Near map center |
| `PlacementEdge` | Near left or right edge |

### Generation Pipeline — `GenerateMap(ml)`
1. Fill map with walls
2. Choose theme (if themed level)
3. Generate base terrain using layout algorithm
   - For mixed layouts: split map into left/right halves, apply different algorithms
4. Generate foliage overlay (cellular automata on floor tiles)
5. Place vaults:
   - 1 big vault at center or edge
   - 1 small vault at the opposite (edge or center)
   - 1 big vault randomly
   - 4-5 small vaults randomly
6. Connect all vaults with tunnels
7. Compute waypoints
8. Apply earthquake (extra rubble) if applicable
9. Apply corrupted terrain if ModCorruptedDungeon
10. Remove unreachable terrain (flood fill from waypoint, replace unreachable with walls)
11. Validate: must have > 1000 passable tiles, otherwise regenerate
12. Generate entities
13. Reset clouds

### Cave Generation Algorithms

#### Cellular Automata (`genCellularAutomataCaveMap`)
- Initial wall density: 42%, 45%, or 48% (random)
- Two-phase rules:
  - Phase 1: cutoff1=5, cutoff2=2, 4 reps
  - Phase 2: cutoff1=5, cutoff2=25, 3 reps

#### Random Walk (`genCaveMap`)
- Target size: height × (37-42 adjusted by grid width)
- 7-9 walks (adjusted by grid width)
- Favors horizontal movement (4/6 chance horizontal, 2/6 vertical)

#### Tree Walk (`genTreeCaveMap`)
- Starts from center (±2,±1 randomized), clears 3×3 area
- Digs blocks from random wall cells until connected to existing floor
- Target: height × (30-34 adjusted) cells

### Foliage Generation (`genFoliage`)
- Separate cellular automata layer
- Initial density: 53-55%
- Overlaid on floor cells only

### Vault Placement
- Vaults can be rotated (90°/270°) and reflected
- Rotation probability adjusted by aspect ratio
- Must not overlap other vaults (1-cell buffer)
- Up to 500 attempts per vault, 10 placement attempts per template

### Tunnel System
- Vaults sorted by distance to center
- Each vault connected to nearest already-connected vault
- 3-5 extra tunnels added between near vaults
- Tunnel fill: 1/8 chance rubble, 1/8 chance foliage, 6/8 floor
- Uses A* pathfinding for tunnel routing

### Earthquake (`earthquake`)
- 1/6 chance to convert each wall to rubble

### Corrupted Terrain (`genCorruptedTerrain`) — extensive system
Applied when `ModCorruptedDungeon` is active. Multiple random effects:
1. **Foliage flip** (1/6 chance): swap floor↔foliage in vaults or outside vaults
2. **Sparse terrain** (1/6 chance): add random foliage/rubble/floor or mix outside vaults
3. **Wall destruction** (1/6 chance): replace non-vault walls with floor/foliage/rubble mix
4. **Corruption zone** (1/4 chance, higher for themed): BFS from random point, corrupt terrain in radius up to 2-3× MaxFOVRange
   - Uniform corruption (single terrain type)
   - Sparse corruption (swap wall/translucent wall)
   - Sparse wall destruction
5. **Translucent walls** (1/MapLevels chance): convert many walls to translucent

---

## 4. vaults.go — Vault System

### Data
- Vault templates loaded from embedded files: `data/small-vaults.txt`, `data/big-vaults.txt`
- Parsed at init into `SmallVaultTemplates` and `BigVaultTemplates`
- Every vault must contain: `W` (waypoint), `>` (static place), `!` (item place), `+` or `-` (entry)

### vault Struct
- `p gruid.Point` — position
- `w, h int` — dimensions
- `entries []ventry` — tunnel entry points
- `places []place` — special places
- `vault *rl.Vault` — parsed template
- `tunnels int` — number of connected tunnels

### ventry Struct
- `p gruid.Point` — position
- `used bool` — whether used by a tunnel

### place Struct
- `p gruid.Point` — position
- `kind placeKind` — type of place
- `used bool` — whether already used

### placeKind Enum (4 types)
| Constant | Description |
|---|---|
| `PlaceEntry` | Entry point |
| `PlaceItem` | Item placement (`!`) |
| `PlaceStatic` | Static object placement (`>`) |
| `PlaceWaypoint` | Patrol waypoint (`W`) |

### Vault Template Rune Mappings
| Rune | Terrain | Notes |
|---|---|---|
| `.` | Floor | |
| `!` | Floor | + registers PlaceItem |
| `-` | Floor | + registers entry point |
| `>` | Floor | + registers PlaceStatic |
| `W` | Floor | + registers PlaceWaypoint |
| `#` | Wall | |
| `+` | Wall | + registers entry point |
| `$` | TranslucentWall | |
| `%` | 50/50 Wall or TranslucentWall | |
| `&` | Random: 1/5 each of Wall/TranslucentWall/Foliage/Rubble/Floor | |
| `"` | Foliage | |
| `^` | Rubble | |
| `:` | Random: 1/3 each of Floor/Foliage/Rubble | |
| `?` | No change (transparent/passthrough) | |

### Key Methods
- `DigVault(v)` — draws vault onto map, records places and entries
- `UnusedEntry(v)` — returns unused entry or random fallback
- `RandomPlace(mg, kind)` — random free place of given kind across all vaults
- `RandomVaultPlace(v, kind)` — random free place in specific vault

---

## 5. clouds.go — Cloud/Gas System

### CloudKind Enum (4 types)
| Constant | Color | Effect |
|---|---|---|
| `NoCloud` | — | No cloud |
| `CloudNormal` | Foreground (white) | Obstructs vision (steam/dust/smoke) |
| `CloudFire` | Red | Burns creatures + obstructs vision + spreads to foliage |
| `CloudPoison` | Green | Poisons creatures + obstructs vision |

### Constants
- `FireDamage = 1`
- `PoisonDamage = 1`

### Cloud Struct
- `Kind CloudKind`
- `P gruid.Point` — position
- `Duration int` — turns remaining

### CloudGrid Struct
- `Clouds []Cloud` — all active clouds
- `Grid []int` — map position → index in Clouds (-1 = no cloud)
- Supports O(1) position lookup and linear iteration

### Cloud Mechanics

#### Cloud Duration
- Fire clouds: 6 + rand(7) = 6-12 turns
- Poison clouds: 7 + rand(5) = 7-11 turns
- Normal clouds: variable

#### Cloud Priority
- Same-type clouds: use maximum duration
- Fire clouds take priority over all others (heat dissipates other cloud types)
- This allows protective usage of "foggy-skin onion" in foliage without removing fires

#### Cloud Update (`UpdateClouds`) — per turn
1. Decrement all cloud durations by 1
2. Remove expired clouds
3. For expired fire clouds on foliage: convert foliage→floor, replace with smoke cloud (4-8 turns)
4. Active fire clouds: inflict `StatusFire` on actors, spread to adjacent flammable tiles (1/3 chance per neighbor)
5. Active poison clouds: inflict `StatusPoison` on actors

#### Cloud Operations
- `AddCloud(cl)` — add cloud, respecting priority rules
- `SwapClouds(p, q)` — swap clouds at two positions
- `RemoveCloudAt(p)` — remove cloud
- `NormalCloudAt(p, d)` — add steam/dust/smoke
- `FireCloudAt(p)` — add fire
- `PoisonCloudAt(p)` — add poison
- `FireAt(p)` — check for fire
- `CloudAt(p)` — check for any cloud

#### Statistics tracked
- `Stats.FireClouds` — total fire clouds created
- `Stats.PoisonClouds` — total poison clouds created

---

## 6. runictraps.go — Runic Trap System

### MagicRune Enum (5 rune types)
| Constant | Color | Effect When Triggered |
|---|---|---|
| `RuneBerserk` | Magenta | Applies `StatusBerserk` for `DurationBerserkTrap` turns (skipped if already berserk) |
| `RuneFire` | Red | Inflicts `StatusConfusion` for `DurationConfusionFireTrap` turns + spawns fire cloud |
| `RuneLignification` | Orange | Applies `StatusLignification` for `DurationLignificationTrap` turns (skipped if already lignified) |
| `RunePoison` | Green | Spawns poison cloud (7-11 turns) |
| `RuneWarp` | Violet | Teleports actor away |

- `NRunes = 5`

### RunicTrap Struct
- `Used bool` — already triggered
- `KnownUsed bool` — player knows it's triggered
- `Rune MagicRune` — which rune type

### Trigger Mechanics — `TriggerTrap(i, ai)`
- **Immunity**: actors with `RunicChicken` trait never trigger traps
- **Conditional skips**:
  - Lignification trap skipped if already lignified or has `MonsLignify`
  - Berserk trap skipped if already berserk
- Logs differently for player vs monsters
- Tracks statistics: `PlayerTrapTriggers`, `MonsterTrapTriggers`, `MapTriggeredTraps`
- After triggering: `Used = true`, cache cleared, `KnownUsed` set if in FOV

---

## 7. fov.go — Field of View System

### Constants
- `MaxFOVRange = 8`

### FOV Computation — `UpdateFOV()`
- Uses `rl.FOV.VisionMap` with custom lighter
- Also uses `rl.FOV.SSCVisionMap` for symmetric shadowcasting
- Range reduced to `MaxFOVRange - 3 = 5` for `NocturnalFlying` (unless lignified or gardener)

### Lighter (implements rl.Lighter)
- `MaxCost`: 9 normally, 6 for flying
- Vision-blocking terrain costs:
  - **Wall, Rubble**: max cost (fully blocks)
  - **Foliage**: blocks unless flying (then clear); contributes fuzzy visibility diagonally
  - **CloudNormal**: max cost (fully blocks)
  - **Other clouds** (fire, poison): high cost but reduced by 3 (partially see-through)
- **Diagonal visibility system** (3 states):
  - `Opaque (0b00)` — both diagonal neighbors block
  - `Fuzzy (0b01)` — one diagonal neighbor partially blocks
  - `Clear (0b11)` — at least one diagonal neighbor is transparent
  - Two adjacent walls/rubble in cardinal directions block diagonal line of sight

### Knowledge System — `UpdateKnowledge()`
- Iterates FOV points within MaxFOVRange
- Updates `KnownTerrain` for visible tiles
- Calls `SeeEntities()` to update entity knowledge

### Entity Sensing — `SenseEntity(i, verb)`
On first sight of an entity, different behavior by type:
- **Actor**: log + story log for notable monsters; triggers `NoiseTrumpet` (Elephanty) or `NoiseRoar` + `StatusFear` (ScaryRoar)
- **Comestible**: notable log + story log
- **Spirit**: notable log + story log + cackle
- **EmptyTotem**: notable log + story log
- **CorruptionOrb**: notable log + story log + cackle
- **RunicTrap**: notable log + story log
- **Portal**: notable log + story log + cackle

### Entity Types Referenced (from SenseEntity)
- `*Actor` — monsters/player
- `*Comestible` — food items
- `*Spirit` — totemic spirits
- `*EmptyTotem` — empty totem
- `*CorruptionOrb` — the Orb of Corruption
- `*RunicTrap` — runic traps
- `*Portal` — portals

### Clarity Status
- `StatusClarity`: senses all monsters within 2×MaxFOVRange manhattan distance
- Overrides noise system (strictly better range)

### Danger Detection
- `DangerInFOV()` — any alive monster or dangerous cloud (fire/poison) in FOV
- `DangerInProximity()` — any alive monster in FOV or heard via noise, or dangerous cloud

---

## 8. procinfo.go — Procedural Generation Info

### Constants
- `MapLevels = 9` — total dungeon levels

### ProcInfo Struct — Full procedural state
| Field | Type | Purpose |
|---|---|---|
| `Layouts` | `[]MapLayout` | Map layout per level (shuffled) |
| `Earthquake` | `int` | Level with extra rubble (2-9, can be 0) |
| `FakePortal` | `[]bool` | Levels with fake/malfunctioning portal |
| `GuardianTotem1` | `int` | Level with totem guardian 1 (level 3-6) |
| `GuardianTotem2` | `int` | Level with totem guardian 2 (wasps, level 3-6) |
| `GuardianPortal1` | `int` | Level with portal guardian 1 (level 4-7) |
| `GuardianPortal2` | `int` | Level with portal guardian 2 (level 6-8) |
| `WanderingUnique1` | `int` | Level with Walking Mushroom (level 4-9) |
| `WanderingUnique2` | `int` | Level with Noisy Imp (level 4-9) |
| `MonsEarly` | `int` | Special early level (0-3) |
| `MonsMid` | `int` | Special mid level (4-6) |
| `MonsMidLate` | `int` | Special mid-late level (6-9) |
| `MonsLate` | `int` | Special late level (7-9) |
| `MonsLateSwarm` | `int` | Swarm late level (7-9, can be 0) |
| `ThemedLevel` | `int` | Themed level (4-9, only with ModCorruptedDungeon, 1/3 chance) |
| `TrapLevel` | `int` | Level with lots of traps (5-9, can be 0) |
| `Spirits` | `[]spiritProcInfo` | Spirit/totem per level |
| `Menhirs` | `[]int` | Menhir generation sequence |
| `MenhirIdx` | `int` | Current menhir index |
| `Comestibles` | `[]int` | Comestible generation sequence |
| `ComestibleIdx` | `int` | Current comestible index |
| `NComestibles` | `[]int` | Comestible count per level |
| `NMenhirs` | `[]int` | Menhir count per level |
| `Runes` | `[]int` | Rune generation sequence |
| `RuneIdx` | `int` | Current rune index |

### spiritProcInfo
- `Idx int` — index into spirit data (-1 = empty totem)
- `Advanced bool` — whether it's an advanced/challenge spirit

### Layout Generation (`layoutsProcGen`)
- 9 levels, initial pattern: [Automata, TreeWalk, Walk, Automata, TreeWalk, Walk, Mixed1, Mixed2, Mixed3]
- Levels 4-6 have 50% chance of becoming mixed layouts
- Entire array is shuffled

### Spirit/Totem Generation (`spiritProcGen`)
- 6 non-empty totems generated per game
- With `ModAdvancedSpirits`: 3 challenge + 3 regular spirits, with positioning bias
- Without mod: 6 regular secondary spirits
- 2 empty totems inserted randomly in levels 2-8
- Level 1 always has a totem, level 9 never does
- Consecutive empty totems made less likely

### Flavour/Event Generation (`flavoursProcGen`)
- **Fake Portals**: 1-3 per game, in levels 2-8
- **ModCorruptedDungeon** can lower minimum levels for special monsters
- **Swarm level**: 1/3 chance of being disabled
- **Random event disabling** (1/10 chance each): earthquake, unique1, unique2, trap level

### Item Count Per Level
- **Comestibles**: [2,2,3,3,4, 4,5,5,6] base + random adjustments
- **Menhirs**: [1,1,1,1,2, 2,2,2,2] base + extras, capped at 3 per level

### Sequence Generation (`genRandomIndices`)
- Creates semi-random sequences that ensure all types appear regularly
- Used for menhirs, comestibles, and runes
- `NextMenhirKind()`, `NextComestibleKind()`, `NextRune()` — draw from sequences, regenerate when exhausted
- `ModHealingCombat` excludes `AmbrosiaBerries` from comestible generation

---

## Cross-File Feature Summary

### Core Game Systems
1. **Turn-based roguelike** with 80×21 maps across 9 dungeon levels
2. **Entity-component system** with ID-based entity management
3. **Mod system** (ModCorruptedDungeon, ModAdvancedSpirits, ModHealingCombat, ModGluttonyRework)
4. **Wizard mode** for debugging

### Map & Terrain
5. **5 terrain types**: Wall, Floor, Foliage (flammable, semi-transparent), Rubble (passable, opaque), TranslucentWall (impassable, transparent, poison)
6. **3 base generation algorithms** combinable into 6 layout types
7. **Vault system** with template-based rooms, entries, waypoints
8. **Tunnel system** connecting vaults with A* pathfinding
9. **Earthquake events** converting walls to rubble
10. **Corrupted terrain** mutations (foliage flip, wall destruction, corruption zones)
11. **5 themed level types** affecting generation

### Vision & Knowledge
12. **Field of view** with symmetric shadowcasting, range 8 (5 for flying)
13. **Fog of war** with persistent terrain knowledge
14. **Diagonal visibility** system (opaque/fuzzy/clear)
15. **Foliage/cloud obstruction** with partial visibility
16. **Entity knowledge tracking** (last known position)

### Sound & Noise
17. **16 noise types** with varying radii (2-16 tiles)
18. **Footstep noise** hearing system with BFS distance
19. **5 monster noise categories** (silent, heavy, light, creep, wing flap)
20. **Good/Bad hearing** traits affecting detection

### Cloud/Gas System
21. **3 cloud types**: normal (vision block), fire (damage + spread), poison (damage)
22. **Fire spreading** to adjacent foliage (1/3 chance per turn)
23. **Fire→smoke transition** when foliage burns out
24. **Cloud priority** system (fire > others)

### Trap System
25. **5 runic trap types**: Berserk, Fire, Lignification, Poison, Warp
26. **Single-use traps** that affect both player and monsters
27. **Conditional trigger immunity** (can't double-apply certain effects)
28. **RunicChicken trait** = complete trap immunity

### Status Effects Referenced
- StatusTimeStop, StatusClarity, StatusFire, StatusPoison
- StatusBerserk, StatusConfusion, StatusLignification, StatusFear
- StatusGardener

### Monster Traits Referenced
- GoodHearing, BadHearing, MonsSilent, MonsHeavyFootsteps
- MonsLightFootsteps, MonsCreep, MonsWingFlap, MonsNotable
- NocturnalFlying, Elephanty, ScaryRoar, RunicChicken, MonsLignify

### Special Entities
- **Blazing Golem** — spawned by dungeon core at turn 1000
- **Walking Mushroom** — wandering unique
- **Noisy Imp** — wandering unique (makes music noise)
- **Hungry Rat** — triggers elephant trumpet
- **Orb of Corruption** — key item
- **Portals** (true and fake)
- **Totems** (with spirits or empty)
- **Menhirs** — various types
- **Comestibles** — food items including Ambrosia Berries

### Procedural Guarantees
- All item/rune types appear regularly (semi-random sequences)
- Level 1 always has a totem
- Level 9 never has a totem
- At least 1 fake portal per game
- Special monster levels distributed across early/mid/late game
- Minimum 1000 passable tiles per map
