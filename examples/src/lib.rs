//! Shared roguelike game model used by both terminal and graphical examples.

use gruid_core::{
    app::Effect,
    grid::Grid,
    messages::{Key, Msg},
    style::{Color, Style},
    Cell, Point, Range,
};
use gruid_rl::{
    fov::{FOV, Lighter},
    grid::{Cell as RlCell, Grid as RlGrid},
    mapgen::{CellularAutomataRule, MapGen},
};
use rand::SeedableRng;

pub const WIDTH: i32 = 80;
pub const HEIGHT: i32 = 24;
pub const WALL: RlCell = RlCell(0);
pub const FLOOR: RlCell = RlCell(1);

// Colours
const COL_BG: Color = Color::from_rgb(20, 20, 30);
const COL_WALL_LIT: Color = Color::from_rgb(100, 100, 130);
const COL_FLOOR_LIT: Color = Color::from_rgb(60, 55, 50);
const COL_WALL_DARK: Color = Color::from_rgb(35, 35, 50);
const COL_FLOOR_DARK: Color = Color::from_rgb(30, 28, 25);
const COL_PLAYER: Color = Color::from_rgb(255, 220, 80);

struct MapLighter<'a> {
    map: &'a RlGrid,
}

impl<'a> Lighter for MapLighter<'a> {
    fn cost(&self, _from: Point, to: Point) -> i32 {
        if self.map.at(to) == Some(WALL) {
            i32::MAX
        } else {
            1
        }
    }
}

pub struct Game {
    map: RlGrid,
    fov: FOV,
    seen: Vec<bool>,
    player: Point,
}

impl Game {
    pub fn new() -> Self {
        let rng = rand::rngs::StdRng::seed_from_u64(42);
        let map = RlGrid::new(WIDTH, HEIGHT);
        map.fill(WALL);

        let mut mg = MapGen::with_grid(map.clone(), rng.clone());
        let rules = vec![
            CellularAutomataRule {
                w_cutoff1: 5,
                w_cutoff2: 25, // >= 25 disables W(2) check
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

        // Find a floor tile for the player
        let mut player = Point::new(WIDTH / 2, HEIGHT / 2);
        'outer: for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let p = Point::new(x, y);
                if map.at(p) == Some(FLOOR) {
                    player = p;
                    break 'outer;
                }
            }
        }

        let rg = Range::new(0, 0, WIDTH, HEIGHT);
        let fov = FOV::new(rg);
        let seen = vec![false; (WIDTH * HEIGHT) as usize];

        let mut game = Game {
            map,
            fov,
            seen,
            player,
        };
        game.compute_fov();
        game
    }

    fn compute_fov(&mut self) {
        let lighter = MapLighter { map: &self.map };
        self.fov.vision_map(&lighter, self.player, 8);
        for ln in self.fov.iter_lighted() {
            let idx = (ln.pos.y * WIDTH + ln.pos.x) as usize;
            if idx < self.seen.len() {
                self.seen[idx] = true;
            }
        }
    }

    fn try_move(&mut self, dx: i32, dy: i32) {
        let np = self.player.shift(dx, dy);
        if np.x >= 0 && np.x < WIDTH && np.y >= 0 && np.y < HEIGHT {
            if self.map.at(np) == Some(FLOOR) {
                self.player = np;
                self.compute_fov();
            }
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl gruid_core::app::Model for Game {
    fn update(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::Init => None,
            Msg::KeyDown { ref key, .. } => {
                match key {
                    Key::Escape | Key::Char('q') | Key::Char('Q') => {
                        return Some(Effect::End);
                    }
                    Key::ArrowUp | Key::Char('k') => self.try_move(0, -1),
                    Key::ArrowDown | Key::Char('j') => self.try_move(0, 1),
                    Key::ArrowLeft | Key::Char('h') => self.try_move(-1, 0),
                    Key::ArrowRight | Key::Char('l') => self.try_move(1, 0),
                    Key::Char('y') => self.try_move(-1, -1),
                    Key::Char('u') => self.try_move(1, -1),
                    Key::Char('b') => self.try_move(-1, 1),
                    Key::Char('n') => self.try_move(1, 1),
                    _ => {}
                }
                None
            }
            Msg::Quit => Some(Effect::End),
            _ => None,
        }
    }

    fn draw(&self, grid: &mut Grid) {
        let bg_cell = Cell::default().with_style(Style::default().with_bg(COL_BG));
        grid.fill(bg_cell);

        for y in 0..HEIGHT {
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

        // Draw player
        let player_style = Style::default().with_fg(COL_PLAYER).with_bg(COL_BG);
        grid.set(
            self.player,
            Cell::default().with_char('@').with_style(player_style),
        );
    }
}
