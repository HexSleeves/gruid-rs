//! Core game state.

use gruid_core::Point;
use gruid_paths::PathRange;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::combat;
use crate::entity::*;
use crate::gamemap::*;
use crate::log::{GameLog, LogStyle};
use crate::terrain::*;

/// Core game state (separate from UI model).
pub struct Game {
    pub entities: Vec<Option<Entity>>,
    pub map: GameMap,
    pub pr: PathRange,
    pub turn: i32,
    pub log: GameLog,
    pub rng: SmallRng,
}

impl Game {
    pub fn new() -> Self {
        let rng = SmallRng::from_os_rng();
        let map = GameMap::new();
        let map_range = gruid_core::Range::new(0, 0, MAP_WIDTH, MAP_HEIGHT);
        let pr = PathRange::new(map_range);
        Self {
            entities: Vec::new(),
            map,
            pr,
            turn: 0,
            log: GameLog::new(),
            rng,
        }
    }

    /// Initialize a new game: generate map, place player and monsters.
    pub fn init(&mut self) {
        // Allocate entity slots: inventory (0..7) + player (8)
        self.entities = vec![None; INVENTORY_SIZE + 1];

        // Generate map
        let (terrain, waypoints, spawns) = generate_map(&mut self.rng, &mut self.pr);
        self.map.terrain.copy_from(&terrain);
        self.map.known_terrain.fill(UNKNOWN);
        self.map.waypoints = waypoints;

        // Place player at a random waypoint
        let player_pos = if !self.map.waypoints.is_empty() {
            let idx = self.rng.random_range(0..self.map.waypoints.len());
            self.map.waypoints[idx]
        } else {
            // Fallback: find any floor tile
            random_floor(&self.map.terrain, &mut self.rng)
        };

        // Create player entity
        self.entities[PLAYER_ID] = Some(Entity {
            name: "player".to_string(),
            ch: '@',
            pos: player_pos,
            known_pos: player_pos,
            seen: true,
            role: Role::Actor(Actor::new(2, 1, 9)),
        });

        // Spawn monsters
        self.spawn_monsters(&spawns);

        // Update FOV
        self.update_fov();

        self.log.log("Welcome to Shamogu! Press ? for help.");
    }

    /// Spawn monsters on the map at given positions.
    fn spawn_monsters(&mut self, spawns: &[Point]) {
        // Level 1 spawn counts: 3 early, 2 mid
        let n_early = 3;
        let n_mid = 2;

        for (i, &pos) in spawns.iter().enumerate() {
            if i >= n_early + n_mid {
                break;
            }
            // Don't spawn on player
            if Some(pos) == self.player().map(|e| e.pos) {
                continue;
            }
            // Don't spawn on walls
            if !self.map.passable(pos) {
                continue;
            }

            let kind = if i < n_early {
                EARLY_MONSTERS[self.rng.random_range(0..EARLY_MONSTERS.len())]
            } else {
                MID_MONSTERS[self.rng.random_range(0..MID_MONSTERS.len())]
            };
            let data = monster_data(kind);
            let actor = Actor::new_monster(data, kind);
            let entity = Entity {
                name: data.name.to_string(),
                ch: data.ch,
                pos,
                known_pos: INVALID_POS,
                seen: false,
                role: Role::Actor(actor),
            };
            self.entities.push(Some(entity));
        }
    }

    // -------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------

    /// Get the player entity.
    pub fn player(&self) -> Option<&Entity> {
        self.entities.get(PLAYER_ID).and_then(|e| e.as_ref())
    }

    /// Get the player position.
    pub fn pp(&self) -> Point {
        self.player().map_or(INVALID_POS, |e| e.pos)
    }

    /// Get the player's Actor component.
    pub fn player_actor(&self) -> Option<&Actor> {
        self.player().and_then(|e| e.actor())
    }

    /// Whether the player is alive.
    pub fn player_alive(&self) -> bool {
        self.player().is_some_and(|e| e.is_alive())
    }

    /// Iterate alive map entities (including player).
    pub fn alive_actors(&self) -> impl Iterator<Item = (Id, &Entity)> {
        self.entities
            .iter()
            .enumerate()
            .skip(FIRST_MAP_ID)
            .filter_map(|(i, e)| e.as_ref().filter(|e| e.is_alive()).map(|e| (i, e)))
    }

    /// Iterate alive monsters (non-player actors).
    pub fn monsters(&self) -> impl Iterator<Item = (Id, &Entity)> {
        self.entities
            .iter()
            .enumerate()
            .skip(FIRST_MAP_ID + 1)
            .filter_map(|(i, e)| e.as_ref().filter(|e| e.is_alive()).map(|e| (i, e)))
    }

    /// Find alive actor at position.
    pub fn actor_at(&self, pos: Point) -> Option<Id> {
        for (id, e) in self.alive_actors() {
            if e.pos == pos {
                return Some(id);
            }
        }
        None
    }

    // -------------------------------------------------------------------
    // Actions
    // -------------------------------------------------------------------

    /// Try to move the player in a direction. Returns true if turn ended.
    pub fn move_player(&mut self, dx: i32, dy: i32) -> bool {
        let pp = self.pp();
        let target = pp.shift(dx, dy);

        // Check for monster at target
        if let Some(target_id) = self.actor_at(target) {
            if target_id != PLAYER_ID {
                return self.attack(PLAYER_ID, target_id);
            }
        }

        // Check passability
        if !self.map.passable(target) {
            return false;
        }

        if let Some(Some(player)) = self.entities.get_mut(PLAYER_ID) {
            player.pos = target;
        }
        true
    }

    /// Execute an attack from attacker to defender.
    pub fn attack(&mut self, attacker_id: Id, defender_id: Id) -> bool {
        let (atk_name, atk_stat) = {
            let e = self.entities[attacker_id].as_ref().unwrap();
            let a = e.actor().unwrap();
            (e.name.clone(), a.attack)
        };
        let (def_name, def_stat) = {
            let e = self.entities[defender_id].as_ref().unwrap();
            let a = e.actor().unwrap();
            (e.name.clone(), a.defense)
        };

        let dmg = combat::compute_damage(&mut self.rng, atk_stat, def_stat);

        if dmg > 0 {
            // Apply damage
            let dead = {
                let defender = self.entities[defender_id].as_mut().unwrap();
                let actor = defender.actor_mut().unwrap();
                actor.hp -= dmg;
                actor.hp <= 0
            };

            let style = if defender_id == PLAYER_ID {
                LogStyle::HurtPlayer
            } else {
                LogStyle::HurtMonster
            };
            self.log.log_styled(
                &format!("{} hits {} for {} damage.", atk_name, def_name, dmg),
                style,
            );

            if dead {
                self.kill(defender_id);
            }
        } else {
            self.log
                .log(&format!("{} attacks {} but misses.", atk_name, def_name));
        }

        true
    }

    /// Kill an entity.
    fn kill(&mut self, id: Id) {
        let name = self.entities[id]
            .as_ref()
            .map(|e| e.name.clone())
            .unwrap_or_default();

        if id == PLAYER_ID {
            self.log.log_styled("You die...", LogStyle::HurtPlayer);
        } else {
            self.log
                .log_styled(&format!("{} dies.", name), LogStyle::HurtMonster);
            // Remove from map
            if let Some(Some(entity)) = self.entities.get_mut(id) {
                entity.pos = INVALID_POS;
            }
        }
    }

    // -------------------------------------------------------------------
    // Monster AI
    // -------------------------------------------------------------------

    /// Process all monster turns.
    pub fn process_monsters(&mut self) {
        // Build actor position cache for pathfinding
        let n = (MAP_WIDTH * MAP_HEIGHT) as usize;
        let mut actor_pos = vec![false; n];
        for (_, e) in self.alive_actors() {
            let idx = (e.pos.y * MAP_WIDTH + e.pos.x) as usize;
            if idx < n {
                actor_pos[idx] = true;
            }
        }

        // Collect monster IDs first to avoid borrow issues
        let monster_ids: Vec<Id> = self.monsters().map(|(id, _)| id).collect();

        let pp = self.pp();

        for id in monster_ids {
            if !self.player_alive() {
                break;
            }
            let entity = match &self.entities[id] {
                Some(e) if e.is_alive() => e,
                _ => continue,
            };
            let actor = match entity.actor() {
                Some(a) => a,
                None => continue,
            };

            let mon_pos = entity.pos;
            let behavior = match &actor.behavior {
                Some(b) => b.clone(),
                None => continue,
            };

            // Check if monster can see the player
            let sees_player =
                self.in_fov(mon_pos) && gruid_paths::manhattan(mon_pos, pp) <= MAX_FOV_RANGE;

            let mut new_state = behavior.state;
            let mut new_target = behavior.target;

            if sees_player {
                new_state = Mindstate::Hunting;
                new_target = pp;
            }

            // Act based on state
            match new_state {
                Mindstate::Hunting => {
                    let dist = gruid_paths::manhattan(mon_pos, pp);
                    if dist == 1 {
                        // Adjacent to player: attack
                        self.attack(id, PLAYER_ID);
                    } else {
                        // Chase player using A*
                        let pather = MonsterPather {
                            terrain: &self.map.terrain,
                            actor_positions: &actor_pos,
                            player_pos: pp,
                        };
                        let path = self.pr.astar_path(&pather, mon_pos, pp);
                        if let Some(path) = path {
                            if path.len() > 1 {
                                let next = path[1];
                                // Check if another actor is at next
                                if self.actor_at(next).is_none() {
                                    if let Some(Some(e)) = self.entities.get_mut(id) {
                                        e.pos = next;
                                    }
                                }
                            }
                        }
                    }
                }
                Mindstate::Wandering => {
                    // Wander toward guard position or random
                    let wander_target = if behavior.guard != INVALID_POS {
                        behavior.guard
                    } else if new_target != INVALID_POS {
                        new_target
                    } else {
                        // Pick a random waypoint
                        if !self.map.waypoints.is_empty() {
                            let idx = self.rng.random_range(0..self.map.waypoints.len());
                            self.map.waypoints[idx]
                        } else {
                            mon_pos
                        }
                    };
                    new_target = wander_target;

                    if mon_pos != wander_target {
                        // Try cardinal directions toward target
                        let dx = (wander_target.x - mon_pos.x).signum();
                        let dy = (wander_target.y - mon_pos.y).signum();
                        let candidates = if self.rng.random_range(0..2) == 0 {
                            [Point::new(dx, 0), Point::new(0, dy)]
                        } else {
                            [Point::new(0, dy), Point::new(dx, 0)]
                        };
                        let mut moved = false;
                        for d in candidates {
                            if d.x == 0 && d.y == 0 {
                                continue;
                            }
                            let np = mon_pos.shift(d.x, d.y);
                            if self.map.passable(np) && self.actor_at(np).is_none() {
                                if let Some(Some(e)) = self.entities.get_mut(id) {
                                    e.pos = np;
                                }
                                moved = true;
                                break;
                            }
                        }
                        // If reached target, pick new one
                        if !moved
                            || gruid_paths::manhattan(
                                self.entities[id].as_ref().map(|e| e.pos).unwrap_or(mon_pos),
                                wander_target,
                            ) <= 1
                        {
                            new_target = INVALID_POS;
                        }
                    } else {
                        new_target = INVALID_POS;
                    }
                }
            }

            // Update behavior
            if let Some(Some(e)) = self.entities.get_mut(id) {
                if let Some(a) = e.actor_mut() {
                    if let Some(b) = &mut a.behavior {
                        b.state = new_state;
                        b.target = new_target;
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------
    // FOV
    // -------------------------------------------------------------------

    /// Update the field of view from the player's position.
    pub fn update_fov(&mut self) {
        let pp = self.pp();
        if pp == INVALID_POS {
            return;
        }

        // Set FOV range centered on player, intersected with map bounds
        let fov_rng = gruid_core::Range::new(
            -MAX_FOV_RANGE,
            -MAX_FOV_RANGE,
            MAX_FOV_RANGE + 1,
            MAX_FOV_RANGE + 1,
        );
        let map_rng = gruid_core::Range::new(-pp.x, -pp.y, MAP_WIDTH - pp.x, MAP_HEIGHT - pp.y);
        let effective_rng = fov_rng.intersect(map_rng);
        self.map.fov.set_range(effective_rng);

        // Run vision map
        let lighter = crate::fov_::MapLighter {
            terrain: &self.map.terrain,
        };
        self.map.fov.vision_map(&lighter, pp);

        // Run SSC vision map for boolean visibility
        let pass_fn = |p: Point| -> bool {
            let t = self.map.terrain.at(p).unwrap_or(WALL);
            !matches!(t, WALL | RUBBLE)
        };
        self.map.fov_points = self
            .map
            .fov
            .ssc_vision_map(pp, MAX_FOV_RANGE, pass_fn, false)
            .to_vec();

        // Update known terrain
        for &p in &self.map.fov_points {
            if self.in_fov(p) {
                if let Some(t) = self.map.terrain.at(p) {
                    self.map.known_terrain.set(p, t);
                }
            }
        }

        // Update entity knowledge
        for id in FIRST_MAP_ID + 1..self.entities.len() {
            if let Some(entity) = &mut self.entities[id] {
                if !entity.is_alive() {
                    continue;
                }
                if self.map.fov.visible(entity.pos) {
                    if let Some(cost) = self.map.fov.at(entity.pos) {
                        if cost <= MAX_FOV_RANGE {
                            entity.known_pos = entity.pos;
                            entity.seen = true;
                        }
                    }
                } else if self.map.fov.visible(entity.known_pos) {
                    entity.known_pos = INVALID_POS;
                }
            }
        }
    }

    /// Whether a position is within the current FOV.
    pub fn in_fov(&self, p: Point) -> bool {
        if let Some(cost) = self.map.fov.at(p) {
            cost <= MAX_FOV_RANGE && self.map.fov.visible(p)
        } else {
            false
        }
    }

    // -------------------------------------------------------------------
    // End turn
    // -------------------------------------------------------------------

    /// Process end of turn.
    pub fn end_turn(&mut self) {
        self.turn += 1;
        self.log.new_turn();
        self.process_monsters();
        self.update_fov();
    }
}

/// Find a random floor tile.
fn random_floor(terrain: &gruid_rl::grid::Grid, rng: &mut impl Rng) -> Point {
    for _ in 0..10000 {
        let p = Point::new(
            rng.random_range(0..MAP_WIDTH),
            rng.random_range(0..MAP_HEIGHT),
        );
        if terrain.at(p) == Some(FLOOR) {
            return p;
        }
    }
    Point::new(MAP_WIDTH / 2, MAP_HEIGHT / 2)
}
