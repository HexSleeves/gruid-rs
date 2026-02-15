//! Shared roguelike game model used by both terminal and graphical examples.
//!
//! Demonstrates: cave generation, FOV, A* pathfinding, Dijkstra maps,
//! UI widgets (status bar, message log, help pager), mouse support,
//! and simple monster AI.

use gruid_core::{
    Cell, Point, Range,
    app::Effect,
    grid::Grid,
    messages::{Key, MouseAction, Msg},
    style::{AttrMask, Color, Style},
};
use gruid_paths::{AstarPather, PathRange, Pather, WeightedPather};
use gruid_rl::{
    fov::{CircularLighter, FOV, FovShape, Lighter},
    grid::{Cell as RlCell, Grid as RlGrid},
    mapgen::{CellularAutomataRule, MapGen},
};
use gruid_ui::{BoxDecor, Pager, PagerAction, PagerConfig, PagerKeys, PagerStyle, StyledText};
use rand::{Rng, SeedableRng};

pub const WIDTH: i32 = 80;
pub const HEIGHT: i32 = 24;
pub const MAP_HEIGHT: i32 = 21; // leave 3 rows for UI
const WALL: RlCell = RlCell(0);
const FLOOR: RlCell = RlCell(1);

// Colours
const COL_BG: Color = Color::from_rgb(20, 20, 30);
const COL_WALL_LIT: Color = Color::from_rgb(100, 100, 130);
const COL_FLOOR_LIT: Color = Color::from_rgb(60, 55, 50);
const COL_WALL_DARK: Color = Color::from_rgb(35, 35, 50);
const COL_FLOOR_DARK: Color = Color::from_rgb(30, 28, 25);
const COL_PLAYER: Color = Color::from_rgb(255, 220, 80);
const COL_MONSTER: Color = Color::from_rgb(220, 50, 50);
const COL_PATH: Color = Color::from_rgb(50, 180, 255);
const COL_DIJKSTRA_NEAR: Color = Color::from_rgb(40, 120, 60);
const COL_DIJKSTRA_FAR: Color = Color::from_rgb(120, 40, 40);
const COL_STATUS_FG: Color = Color::from_rgb(200, 200, 200);
const COL_STATUS_BG: Color = Color::from_rgb(30, 30, 50);
const COL_LOG_FG: Color = Color::from_rgb(170, 170, 190);
const COL_CURSOR: Color = Color::from_rgb(80, 200, 80);

const HELP_TEXT: &str = "\
Movement:    arrows / hjkl / yubn (diagonals)
Wait:        . or space
Mouse:       click to auto-move toward target
Examine:     x to enter look mode, move cursor, ESC to exit
Pathfinding: p to toggle path overlay
Algorithm:   TAB to switch A* / JPS
Dijkstra:    d to toggle distance heatmap
FOV shape:   f to toggle square / circle
Help:        ? to show this screen
Quit:        q or ESC";

// ---------------------------------------------------------------------------
// Monster
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Monster {
    pos: Point,
    ch: char,
    hp: i32,
    awake: bool,
}

// ---------------------------------------------------------------------------
// Map pather for A* / Dijkstra
// ---------------------------------------------------------------------------

struct MapPather<'a> {
    map: &'a RlGrid,
}

impl Pather for MapPather<'_> {
    fn neighbors(&self, p: Point, buf: &mut Vec<Point>) {
        for &d in &[
            Point::new(1, 0),
            Point::new(-1, 0),
            Point::new(0, 1),
            Point::new(0, -1),
        ] {
            let np = p.shift(d.x, d.y);
            if self.map.at(np) == Some(FLOOR) {
                buf.push(np);
            }
        }
    }
}

impl WeightedPather for MapPather<'_> {
    fn cost(&self, _from: Point, _to: Point) -> i32 {
        1
    }
}

impl AstarPather for MapPather<'_> {
    fn estimate(&self, from: Point, to: Point) -> i32 {
        gruid_paths::manhattan(from, to)
    }
}

// ---------------------------------------------------------------------------
// FOV lighter
// ---------------------------------------------------------------------------

struct MapLighter<'a> {
    map: &'a RlGrid,
}

impl Lighter for MapLighter<'_> {
    fn cost(&self, _src: Point, from: Point, _to: Point) -> i32 {
        if self.map.at(from) == Some(WALL) {
            i32::MAX
        } else {
            1
        }
    }
    fn max_cost(&self, _src: Point) -> i32 {
        8
    }
}

// ---------------------------------------------------------------------------
// UI modes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Play,
    Look,
    Help,
}

#[derive(Clone, Copy, PartialEq, Eq)]
/// Auto-move tick message.
struct AutoMoveTick;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PathAlgo {
    Astar,
    Jps,
}

impl PathAlgo {
    fn label(self) -> &'static str {
        match self {
            Self::Astar => "A*",
            Self::Jps => "JPS",
        }
    }

    fn toggle(self) -> Self {
        match self {
            Self::Astar => Self::Jps,
            Self::Jps => Self::Astar,
        }
    }
}

// ---------------------------------------------------------------------------
// Game
// ---------------------------------------------------------------------------

pub struct Game {
    map: RlGrid,
    fov: FOV,
    path_range: PathRange,
    seen: Vec<bool>,
    player: Point,
    hp: i32,
    max_hp: i32,
    turns: u32,
    monsters: Vec<Monster>,
    messages: Vec<String>,
    // Overlays
    show_path: bool,
    show_dijkstra: bool,
    path_algo: PathAlgo,
    fov_shape: FovShape,
    path_cache: Vec<Point>,
    // Cursor / mouse
    cursor: Point,
    mode: Mode,
    // Help pager
    pager: Option<Pager>,
    // Auto-move
    auto_path: Vec<Point>,
    auto_step: usize,
}

impl Game {
    pub fn new() -> Self {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let map = RlGrid::new(WIDTH, MAP_HEIGHT);
        map.fill(WALL);

        let mut mg = MapGen::with_grid(map.clone(), rng.clone());
        let rules = vec![
            CellularAutomataRule {
                w_cutoff1: 5,
                w_cutoff2: 25,
                walls_out_of_range: true,
                reps: 4,
            },
            CellularAutomataRule {
                w_cutoff1: 5,
                w_cutoff2: 25,
                walls_out_of_range: true,
                reps: 3,
            },
        ];
        mg.cellular_automata_cave(WALL, FLOOR, 0.45, &rules);
        let map = mg.grid.clone();

        // Find a floor tile for the player.
        let mut player = Point::new(WIDTH / 2, MAP_HEIGHT / 2);
        'find_player: for y in 0..MAP_HEIGHT {
            for x in 0..WIDTH {
                let p = Point::new(x, y);
                if map.at(p) == Some(FLOOR) {
                    player = p;
                    break 'find_player;
                }
            }
        }

        // Spawn some monsters on random floor tiles.
        let monster_chars = ['g', 'k', 'r', 's', 'z'];
        let mut monsters = Vec::new();
        let mut attempts = 0;
        while monsters.len() < 8 && attempts < 500 {
            let p = Point::new(rng.random_range(0..WIDTH), rng.random_range(0..MAP_HEIGHT));
            attempts += 1;
            if map.at(p) != Some(FLOOR)
                || p == player
                || gruid_paths::manhattan(p, player) < 5
                || monsters.iter().any(|m: &Monster| m.pos == p)
            {
                continue;
            }
            monsters.push(Monster {
                pos: p,
                ch: monster_chars[monsters.len() % monster_chars.len()],
                hp: 3,
                awake: false,
            });
        }

        let rg = Range::new(0, 0, WIDTH, MAP_HEIGHT);
        let fov = FOV::new(rg);
        let path_range = PathRange::new(rg);
        let seen = vec![false; (WIDTH * MAP_HEIGHT) as usize];

        let mut game = Game {
            map,
            fov,
            path_range,
            seen,
            player,
            hp: 20,
            max_hp: 20,
            turns: 0,
            monsters,
            messages: vec!["Welcome! Press ? for help.".into()],
            show_path: false,
            show_dijkstra: false,
            path_algo: PathAlgo::Astar,
            fov_shape: FovShape::Square,
            path_cache: Vec::new(),
            cursor: player,
            mode: Mode::Play,
            pager: None,
            auto_path: Vec::new(),
            auto_step: 0,
        };
        game.compute_fov();
        game
    }

    fn compute_fov(&mut self) {
        let base = MapLighter { map: &self.map };
        match self.fov_shape {
            FovShape::Square => {
                self.fov.vision_map(&base, self.player);
            }
            FovShape::Circle => {
                let circular = CircularLighter::new(base);
                self.fov.vision_map(&circular, self.player);
            }
        }
        for ln in self.fov.iter_lighted() {
            let idx = (ln.pos.y * WIDTH + ln.pos.x) as usize;
            if idx < self.seen.len() {
                self.seen[idx] = true;
            }
        }
        // Wake up monsters in FOV.
        for m in &mut self.monsters {
            if self.fov.at(m.pos).is_some() {
                m.awake = true;
            }
        }
    }

    fn find_path(&mut self, from: Point, to: Point) -> Option<Vec<Point>> {
        match self.path_algo {
            PathAlgo::Astar => {
                let pather = MapPather { map: &self.map };
                self.path_range.astar_path(&pather, from, to)
            }
            PathAlgo::Jps => {
                let map = &self.map;
                self.path_range.jps_path(
                    from,
                    to,
                    |p| map.at(p) == Some(FLOOR),
                    false, // 4-way cardinal only
                )
            }
        }
    }

    fn recompute_path(&mut self) {
        self.path_cache = self.find_path(self.player, self.cursor).unwrap_or_default();
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let np = self.player.shift(dx, dy);
        if np.x < 0 || np.x >= WIDTH || np.y < 0 || np.y >= MAP_HEIGHT {
            return false;
        }
        if self.map.at(np) != Some(FLOOR) {
            return false;
        }

        // Check for monster at target.
        if let Some(mi) = self.monsters.iter().position(|m| m.pos == np && m.hp > 0) {
            self.monsters[mi].hp -= 1;
            let ch = self.monsters[mi].ch;
            if self.monsters[mi].hp <= 0 {
                self.log(format!("You kill the {ch}!"));
                self.monsters.remove(mi);
            } else {
                let hp = self.monsters[mi].hp;
                self.log(format!("You hit the {ch} ({hp} hp left)."));
            }
            self.turns += 1;
            self.tick_monsters();
            self.compute_fov();
            return true;
        }

        self.player = np;
        self.turns += 1;
        self.tick_monsters();
        self.compute_fov();
        if self.show_path || self.show_dijkstra {
            self.recompute_path();
        }
        true
    }

    fn tick_monsters(&mut self) {
        let player = self.player;
        // Collect attack messages first, then apply movement.
        let mut attacks: Vec<(usize, char)> = Vec::new();
        let mut moves: Vec<(usize, Point)> = Vec::new();

        for i in 0..self.monsters.len() {
            if !self.monsters[i].awake || self.monsters[i].hp <= 0 {
                continue;
            }
            let mpos = self.monsters[i].pos;
            if gruid_paths::manhattan(mpos, player) <= 1 {
                attacks.push((i, self.monsters[i].ch));
                continue;
            }
            // Move toward player using A*.
            let pather = MapPather { map: &self.map };

            if let Some(path) = if self.path_algo == PathAlgo::Jps {
                self.path_range
                    .jps_path(mpos, player, |p| self.map.at(p) == Some(FLOOR), false)
            } else {
                self.path_range.astar_path(&pather, mpos, player)
            } {
                if path.len() >= 2 {
                    let next = path[1];
                    let blocked = self
                        .monsters
                        .iter()
                        .enumerate()
                        .any(|(j, m)| j != i && m.pos == next && m.hp > 0);
                    if !blocked {
                        moves.push((i, next));
                    }
                }
            }
        }

        for (_, ch) in &attacks {
            self.hp -= 1;
            self.log(format!("The {ch} hits you!"));
        }
        for &(i, next) in &moves {
            self.monsters[i].pos = next;
        }
    }

    fn log(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 50 {
            self.messages.remove(0);
        }
    }

    fn open_help(&mut self) {
        let grid = Grid::new(WIDTH, HEIGHT);
        let box_ = BoxDecor {
            title: StyledText::new(" Help ", Style::default().with_fg(COL_PLAYER)),
            ..BoxDecor::new()
        };
        self.pager = Some(Pager::new(PagerConfig {
            content: StyledText::new(HELP_TEXT, Style::default().with_fg(COL_STATUS_FG)),
            grid,
            keys: PagerKeys {
                quit: vec![Key::Escape, Key::Char('q'), Key::Char('?')],
                ..PagerKeys::default()
            },
            box_: Some(box_),
            style: PagerStyle::default(),
        }));
        self.mode = Mode::Help;
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl gruid_core::app::Model for Game {
    fn update(&mut self, msg: Msg) -> Option<Effect> {
        // ---- Help mode ----
        if self.mode == Mode::Help {
            if let Some(ref mut pager) = self.pager {
                let action = pager.update(msg);
                if action == PagerAction::Quit {
                    self.pager = None;
                    self.mode = Mode::Play;
                }
            }
            return None;
        }

        match msg {
            Msg::Init => None,
            Msg::Quit => Some(Effect::End),

            // ---- Keyboard ----
            Msg::KeyDown { ref key, .. } => {
                // Auto-move step: any key cancels it.
                if !self.auto_path.is_empty() {
                    self.auto_path.clear();
                    self.auto_step = 0;
                }

                match self.mode {
                    Mode::Look => match key {
                        Key::Escape | Key::Char('x') => {
                            self.mode = Mode::Play;
                            self.cursor = self.player;
                        }
                        Key::ArrowUp | Key::Char('k') => self.cursor = self.cursor.shift(0, -1),
                        Key::ArrowDown | Key::Char('j') => self.cursor = self.cursor.shift(0, 1),
                        Key::ArrowLeft | Key::Char('h') => self.cursor = self.cursor.shift(-1, 0),
                        Key::ArrowRight | Key::Char('l') => self.cursor = self.cursor.shift(1, 0),
                        _ => {}
                    },
                    Mode::Play => match key {
                        Key::Escape | Key::Char('q') | Key::Char('Q') => {
                            return Some(Effect::End);
                        }
                        // Movement
                        Key::ArrowUp | Key::Char('k') => {
                            self.try_move(0, -1);
                        }
                        Key::ArrowDown | Key::Char('j') => {
                            self.try_move(0, 1);
                        }
                        Key::ArrowLeft | Key::Char('h') => {
                            self.try_move(-1, 0);
                        }
                        Key::ArrowRight | Key::Char('l') => {
                            self.try_move(1, 0);
                        }
                        Key::Char('y') => {
                            self.try_move(-1, -1);
                        }
                        Key::Char('u') => {
                            self.try_move(1, -1);
                        }
                        Key::Char('b') => {
                            self.try_move(-1, 1);
                        }
                        Key::Char('n') => {
                            self.try_move(1, 1);
                        }
                        // Wait
                        Key::Char('.') | Key::Space => {
                            self.turns += 1;
                            self.tick_monsters();
                            self.compute_fov();
                        }
                        // Toggles
                        Key::Char('p') => {
                            self.show_path = !self.show_path;
                            if self.show_path {
                                self.recompute_path();
                                self.log("Path overlay ON.".into());
                            } else {
                                self.path_cache.clear();
                                self.log("Path overlay OFF.".into());
                            }
                        }
                        Key::Char('d') => {
                            self.show_dijkstra = !self.show_dijkstra;
                            if self.show_dijkstra {
                                let pather = MapPather { map: &self.map };
                                self.path_range.dijkstra_map(
                                    &pather,
                                    &[self.player],
                                    gruid_paths::UNREACHABLE,
                                );
                                self.log("Dijkstra heatmap ON.".into());
                            } else {
                                self.log("Dijkstra heatmap OFF.".into());
                            }
                        }
                        Key::Tab => {
                            self.path_algo = self.path_algo.toggle();
                            let label = self.path_algo.label();
                            self.log(format!("Pathfinding: {label}"));
                            if self.show_path {
                                self.recompute_path();
                            }
                        }
                        Key::Char('f') => {
                            self.fov_shape = match self.fov_shape {
                                FovShape::Square => FovShape::Circle,
                                FovShape::Circle => FovShape::Square,
                            };
                            self.compute_fov();
                            let label = match self.fov_shape {
                                FovShape::Square => "square",
                                FovShape::Circle => "circle",
                            };
                            self.log(format!("FOV shape: {label}"));
                        }
                        Key::Char('x') => {
                            self.mode = Mode::Look;
                            self.cursor = self.player;
                            self.log("Look mode. Move cursor, ESC to exit.".into());
                        }
                        Key::Char('?') => {
                            self.open_help();
                        }
                        _ => {}
                    },
                    _ => {}
                }

                // Update path in look mode.
                if self.mode == Mode::Look && self.show_path {
                    self.recompute_path();
                }

                None
            }

            // ---- Mouse ----
            Msg::Mouse { action, pos, .. } => {
                // Only handle within the map area.
                if pos.y >= 0 && pos.y < MAP_HEIGHT && pos.x >= 0 && pos.x < WIDTH {
                    self.cursor = pos;

                    if self.show_path {
                        self.recompute_path();
                    }

                    if action == MouseAction::Main && self.mode == Mode::Play {
                        // Click to auto-move.
                        if let Some(path) = self.find_path(self.player, pos) {
                            if path.len() > 1 {
                                self.auto_path = path;
                                self.auto_step = 1;
                                // Trigger first step.
                                return Some(Effect::Cmd(Box::new(|| {
                                    Some(Msg::custom(AutoMoveTick))
                                })));
                            }
                        }
                    }
                }
                None
            }

            // ---- Timer tick for auto-move ----
            _ if msg.downcast_ref::<AutoMoveTick>().is_some() => {
                if self.auto_step < self.auto_path.len() {
                    let next = self.auto_path[self.auto_step];
                    let dx = next.x - self.player.x;
                    let dy = next.y - self.player.y;
                    if self.try_move(dx, dy) {
                        self.auto_step += 1;
                        if self.auto_step < self.auto_path.len() && self.hp > 0 {
                            // Schedule next step.
                            return Some(Effect::Cmd(Box::new(|| {
                                std::thread::sleep(std::time::Duration::from_millis(60));
                                Some(Msg::custom(AutoMoveTick))
                            })));
                        }
                    }
                    self.auto_path.clear();
                    self.auto_step = 0;
                }

                if self.show_dijkstra {
                    println!("Computing Dijkstra heatmap");
                    let pather = MapPather { map: &self.map };
                    self.path_range
                        .dijkstra_map(&pather, &[self.player], gruid_paths::UNREACHABLE);
                }

                None
            }

            _ => None,
        }
    }

    fn draw(&self, grid: &mut Grid) {
        // ---- Help overlay ----
        if self.mode == Mode::Help {
            if let Some(ref pager) = self.pager {
                pager.draw(grid);
            }
            return;
        }

        let bg_cell = Cell::default().with_style(Style::default().with_bg(COL_BG));
        grid.fill(bg_cell);

        // ---- Map ----
        for y in 0..MAP_HEIGHT {
            for x in 0..WIDTH {
                let p = Point::new(x, y);
                let terrain = self.map.at(p);
                let idx = (y * WIDTH + x) as usize;
                let seen = idx < self.seen.len() && self.seen[idx];
                let lit = self.fov.at(p).is_some();

                if !seen && !lit {
                    continue;
                }

                let (ch, fg, bg) = if terrain == Some(WALL) {
                    if lit {
                        ('#', COL_WALL_LIT, COL_BG)
                    } else {
                        ('#', COL_WALL_DARK, COL_BG)
                    }
                } else if lit {
                    ('.', COL_FLOOR_LIT, COL_BG)
                } else {
                    ('.', COL_FLOOR_DARK, COL_BG)
                };

                let style = Style::default().with_fg(fg).with_bg(bg);
                grid.set(p, Cell::default().with_char(ch).with_style(style));
            }
        }

        // ---- Dijkstra heatmap overlay ----
        if self.show_dijkstra {
            for y in 0..MAP_HEIGHT {
                for x in 0..WIDTH {
                    let p = Point::new(x, y);
                    let d = self.path_range.dijkstra_at(p);
                    if d == gruid_paths::UNREACHABLE || d < 0 {
                        continue;
                    }
                    let idx = (y * WIDTH + x) as usize;
                    if idx >= self.seen.len() || !self.seen[idx] {
                        continue;
                    }
                    // Interpolate near (green) to far (red).
                    let t = (d as f32 / 30.0).min(1.0);
                    let r = lerp_u8(COL_DIJKSTRA_NEAR.r(), COL_DIJKSTRA_FAR.r(), t);
                    let g = lerp_u8(COL_DIJKSTRA_NEAR.g(), COL_DIJKSTRA_FAR.g(), t);
                    let b = lerp_u8(COL_DIJKSTRA_NEAR.b(), COL_DIJKSTRA_FAR.b(), t);
                    let bg = Color::from_rgb(r, g, b);
                    let existing = grid.at(p);
                    grid.set(
                        p,
                        Cell::default()
                            .with_char(existing.ch)
                            .with_style(existing.style.with_bg(bg)),
                    );
                }
            }
        }

        // ---- A* path overlay ----
        if self.show_path && self.path_cache.len() > 1 {
            for &p in &self.path_cache[1..] {
                if p == self.player {
                    continue;
                }
                let existing = grid.at(p);
                let style = existing.style.with_fg(COL_PATH).with_attrs(AttrMask::BOLD);
                grid.set(p, Cell::default().with_char('*').with_style(style));
            }
        }

        // ---- Monsters ----
        for m in &self.monsters {
            if m.hp <= 0 {
                continue;
            }
            if self.fov.at(m.pos).is_some() {
                let style = Style::default().with_fg(COL_MONSTER).with_bg(COL_BG);
                grid.set(m.pos, Cell::default().with_char(m.ch).with_style(style));
            }
        }

        // ---- Player ----
        let player_style = Style::default()
            .with_fg(COL_PLAYER)
            .with_bg(COL_BG)
            .with_attrs(AttrMask::BOLD);
        grid.set(
            self.player,
            Cell::default().with_char('@').with_style(player_style),
        );

        // ---- Look cursor ----
        if self.mode == Mode::Look {
            let existing = grid.at(self.cursor);
            let style = existing.style.with_bg(COL_CURSOR);
            grid.set(
                self.cursor,
                Cell::default().with_char(existing.ch).with_style(style),
            );
        }

        // ---- Status bar (row MAP_HEIGHT) ----
        let status_y = MAP_HEIGHT;
        let status_style = Style::default()
            .with_fg(COL_STATUS_FG)
            .with_bg(COL_STATUS_BG);
        for x in 0..WIDTH {
            grid.set(
                Point::new(x, status_y),
                Cell::default().with_char(' ').with_style(status_style),
            );
        }

        let hp_text = format!(" HP: {}/{}", self.hp, self.max_hp);
        let pos_text = format!("Pos: ({},{})", self.player.x, self.player.y);
        let turn_text = format!("Turn: {}", self.turns);
        let mode_text = match self.mode {
            Mode::Look => "[LOOK]",
            _ => "",
        };
        let fov_tag = match self.fov_shape {
            FovShape::Circle => "[FOV:○]",
            FovShape::Square => "",
        };
        let overlays = format!(
            "{}{}{}",
            if self.show_path {
                match self.path_algo {
                    PathAlgo::Astar => "[A*]",
                    PathAlgo::Jps => "[JPS]",
                }
            } else {
                ""
            },
            if self.show_dijkstra { "[DJKS]" } else { "" },
            fov_tag,
        );

        let status = format!("{hp_text}  {pos_text}  {turn_text}  {mode_text}{overlays}");
        let hp_style = if self.hp <= 5 {
            status_style.with_fg(COL_MONSTER)
        } else {
            status_style
        };
        for (i, ch) in status.chars().enumerate() {
            let x = i as i32;
            if x >= WIDTH {
                break;
            }
            let s = if i < hp_text.len() {
                hp_style
            } else {
                status_style
            };
            grid.set(
                Point::new(x, status_y),
                Cell::default().with_char(ch).with_style(s),
            );
        }

        // Monster count on the right side.
        let alive = self.monsters.iter().filter(|m| m.hp > 0).count();
        let right_text = format!("Monsters: {} ", alive);
        let start_x = (WIDTH - right_text.len() as i32).max(0);
        for (i, ch) in right_text.chars().enumerate() {
            grid.set(
                Point::new(start_x + i as i32, status_y),
                Cell::default().with_char(ch).with_style(status_style),
            );
        }

        // ---- Message log (rows MAP_HEIGHT+1 .. HEIGHT-1) ----
        let log_rows = (HEIGHT - MAP_HEIGHT - 1) as usize;
        let log_style = Style::default().with_fg(COL_LOG_FG).with_bg(COL_BG);
        let start = self.messages.len().saturating_sub(log_rows);
        for (row, msg) in self.messages[start..].iter().enumerate() {
            let y = MAP_HEIGHT + 1 + row as i32;
            for (i, ch) in msg.chars().enumerate() {
                let x = i as i32;
                if x >= WIDTH {
                    break;
                }
                grid.set(
                    Point::new(x, y),
                    Cell::default().with_char(ch).with_style(log_style),
                );
            }
        }

        // ---- Look mode info ----
        if self.mode == Mode::Look {
            let info_y = HEIGHT - 1;
            let mut info = format!("({},{}) ", self.cursor.x, self.cursor.y);
            if let Some(terrain) = self.map.at(self.cursor) {
                if terrain == WALL {
                    info.push_str("Wall");
                } else {
                    info.push_str("Floor");
                }
            }
            if let Some(m) = self
                .monsters
                .iter()
                .find(|m| m.pos == self.cursor && m.hp > 0)
            {
                info.push_str(&format!(" | Monster '{}' HP:{}", m.ch, m.hp));
            }
            if self.player == self.cursor {
                info.push_str(" | You");
            }
            let info_style = Style::default().with_fg(COL_PLAYER).with_bg(COL_BG);
            for (i, ch) in info.chars().enumerate() {
                if i as i32 >= WIDTH {
                    break;
                }
                grid.set(
                    Point::new(i as i32, info_y),
                    Cell::default().with_char(ch).with_style(info_style),
                );
            }
        }
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}
