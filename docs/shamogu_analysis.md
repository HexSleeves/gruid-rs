# Shamogu Game Analysis — Comprehensive Feature Extraction

## 1. actions.go — Player Actions (1880 lines)

### Action Types (Structs)

| Action Struct | Purpose |
|---|---|
| `ActionNone` | No-op action |
| `ActionWait` | Wait a turn (rest in place) |
| `ActionBump{Delta}` | Move/attack in a cardinal direction |
| `ActionRun{Delta}` | Auto-run in a cardinal direction |
| `ActionTravel` | Auto-travel to cursor target |
| `ActionAutoExplore` | Auto-explore unexplored map |
| `ActionCursorBump{Delta}` | Move examine cursor one step |
| `ActionCursorRun{Delta}` | Move examine cursor multiple steps |
| `ActionNextMonster` | Cycle examine cursor to next monster |
| `ActionPreviousMonster` | Cycle examine cursor to previous monster |
| `ActionNextItem{Type}` | Cycle examine cursor to next item of type |
| `ActionExamine{Target}` | Examine a specific screen position |
| `ActionExamineModeToggle` | Toggle keyboard examine mode |
| `ActionScroll{Delta}` | Scroll description boxes up/down |
| `ActionInteract` | Interact with item on current cell (equip/activate) |
| `ActionInventory` | Open inventory menu |
| `ActionUseItem{ID}` | Use a specific inventory item |
| `ActionBindItem{ID}` | Choose first comestible (ModGluttonyRework) |
| `ActionUseTwoItems{ID0,ID1}` | Use two comestibles at once (ModGluttonyRework) |
| `ActionEquipItemAt{ID}` | Equip item into a specific slot |
| `ActionMenu` | Open main game menu |
| `ActionConfig` | Open settings menu |
| `ActionToggleDarkLight` | Toggle dark/light mode |
| `ActionToggleExtraWarnings` | Toggle fire/poison/expiry warnings |
| `ActionToggleAdvancedNewGame` | Toggle advanced new game as default |
| `ActionSetKeys` | View/customize keybindings |
| `ActionViewMessages` | Open message log pager |
| `ActionDump` | Dump game statistics to file |
| `ActionSaveQuit` | Save and quit |
| `ActionQuit` | Quit without saving (asks confirmation) |
| `ActionQuitConfirm{State}` | Confirm quit |
| `ActionWizard` | Enter/cycle wizard mode |
| `ActionWizardConfirm{State}` | Confirm wizard mode |
| `ActionWizardNextLevel` | Cheat: skip to next level |
| `ActionHelp` | Open help menu |
| `ActionHelpDefaultKeys` | Show default keybindings |
| `ActionHelpCombat` | Show combat help |
| `ActionHelpItems` | Show items help |
| `ActionHelpMods` | Show mods help |
| `ActionHelpStatuses` | Show statuses help |
| `ActionHelpTips` | Show gameplay tips |

### Item Types (Targetable)

```
itemComestible  — food items
itemTotem       — spirit totems
itemMenhir      — menhir stones
itemPortal      — portals (includes CorruptionOrb)
itemRune        — runic traps
```

### Wizard Mode

```
WizardNone           — normal permadeath
WizardNormal         — non-permadeath (resurrection on death)
WizardReveal         — reveal map and monsters
WizardRevealTerrain  — reveal map only (no monsters)
```

### Menu System
- Main menu: Interact, Inventory, View Messages, Help, Dump, Config, Save/Quit, Quit, (Wizard: Next Level)
- Config menu: Set Keys, Toggle Dark/Light, Toggle Extra Warnings, Toggle Advanced New Game
- Help menu: Keybindings, Combat, Items, Mods, Statuses, Tips
- Inventory: Equip mode (spirits/comestibles) and Use mode
- Keyboard-rebindable actions (35 configurable actions including movement, cursor, targeting, menus)

### Interaction System
- Items on the ground: Comestibles can be picked up, Spirits can be equipped/upgrade, non-equippables get used
- Inventory slots: Primary Spirit (1), Secondary Spirits, Comestibles section
- Spirit upgrading: spirits can be upgraded once (Level 0→1)
- Comestible management: full inventory forces replacement

---

## 2. player.go — Player-Specific Logic (291 lines)

### Key Methods

| Method | Mechanic |
|---|---|
| `PlayerBump(dir)` | Core player movement/attack dispatcher |
| `PlayerFears(j, aj)` | Check if player is afraid (StatusFear or Elephant+Rat without Berserk) |
| `Sprint(dir)` | Sprinting movement (2-3 tiles same dir, 1 tile backward) |
| `WallThrough(dir, at)` | Move through translucent walls (Shadow status) |
| `WallJump(dir)` | Jump off walls (Gawalt trait) |
| `ComputePlayerStats()` | Recompute stats from equipped spirits |

### Player Base Stats
- **Attack**: 2 (base)
- **Defense**: 1 (base)
- **MaxHP**: 9 (base)
- **Traits**: `Player` (base)
- Spirits add bonuses to Attack, Defense, MaxHP, and Traits

### Movement Mechanics

1. **Daze check**: Dazed player cannot act (must wait or eat)
2. **Shadow + Translucent Wall**: Instant pass-through
3. **Gawalt + Wall**: Wall-jump (propel off wall, hit monster on the way)
4. **Sprint**: Move 2-3 tiles in same direction, 1 tile backward, 2 tiles lateral; jump over monsters (unbalancing them); may fall if imbalanced
5. **Crocodile**: Cannot turn and attack in same turn; must spend turn turning backward
6. **Elephant**: Must spend turn turning when facing a wall
7. **Fear**: Cannot move toward feared monsters
8. **Lignification**: Cannot move at all
9. **Poison at 1 HP**: Cannot move
10. **Dig status**: Can walk into walls (destroying them)

### Attack Resolution (from bump)

1. **Adjacent melee**:
   - Crocodile: direction change = turn only; else AttackDrag
   - FourDirs (without Focus): attacks all 4 adjacent foes
   - Default: plain bump attack
2. **Ranged** (target in line of sight beyond adjacent):
   - PatternRanged / PatternRangedRecoil: ranged attack
   - PatternCatch (if target doesn't resist move): catch attack
   - PatternSwap / PatternSwapDaze: swap-attack
   - PatternRampage / PatternBat (if HP > 1 or no poison): charge to target
3. **Move + Charge**: After moving, check tile ahead for charge target
   - FourDirs (without Focus): four-directional charge attack
   - Normal: charge attack on actor in next tile

---

## 3. monsters.go — Monster AI (433 lines)

### AI States (Mindstate)

```
Wandering  — default patrol state
Hunting    — actively pursuing player
```

### Behavior Struct

```go
type Behavior struct {
    SkipTurn bool          // skip next turn (from position swapping)
    State    Mindstate     // Wandering or Hunting
    Target   gruid.Point   // current movement target
    Guard    gruid.Point   // guard position (stays nearby)
    Path     []gruid.Point // precomputed path
}
```

### Monster AI Flow (`HandleMonsterTurn`)

1. **Skip turn check**: If flagged, skip
2. **Daze check**: Dazed monsters do nothing
3. **Confusion + Discord**: Confused monsters may attack random adjacent monster
4. **Target update**: If player is in FOV, update target
   - Wandering monsters spend 1 turn "noticing" player (unless Dazzling Zebra)
   - Dazzling players are noticed at end of monster turn too
5. **Target reached / invalid**: Choose new target
   - Hungry monsters hunt player by smell
   - Marked player draws guards
   - Guard monsters stay within MaxFOVRange-1 of guard point
   - Random wandering target (biased toward vault waypoints)
   - `CallToCommonTarget`: nearby monsters share target (herd behavior, 1-2 recruits)
6. **Disorientation**: If player has StatusDisorient, monster moves in wrong direction
7. **Flee**: Afraid or Musical monsters flee (move away from player)
8. **Bump next**: Move toward target via path

### Monster Special Behaviors

| Behavior | Description |
|---|---|
| `monsterDiscord` | Confused monsters attack random adjacent monster |
| `monsterBumpDisoriented` | Disoriented monsters move in player's facing direction reversed |
| `monsterFlee` | Afraid/Musical monsters flee; Musical monsters also make noise |
| `MonsterUpdateTarget` | Awareness system — wandering→hunting transition |
| `PlayerIsHidden` | Gawalt on menhir/in shadow OR Elephant in dead-end (≥3 adjacent walls) |
| `releaseSpores` | Wandering Mushroom: lignify all actors in BFS range |
| `HungryHunt` | Hungry monsters track player by smell within 1.5× FOV range |
| `CallToCommonTarget` | Nearby wandering monsters adopt same target (herding) |
| `RandomPassableTarget` | Biased toward vault waypoints |

### Monster Awareness Triggers on Noticing Player

- **MonsBarking**: Barks (noise + fear on player); confused barkers bite tongue (self-damage)
- **MonsSpores**: Releases lignifying spores (unless player already lignified)
- **Dazzling (player trait)**: Monsters notice player instantly (no 1-turn delay)

### Monster Movement

- Wandering: follow precomputed path, spend turn recomputing if path stale
- Hunting: recompute path each turn; use ranged/rampage/swap attacks when applicable
- Out-of-view monsters: simplified swap-or-wait logic to avoid complex blockage resolution
- Monster position swapping: when two monsters want to trade positions

### Monster Attack Patterns (mirroring player)

- PatternRanged / PatternRangedRecoil: ranged attack in line of sight
- PatternCatch: catch player if doesn't resist move
- PatternSwap / PatternSwapDaze: swap positions with player
- PatternRampage: charge from range
- PatternFourDirsMons: four-directional charge attacks
- Adjacent melee: standard bump attack
- Charge: move then attack if player is in next tile

---

## 4. combat.go — Combat System (890 lines)

### Attack Kinds

```
AttackPlain       — adjacent melee, no movement
AttackCharge      — melee after moving (bonus for rampage)
AttackRanged      — ranged, no movement (swap occurs after)
AttackDrag        — crocodile backward drag
AttackSlap        — crocodile tail slap (multi-target)
AttackGale        — pushing gale environmental damage
AttackLightning   — lightning environmental damage
AttackFire        — fire environmental damage
AttackFireCatch   — catching fire damage
AttackPoison      — poison movement damage
AttackOther       — miscellaneous (not logged by InflictDamage)
```

### Combat Constants

```
HPCritical = 3                          — critical HP warning threshold
MaxAttackDamage = 3                     — max damage per attack roll
BerserkAttackBonus = 2                  — attack bonus while berserk
VampirismAttackBonus = 3                — attack bonus while vampiric
ConfusedImbalanceDefenseBonus = 2       — defense bonus when confused+imbalanced
LignificationDefenseBonus = 2           — defense bonus when lignified
```

### Stat Modifiers

**GetAttack()**:
- +BerserkAttackBonus if Berserk
- +VampirismAttackBonus if Vampirism
- /2 if Imbalanced

**GetDefense()**:
- +LignificationDefenseBonus if Lignified
- +ConfusedImbalanceDefenseBonus if Imbalanced AND Confused

**GetMaxHP()**:
- +HPBonus if Berserk
- +HPBonus if Lignified

### Damage Computation (`computeDamage`)

- Probability-based system using attack/defense rolls
- Always ≥5% miss chance
- 3 attack rolls using `probA[attack]` percentages
- Up to 2 defense absorption rolls using `probD[defense]`
- Damage capped at attack value
- **Probability tables**: `probA = [0, 35, 44, 52, 59, 66, 72, 78, 84, 90, 95, 100]`, `probD = [0, 29, 42, 53, 62, 70, 76, 81, 86, 90]`

### Damage Modifiers (AttackDamage)

- Rampage charge: +1 attack bonus (except vs lignified/resist-move)
- Catch ranged: +1 attack bonus
- MonsScales: +1 defense vs ranged
- MonsIgnoreDefense: defense = 0
- Focus / MonsFourHeaded: 4 attack rolls instead of 1
- Lignification cap: max 1 damage
- Gawalt (non-shadow): reduce damage by 1 if > 1
- Dig + charge OR AttackSlap OR Vampirism miss: +1 effective damage
- Berserk: +1 effective damage
- **Lucky roll for player**: reduces fatal damage with probability based on expected max damage (newbie protection)

### Bump Attack Effects (`BumpAttackActor`)

**Pre-attack:**
- Attacking monster becomes Hunting
- Dazzling redirect: attacks aimed at Dazzling player may hit monster behind player

**Pattern-specific effects (by attacker trait):**

| Pattern | Effect |
|---|---|
| Pushing / PushingCharge | Push foe 1 tile, imbalance, pierce to foe behind; move into vacated space |
| PatternSwap / PatternSwapDaze | Swap positions, swap clouds, blink adjacent actors, reverse direction; may daze |
| PatternCatch + Ranged | Pull foe adjacent, imbalance |
| PatternRangedRecoil | Recoil 1-2 tiles backward after attack |
| PatternCrocodile + Drag | Drag foe backward, imbalance, move self backward |
| PatternBat + Plain | Retreat 2 tiles after melee attack |
| PatternBat + Charge | Apply confusion to target |

**Shared extra attack effects:**
- VenomousMelee: chance to poison (melee only, probability scales with damage)
- BurningHits: chance to set target on fire (+ fire cloud on burnable terrain)

**Monster-specific extra attack effects (one per monster max):**

| Trait | Effect |
|---|---|
| MonsSpitFire + Ranged | Set target on fire; confused self-burn |
| MonsFear | Chance to frighten target |
| MonsConfusion | Chance to confuse target |
| MonsLignify | Chance to lignify target |
| MonsBerserking | Chance to berserk target |
| MonsBlink | Blink target to random FOV position |
| MonsTeleport | Teleport target away |

**Defensive effects:**
- DazingSpines: chance to daze attacker (melee only)
- Lignified + Afraid: getting hit triggers Berserk (cornered mechanic)

### Other Combat Systems

- `FourDirectionalAttack`: Attacks all adjacent foes; attack bonus scales with number of foes (+2/+3/+4 for 2+)
- `RangedTargetInDir`: Line-of-sight target finding along cardinal direction
- `ActorInRange`: Check if actor at position is reachable in a straight line
- `ExplosionAt`: Fire explosion at position — noise, fire clouds, fire status on adjacent
- `Dig / digAt`: Destroy walls (creates Rubble); translucent walls create poison cloud
- `BlinkPos`: Random free position within FOV
- `dazzlingRedirect`: Redirect attack to monster behind player

### Damage Types & Logging

- Player damage: tracked in Stats (Hurt, Damage, MapDamage)
- Hit/miss logging with charge variants
- Monster death: tracked in Stats, death animation, may trigger explosion (MonsExplodingDeath)
- Healing on combat (ModHealingCombat): random chance to heal 1 HP on kill
- Wizard mode: resurrects player on death
- Critical HP warning at HPCritical (3)
- Vampirism: heals attacker for damage dealt

---

## 5. auto.go — Autoexplore/Automove (218 lines)

### Auto Movement Modes

```
noAuto       — no automatic movement
autoRun      — run in a direction until corridor changes
autoTravel   — follow path to specific destination
autoExplore  — explore unknown tiles automatically
```

### Auto Struct

```go
type auto struct {
    mode       autoMode
    delta      gruid.Point      // running direction
    dirChange  bool             // smart corridor following
    dirn       dirNeighbors     // lateral passability config
    path       []gruid.Point    // travel path
    sources    []gruid.Point    // explore frontier sources
    PRauto     *paths.PathRange // cached BFS for exploration
    aemRebuild bool             // needs rebuild flag
}
```

### Dir Neighbors (for smart running)

```
dirFreeLaterals     — both sides open
dirBlockedLeft      — left side blocked
dirBlockedRight     — right side blocked
dirBlockedLaterals  — both sides blocked (corridor)
```

### Auto-Run Logic
- Smart corridor following: changes direction at turns
- Stops when lateral passability configuration changes (corridor opens/narrows)
- Stops on danger proximity

### Auto-Travel Logic
- Follow precomputed path step by step
- Stops on danger or path exhaustion

### Auto-Explore Logic
- BFS from exploration frontier (unknown tiles adjacent to known passable tiles)
- Greedy descent toward nearest frontier tile
- Stops on danger, unreachable tiles, or full exploration
- Avoids traps in pathing

### Safety: All auto modes
- Disabled while Sprinting, Digging, Lignified, or Poisoned
- Stop immediately when danger is nearby

---

## 6. target.go — Targeting System (95 lines)

### Structs

```go
type targeting struct {
    info   targInfo      // info about current target
    kb     bool          // keyboard examine mode active
    p      gruid.Point   // cursor position
    scroll int           // description scroll offset
}

type targInfo struct {
    entities []ID        // entities at cursor position
    cloud    CloudKind   // cloud at position (if any)
    sees     bool        // player can currently see position
    unknown  bool        // tile is unexplored
}
```

### Methods

| Method | Purpose |
|---|---|
| `HideCursor()` | Set cursor to InvalidPos |
| `SetCursor(p)` | Set cursor to specific position |
| `CancelExamine()` | Reset all targeting state |
| `Examine(p)` | Set cursor + compute travel path + update info |
| `RefreshExamineInfo()` | Refresh info for current cursor position |
| `updateTargInfo()` | Gather entities, cloud, visibility at cursor |

### Info Gathering
- In Wizard Reveal mode: shows actual actors and items
- Normal mode: shows only sensed actors and known items
- Cloud info: only shown if position is in FOV or Wizard Reveal

---

## 7. paths.go — Pathfinding (248 lines)

### Pathfinding Types

| Type | Interface | Used For |
|---|---|---|
| `MapPath` | `paths.Pather` | General map passability (BFS) |
| `MappingPath` | `paths.Pather` | Magic mapping, connected components |
| `MonPath` | `paths.Pather` + Cost + Estimation | Monster A* pathfinding |
| `tunnelPath` | `paths.Pather` + Cost + Estimation | Map generation tunnel routing |

### Player Pathfinding (`PlayerPath`)

- Uses JPS (Jump Point Search) for efficiency
- First tries path avoiding traps
- Falls back to trap-inclusive path if needed (or if destination has trap)
- RunicChicken players don't avoid traps
- Respects Dig status (can path through any in-map tile)
- Wizard mode: ignores known-tile requirement

### Monster Pathfinding (`MonPath`)

- **Wandering far from player**: JPS (cheaper, no actor avoidance)
- **Hunting or near player**: A* with actor-awareness
  - Cost 5 to move through other monsters (avoidance)
  - Cost 1 for free tiles
  - Confused monsters ignore actor costs
- **Hunting + MonsDig**: Can path through walls
- **Wandering**: Avoids traps; falls back to trap-inclusive if no path
- Paths are shuffled for non-predictable movement
- Boolean cache for fast actor-position lookups during A*

### Passability Functions

| Function | Behavior |
|---|---|
| `PlayerPassableNoTrapsFunc` | Known + passable + no traps (Dig: known + no traps) |
| `PlayerPassableFunc` | Known + passable (Dig: known only) |
| `WizardPlayerPassableNoTrapsFunc` | Passable + no traps (Dig: inMap + no traps) |
| `WizardPlayerPassableFunc` | Passable (Dig: inMap only) |

### Tunnel Pathfinding (Map Generation)

- Costs favor internal walls over vault cells
- Vault entries allowed but expensive (10)
- Non-entry vault cells very expensive (100)
- Lateral wall counting reduces cost for internal tunnels
- Edge-of-map penalty for aesthetics

---

## Cross-Cutting: Complete Status Effects

| Status | Type | Description |
|---|---|---|
| StatusBerserk | Buff | +2 Attack, +1 effective damage, temp HP bonus, fear immunity; followed by Poison |
| StatusClarity | Buff | Protects from Confusion/Daze/Berserk; sense nearby monsters |
| StatusConfusion | Debuff | Spirit abilities hurt self; doubles Imbalance/Daze/Fear duration |
| StatusDaze | Debuff | Cannot act (wait or eat only) |
| StatusDig | Buff | Walk through walls, +1 charge damage, guaranteed pushing |
| StatusDisorient | Buff | Enemies in view move wrong direction |
| StatusFear | Debuff | Cannot attack or approach foes |
| StatusFire | Debuff | Burns for damage when stationary or in fire cloud |
| StatusFocus | Buff | Attack with all 4 heads (4× damage rolls) |
| StatusFoggySkin | Buff | Exudes fog; protects from Fire and Lignification |
| StatusGardener | Buff | Grows foliage in 2-tile radius each turn |
| StatusGluttony | Debuff | Must eat before expiry or auto-eat/get confused |
| StatusImbalance | Debuff | Halves attack; +2 defense if also confused |
| StatusLignification | Mixed | +2 Defense, caps damage at 1, temp HP, prevents movement; followed by Imbalance |
| StatusPoison | Debuff | Moving hurts; exudes confusing toxins on expiry |
| StatusShadow | Buff | Hidden from non-hunting monsters, silent combat, pass through translucent walls |
| StatusSprint | Buff | Move 2-3× speed, jump over foes; cancels normal attack |
| StatusTimeStop | Buff | Time frozen for everyone else |
| StatusVampirism | Buff | +3 Attack, guaranteed hit, heals for damage dealt |

## Cross-Cutting: Complete Trait List

### Player/Shared Traits (28)
Player, PatternBat, PatternCatch, PatternCrocodile, PatternFourDirs, PatternFourDirsMons, PatternRampage, PatternRanged, PatternRangedRecoil, PatternSwap, PatternSwapDaze, BadHearing, BurningHits, Dazzling, DazingSpines, Gawalt, Gluttony, GoodHearing, GoodSmell, NocturnalFlying, Pushing, PushingCharge, ScaryRoar, Elephanty, RunicChicken, TrailingCloud, VenomousMelee, WoodyLegs

### Resistance/Vulnerability Traits (7)
VulnerabilityFire, ResistanceConfusion, ResistanceDaze, ResistanceFear, ResistanceFire, ResistanceImbalance, ResistancePoison

### Monster-Only Traits (24)
MonsBarking, MonsBerserking, MonsBlink, MonsConfusion, MonsDig, MonsExplodingDeath, MonsFear, MonsFourHeaded, MonsHungry, MonsIgnoreDefense, MonsLignify, MonsSpores, MonsMusic, MonsScales, MonsSpitFire, MonsTeleport, MonsImmunityConfusion, MonsImmunityDaze, MonsImmunityFear, MonsImmunityFire, MonsImmunityImbalance, MonsImmunityLignification, MonsImmunityPoison

### Monster Noise Traits (6)
MonsCreep, MonsHeavyFootsteps, MonsLightFootsteps, MonsSilent, MonsWingFlap, MonsNotable

## Cross-Cutting: Item Types

| Type | Description |
|---|---|
| Spirit | Equippable spirit (upgradeable once); provides passive traits + active ability |
| EmptyTotem | Empty totem pedestal |
| Comestible | Food item (eat for effect) |
| Menhir | Activatable stone (Earth, Warping, Poison, Fire variants) |
| Portal | Level transition |
| CorruptionOrb | Malfunctioning portal variant |
| RunicTrap | Static trap triggered by stepping |

## Cross-Cutting: Noise Types

NoiseBark, NoiseCombat, NoiseDig, NoiseExplosion, NoiseWind, NoiseChomp, NoiseMusic

## Cross-Cutting: Cloud Types

CloudFire (burns + blocks vision), CloudPoison (poisons + blocks vision), normal/dust clouds
