//! Map state and generation.

use gruid_core::{Point, Range};
use gruid_paths::{AstarPather, PathRange, Pather, WeightedPather};
use gruid_rl::{
    fov::FOV,
    grid::Grid as RlGrid,
    mapgen::{CellularAutomataRule, MapGen},
    vault::Vault,
};
use rand::Rng;

use crate::terrain::*;

pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 21;

/// Maximum FOV range.
pub const MAX_FOV_RANGE: i32 = 8;

/// The game map for a single dungeon level.
pub struct GameMap {
    pub terrain: RlGrid,
    pub known_terrain: RlGrid,
    pub fov: FOV,
    pub fov_points: Vec<Point>,
    pub waypoints: Vec<Point>,
    pub level: i32,
}

impl GameMap {
    pub fn new() -> Self {
        let terrain = RlGrid::new(MAP_WIDTH, MAP_HEIGHT);
        let known_terrain = RlGrid::new(MAP_WIDTH, MAP_HEIGHT);
        known_terrain.fill(UNKNOWN);
        let fov_range = Range::new(
            -MAX_FOV_RANGE,
            -MAX_FOV_RANGE,
            MAX_FOV_RANGE + 1,
            MAX_FOV_RANGE + 1,
        );
        Self {
            terrain,
            known_terrain,
            fov: FOV::new(fov_range),
            fov_points: Vec::new(),
            waypoints: Vec::new(),
            level: 1,
        }
    }

    /// Whether position p is passable terrain.
    pub fn passable(&self, p: Point) -> bool {
        self.terrain.at(p).is_some_and(passable)
    }

    /// Whether position p is within map bounds.
    pub fn in_map(&self, p: Point) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < MAP_WIDTH && p.y < MAP_HEIGHT
    }
}

// ---------------------------------------------------------------------------
// Map generation
// ---------------------------------------------------------------------------

/// Vault data files.
const SMALL_VAULTS: &str = include_str!("../data/small-vaults.txt");
const BIG_VAULTS: &str = include_str!("../data/big-vaults.txt");

/// Split vault template file into individual vault strings.
fn split_vaults(s: &str) -> Vec<String> {
    let s = s.replace(' ', "");
    s.trim().split("\n\n").map(|v| v.to_string()).collect()
}

/// Vault placement position in the map.
struct VaultInfo {
    pos: Point,
    w: i32,
    h: i32,
    entries: Vec<VaultEntry>,
    places: Vec<VaultPlace>,
    vault: Vault,
    tunnels: i32,
}

struct VaultEntry {
    pos: Point,
    used: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlaceKind {
    Waypoint,
    Item,
    Static,
}

struct VaultPlace {
    pos: Point,
    kind: PlaceKind,
    #[allow(dead_code)]
    used: bool,
}

enum Placement {
    Random,
    Center,
    Edge,
}

/// Internal map generator state.
struct MapGenState {
    terrain: RlGrid,
    vaults: Vec<VaultInfo>,
    tunnel: Vec<bool>,
    vault_mask: Vec<bool>,
    item_place: Vec<bool>,
}

impl MapGenState {
    fn new(terrain: RlGrid) -> Self {
        let n = (MAP_WIDTH * MAP_HEIGHT) as usize;
        Self {
            terrain,
            vaults: Vec::new(),
            tunnel: vec![false; n],
            vault_mask: vec![false; n],
            item_place: vec![false; n],
        }
    }

    fn idx(&self, p: Point) -> usize {
        (p.y * MAP_WIDTH + p.x) as usize
    }

    fn in_vault(&self, p: Point) -> bool {
        let i = self.idx(p);
        i < self.vault_mask.len() && self.vault_mask[i]
    }

    fn in_tunnel(&self, p: Point) -> bool {
        let i = self.idx(p);
        i < self.tunnel.len() && self.tunnel[i]
    }
}

fn vault_center(v: &VaultInfo) -> Point {
    Point::new(v.pos.x + v.w / 2, v.pos.y + v.h / 2)
}

fn vault_distance(v1: &VaultInfo, v2: &VaultInfo) -> i32 {
    gruid_paths::manhattan(vault_center(v1), vault_center(v2))
}

fn in_map(p: Point) -> bool {
    p.x >= 0 && p.y >= 0 && p.x < MAP_WIDTH && p.y < MAP_HEIGHT
}

/// Generate a complete map level. Returns (terrain, waypoints, entity spawn points).
pub fn generate_map(rng: &mut impl Rng, pr: &mut PathRange) -> (RlGrid, Vec<Point>, Vec<Point>) {
    loop {
        let terrain = RlGrid::new(MAP_WIDTH, MAP_HEIGHT);
        terrain.fill(WALL);
        let mut mg = MapGenState::new(terrain);

        // 1. Generate cave base
        gen_cellular_automata(&mut mg, rng);

        // 2. Generate foliage overlay
        gen_foliage(&mut mg, rng);

        // 3. Place vaults
        let small_templates = split_vaults(SMALL_VAULTS);
        let big_templates = split_vaults(BIG_VAULTS);

        let big_center = rng.random_range(0..2) == 0;
        if big_center {
            gen_vaults(&mut mg, rng, &big_templates, 1, Placement::Center);
            gen_vaults(&mut mg, rng, &small_templates, 1, Placement::Edge);
        } else {
            gen_vaults(&mut mg, rng, &big_templates, 1, Placement::Edge);
            gen_vaults(&mut mg, rng, &small_templates, 1, Placement::Center);
        }
        gen_vaults(&mut mg, rng, &big_templates, 1, Placement::Random);
        let n_small = 4 + rng.random_range(0..2);
        gen_vaults(&mut mg, rng, &small_templates, n_small, Placement::Random);

        // 4. Connect vaults with tunnels
        connect_all_vaults(&mut mg, rng, pr);

        // 5. Collect waypoints
        let mut waypoints = Vec::new();
        for vi in &mg.vaults {
            for pl in &vi.places {
                if pl.kind == PlaceKind::Waypoint {
                    let t = mg.terrain.at(pl.pos).unwrap_or(WALL);
                    if passable(t) {
                        waypoints.push(pl.pos);
                    }
                }
            }
        }

        // 6. Keep connected
        if waypoints.is_empty() {
            continue;
        }
        let seed = waypoints[rng.random_range(0..waypoints.len())];
        let pass = |p: Point| -> bool { mg.terrain.at(p).is_some_and(passable) };
        pr.cc_map(&MappingPath { passable: pass }, seed);
        let rl_mg = MapGen::with_grid(mg.terrain.clone(), rand::rng());
        let ntiles = rl_mg.keep_connected(pr, seed, WALL);
        // Sync back — keep_connected wrote to rl_mg.grid
        mg.terrain.copy_from(&rl_mg.grid);

        if ntiles < 1000 {
            continue;
        }

        // 7. Find monster spawn points (random passable positions away from waypoints)
        let mut spawns = Vec::new();
        for _ in 0..20 {
            if let Some(p) = random_passable(&mg.terrain, rng) {
                spawns.push(p);
            }
        }

        return (mg.terrain, waypoints, spawns);
    }
}

/// Find a random passable position.
fn random_passable(terrain: &RlGrid, rng: &mut impl Rng) -> Option<Point> {
    for _ in 0..1000 {
        let p = Point::new(
            rng.random_range(0..MAP_WIDTH),
            rng.random_range(0..MAP_HEIGHT),
        );
        if terrain.at(p) == Some(FLOOR) {
            return Some(p);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Cellular Automata
// ---------------------------------------------------------------------------

fn gen_cellular_automata(mg: &mut MapGenState, rng: &mut impl Rng) {
    let rules = vec![
        CellularAutomataRule {
            w_cutoff1: 5,
            w_cutoff2: 2,
            reps: 4,
            walls_out_of_range: true,
        },
        CellularAutomataRule {
            w_cutoff1: 5,
            w_cutoff2: 25,
            reps: 3,
            walls_out_of_range: true,
        },
    ];
    let n = match rng.random_range(0..3u32) {
        0 => 0.42,
        1 => 0.45,
        _ => 0.48,
    };
    let mut map_gen = MapGen::with_grid(mg.terrain.clone(), rand::rng());
    map_gen.cellular_automata_cave(WALL, FLOOR, n, &rules);
    mg.terrain.copy_from(&map_gen.grid);
}

// ---------------------------------------------------------------------------
// Foliage overlay
// ---------------------------------------------------------------------------

fn gen_foliage(mg: &mut MapGenState, rng: &mut impl Rng) {
    let foliage_grid = RlGrid::new(MAP_WIDTH, MAP_HEIGHT);
    let rules = vec![
        CellularAutomataRule {
            w_cutoff1: 5,
            w_cutoff2: 2,
            reps: 4,
            walls_out_of_range: true,
        },
        CellularAutomataRule {
            w_cutoff1: 5,
            w_cutoff2: 25,
            reps: 2,
            walls_out_of_range: true,
        },
    ];
    let winit = match rng.random_range(0..3u32) {
        0 => 0.54,
        1 => 0.53,
        _ => 0.55,
    };
    let mut fol_gen = MapGen::with_grid(foliage_grid, rand::rng());
    fol_gen.cellular_automata_cave(WALL, FOLIAGE, winit, &rules);

    // Apply foliage where both terrain is floor and overlay is foliage
    let sz = mg.terrain.size();
    for y in 0..sz.y {
        for x in 0..sz.x {
            let p = Point::new(x, y);
            if mg.terrain.at(p) == Some(FLOOR) && fol_gen.grid.at(p) == Some(FOLIAGE) {
                mg.terrain.set(p, FOLIAGE);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Vault placement
// ---------------------------------------------------------------------------

fn gen_vaults(
    mg: &mut MapGenState,
    rng: &mut impl Rng,
    templates: &[String],
    n: usize,
    placement: Placement,
) {
    if templates.is_empty() {
        return;
    }
    for _ in 0..n {
        let mut placed = false;
        for _ in 0..500 {
            let tpl = &templates[rng.random_range(0..templates.len())];
            for _ in 0..10 {
                if let Some(vi) = try_place_vault(mg, rng, tpl, &placement) {
                    mg.vaults.push(vi);
                    placed = true;
                    break;
                }
            }
            if placed {
                break;
            }
        }
    }
}

fn try_place_vault(
    mg: &mut MapGenState,
    rng: &mut impl Rng,
    tpl: &str,
    placement: &Placement,
) -> Option<VaultInfo> {
    let pos = match placement {
        Placement::Random => Point::new(
            rng.random_range(0..MAP_WIDTH.saturating_sub(1).max(1)),
            rng.random_range(0..MAP_HEIGHT.saturating_sub(1).max(1)),
        ),
        Placement::Center => Point::new(
            MAP_WIDTH / 2 - 4 + rng.random_range(0..5),
            MAP_HEIGHT / 2 - 3 + rng.random_range(0..4),
        ),
        Placement::Edge => {
            if rng.random_range(0..2) == 0 {
                Point::new(
                    rng.random_range(0..(MAP_WIDTH / 4).max(1)),
                    rng.random_range(0..MAP_HEIGHT.saturating_sub(1).max(1)),
                )
            } else {
                Point::new(
                    3 * MAP_WIDTH / 4 + rng.random_range(0..(MAP_WIDTH / 4).max(1)) - 1,
                    rng.random_range(0..MAP_HEIGHT.saturating_sub(1).max(1)),
                )
            }
        }
    };

    let mut vault = match Vault::new(tpl) {
        Ok(v) => v,
        Err(_) => return None,
    };

    // Random rotation/reflection
    let w = vault.size().x;
    let h = vault.size().y;
    let mut drev = 2;
    if w > h + 2 {
        drev += (w - h - 2).min(4);
    }
    if rng.random_range(0..drev) == 0 {
        if rng.random_range(0..2) == 0 {
            vault.reflect();
            vault.rotate(1 + 2 * rng.random_range(0..2));
        } else {
            vault.rotate(1 + 2 * rng.random_range(0..2));
        }
    } else if rng.random_range(0..2) == 0 {
        vault.reflect();
        vault.rotate(2 * rng.random_range(0..2));
    } else {
        vault.rotate(2 * rng.random_range(0..2));
    }

    let vw = vault.size().x;
    let vh = vault.size().y;
    if vw == 0 || vh == 0 {
        return None;
    }

    // Check fit
    if MAP_WIDTH - pos.x < vw || MAP_HEIGHT - pos.y < vh {
        return None;
    }
    for i in (pos.x - 1)..=(pos.x + vw) {
        for j in (pos.y - 1)..=(pos.y + vh) {
            let p = Point::new(i, j);
            if in_map(p) && mg.in_vault(p) {
                return None;
            }
        }
    }

    // Dig vault
    let mut vi = VaultInfo {
        pos,
        w: vw,
        h: vh,
        entries: Vec::new(),
        places: Vec::new(),
        vault,
        tunnels: 0,
    };
    dig_vault(mg, &mut vi, rng);
    Some(vi)
}

fn dig_vault(mg: &mut MapGenState, vi: &mut VaultInfo, rng: &mut impl Rng) {
    vi.vault.iter(|p, c| {
        let q = Point::new(vi.pos.x + p.x, vi.pos.y + p.y);
        if in_map(q) && c != '?' {
            let idx = mg.idx(q);
            mg.vault_mask[idx] = true;
        }
        match c {
            '.' | '!' | '-' | '>' | 'W' => {
                if in_map(q) {
                    mg.terrain.set(q, FLOOR);
                }
            }
            '#' | '+' => {
                if in_map(q) {
                    mg.terrain.set(q, WALL);
                }
            }
            '$' => {
                if in_map(q) {
                    mg.terrain.set(q, TRANSLUCENT_WALL);
                }
            }
            '%' => {
                if in_map(q) {
                    if rng.random_range(0..2) == 0 {
                        mg.terrain.set(q, WALL);
                    } else {
                        mg.terrain.set(q, TRANSLUCENT_WALL);
                    }
                }
            }
            '&' => {
                if in_map(q) {
                    let choices = [WALL, TRANSLUCENT_WALL, FOLIAGE, RUBBLE, FLOOR];
                    mg.terrain.set(q, choices[rng.random_range(0..5)]);
                }
            }
            '"' => {
                if in_map(q) {
                    mg.terrain.set(q, FOLIAGE);
                }
            }
            '^' => {
                if in_map(q) {
                    mg.terrain.set(q, RUBBLE);
                }
            }
            ':' => {
                if in_map(q) {
                    let choices = [FLOOR, FOLIAGE, RUBBLE];
                    mg.terrain.set(q, choices[rng.random_range(0..3)]);
                }
            }
            '?' => {}
            _ => {}
        }

        // Record special places
        match c {
            'W' => vi.places.push(VaultPlace {
                pos: q,
                kind: PlaceKind::Waypoint,
                used: false,
            }),
            '!' => {
                vi.places.push(VaultPlace {
                    pos: q,
                    kind: PlaceKind::Item,
                    used: false,
                });
                if in_map(q) {
                    let idx = mg.idx(q);
                    mg.item_place[idx] = true;
                }
            }
            '>' => {
                vi.places.push(VaultPlace {
                    pos: q,
                    kind: PlaceKind::Static,
                    used: false,
                });
                if in_map(q) {
                    let idx = mg.idx(q);
                    mg.item_place[idx] = true;
                }
            }
            '+' | '-' => {
                if q.x > 0 && q.x < MAP_WIDTH - 1 && q.y > 0 && q.y < MAP_HEIGHT - 1 {
                    vi.entries.push(VaultEntry {
                        pos: q,
                        used: false,
                    });
                }
            }
            _ => {}
        }
    });
}

// ---------------------------------------------------------------------------
// Tunnel connection
// ---------------------------------------------------------------------------

fn connect_all_vaults(mg: &mut MapGenState, rng: &mut impl Rng, pr: &mut PathRange) {
    // Sort vaults by distance to map center (closest first)
    let center = Point::new(MAP_WIDTH / 2, MAP_HEIGHT / 2);
    mg.vaults.sort_by_key(|v| {
        let c = vault_center(v);
        gruid_paths::manhattan(c, center)
    });

    // Primary tunnels: connect each vault to nearest already-connected
    for i in 1..mg.vaults.len() {
        let nearest_idx = find_nearest_connected(&mg.vaults, i);
        connect_vault_pair(mg, rng, pr, i, nearest_idx);
    }

    // Extra tunnels
    let extra = match rng.random_range(0..6) {
        0 => 3,
        1 => 5,
        _ => 4,
    };
    let mut count = 0;
    let nv = mg.vaults.len();
    for n in 0..2 {
        for i in 0..nv {
            if count >= extra {
                break;
            }
            if mg.vaults[i].tunnels > n + 1 {
                continue;
            }
            let near_idx = find_near_vault(mg, rng, i);
            if near_idx != i {
                connect_vault_pair(mg, rng, pr, i, near_idx);
                count += 1;
            }
        }
    }
}

fn find_nearest_connected(vaults: &[VaultInfo], i: usize) -> usize {
    let mut best = 0;
    let mut best_dist = i32::MAX;
    for j in 0..i {
        let d = vault_distance(&vaults[i], &vaults[j]);
        if d < best_dist {
            best_dist = d;
            best = j;
        }
    }
    best
}

fn find_near_vault(mg: &MapGenState, rng: &mut impl Rng, vi_idx: usize) -> usize {
    let nv = mg.vaults.len();
    if nv <= 1 {
        return vi_idx;
    }
    // Sort by distance
    let mut indices: Vec<usize> = (0..nv).collect();
    indices.sort_by_key(|&j| vault_distance(&mg.vaults[vi_idx], &mg.vaults[j]));
    let mut result = vi_idx;
    for (rank, &j) in indices.iter().enumerate() {
        if j == vi_idx {
            continue;
        }
        if result != vi_idx && rank > 0 && vault_distance(&mg.vaults[vi_idx], &mg.vaults[j]) > 40 {
            break;
        }
        result = j;
        if rank > 2 || (rank == 2 && rng.random_range(0..4) > 0) || rng.random_range(0..2) == 0 {
            break;
        }
    }
    result
}

fn connect_vault_pair(
    mg: &mut MapGenState,
    rng: &mut impl Rng,
    pr: &mut PathRange,
    v1_idx: usize,
    v2_idx: usize,
) {
    if v1_idx == v2_idx {
        return;
    }
    if mg.vaults[v1_idx].entries.is_empty() || mg.vaults[v2_idx].entries.is_empty() {
        return;
    }

    let e1_idx = unused_entry(&mg.vaults[v1_idx], rng);
    let e1_pos = mg.vaults[v1_idx].entries[e1_idx].pos;
    let e2_idx = unused_entry(&mg.vaults[v2_idx], rng);
    let e2_pos = mg.vaults[v2_idx].entries[e2_idx].pos;

    let tp = TunnelPather {
        terrain: &mg.terrain,
        vault_mask: &mg.vault_mask,
        tunnel: &mg.tunnel,
    };
    let path = match pr.astar_path(&tp, e1_pos, e2_pos) {
        Some(p) if !p.is_empty() => p,
        _ => return,
    };

    let fill = match rng.random_range(0..8u32) {
        0 => RUBBLE,
        1 => FOLIAGE,
        _ => FLOOR,
    };
    for &p in &path {
        let t = mg.terrain.at(p).unwrap_or(WALL);
        if !passable(t) {
            if rng.random_range(0..2) == 0 {
                mg.terrain.set(p, FLOOR);
            } else {
                mg.terrain.set(p, fill);
            }
        }
        let idx = mg.idx(p);
        mg.tunnel[idx] = true;
    }

    mg.vaults[v1_idx].entries[e1_idx].used = true;
    mg.vaults[v2_idx].entries[e2_idx].used = true;
    mg.vaults[v1_idx].tunnels += 1;
    mg.vaults[v2_idx].tunnels += 1;
}

fn unused_entry(vi: &VaultInfo, rng: &mut impl Rng) -> usize {
    let unused: Vec<usize> = vi
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.used)
        .map(|(i, _)| i)
        .collect();
    if unused.is_empty() {
        rng.random_range(0..vi.entries.len())
    } else {
        unused[rng.random_range(0..unused.len())]
    }
}

// ---------------------------------------------------------------------------
// Pathfinding helpers
// ---------------------------------------------------------------------------

/// Tunnel pathfinder for map generation — A* with wall-aware costs.
struct TunnelPather<'a> {
    terrain: &'a RlGrid,
    vault_mask: &'a [bool],
    tunnel: &'a [bool],
}

impl TunnelPather<'_> {
    fn idx(p: Point) -> usize {
        (p.y * MAP_WIDTH + p.x) as usize
    }
}

impl Pather for TunnelPather<'_> {
    fn neighbors(&self, p: Point, buf: &mut Vec<Point>) {
        for &d in &[
            Point::new(1, 0),
            Point::new(-1, 0),
            Point::new(0, 1),
            Point::new(0, -1),
        ] {
            let np = p.shift(d.x, d.y);
            if in_map(np) {
                buf.push(np);
            }
        }
    }
}

impl WeightedPather for TunnelPather<'_> {
    fn cost(&self, from: Point, to: Point) -> i32 {
        let idx = Self::idx(to);
        if idx < self.vault_mask.len() && self.vault_mask[idx] {
            if !(idx < self.tunnel.len() && self.tunnel[idx]) {
                return 100;
            }
            return 10;
        }
        let t = self.terrain.at(to).unwrap_or(WALL);
        if passable(t) {
            if !(idx < self.tunnel.len() && self.tunnel[idx]) && t != FLOOR {
                return 2;
            }
            return 1;
        }
        // Wall — favor internal walls
        let mut c = 3;
        let tf = self.terrain.at(from).unwrap_or(WALL);
        if from.x == MAP_WIDTH - 1
            || from.x == 0
            || from.y == 0
            || from.y == MAP_HEIGHT - 1
            || passable(tf)
        {
            c += 1;
        }
        let wc = count_lateral_walls(self.terrain, from, to);
        (c - wc).max(1)
    }
}

impl AstarPather for TunnelPather<'_> {
    fn estimate(&self, from: Point, to: Point) -> i32 {
        gruid_paths::manhattan(from, to)
    }
}

fn count_lateral_walls(terrain: &RlGrid, from: Point, to: Point) -> i32 {
    let dir = Point::new(to.x - from.x, to.y - from.y);
    let mut n = 0;
    for &d in &[
        Point::new(1, 0),
        Point::new(-1, 0),
        Point::new(0, 1),
        Point::new(0, -1),
    ] {
        let p = Point::new(to.x + d.x, to.y + d.y);
        if Point::new(p.x - to.x, p.y - to.y) == dir || p == from {
            continue;
        }
        if terrain.at(p) == Some(WALL) {
            n += 1;
        }
    }
    n
}

/// Simple cardinal pather that checks passability.
pub struct MappingPath<F: Fn(Point) -> bool> {
    pub passable: F,
}

impl<F: Fn(Point) -> bool> Pather for MappingPath<F> {
    fn neighbors(&self, p: Point, buf: &mut Vec<Point>) {
        if !(self.passable)(p) {
            return;
        }
        for &d in &[
            Point::new(1, 0),
            Point::new(-1, 0),
            Point::new(0, 1),
            Point::new(0, -1),
        ] {
            let np = p.shift(d.x, d.y);
            if in_map(np) {
                buf.push(np);
            }
        }
    }
}

/// Simple passability pather for monster A*.
pub struct MonsterPather<'a> {
    pub terrain: &'a RlGrid,
    pub actor_positions: &'a [bool],
    pub player_pos: Point,
}

impl Pather for MonsterPather<'_> {
    fn neighbors(&self, p: Point, buf: &mut Vec<Point>) {
        for &d in &[
            Point::new(1, 0),
            Point::new(-1, 0),
            Point::new(0, 1),
            Point::new(0, -1),
        ] {
            let np = p.shift(d.x, d.y);
            if self.terrain.at(np).is_some_and(passable) {
                buf.push(np);
            }
        }
    }
}

impl WeightedPather for MonsterPather<'_> {
    fn cost(&self, _from: Point, to: Point) -> i32 {
        let idx = (to.y * MAP_WIDTH + to.x) as usize;
        if idx < self.actor_positions.len() && self.actor_positions[idx] && to != self.player_pos {
            return 5;
        }
        1
    }
}

impl AstarPather for MonsterPather<'_> {
    fn estimate(&self, from: Point, to: Point) -> i32 {
        gruid_paths::manhattan(from, to)
    }
}
