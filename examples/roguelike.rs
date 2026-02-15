//! A minimal roguelike demo using gruid.
//!
//! Generates a cave map, displays FOV, and lets you walk with arrow keys / hjkl.
//! Press Escape or 'q' to quit.

use gruid_core::{
    app::{App, AppConfig, Effect, Model},
    grid::Grid,
    messages::{Key, Msg},
    style::{Color, Style},
    Cell, Point, Range,
};
use gruid_crossterm::CrosstermDriver;
use gruid_rl::{
    fov::{FOV, Lighter},
    grid::{Cell as RlCell, Grid as RlGrid},
    mapgen::{CellularAutomataRule, MapGen},
};
use rand::SeedableRng;

const WIDTH: i32 = 80;
const HEIGHT: i32 = 24;
const WALL: RlCell = RlCell(0);
const FLOOR: RlCell = RlCell(1);

// Colours
const COL_BG: Color = Color::from_rgb(20, 20, 30);
const COL_WALL_LIT: Color = Color::from_rgb(100, 100, 130);
const COL_FLOOR_LIT: Color = Color::from_rgb(60, 55, 50);
const COL_WALL_DARK: Color = Color::from_rgb(35, 35, 50);
const COL_FLOOR_DARK: Color = Color::from_rgb(30, 28, 25);
const COL_PLAYER: Color = Color::from_rgb(255, 220, 80);

// ---------------------------------------------------------------------------
// Lighter impl
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Game model
// ---------------------------------------------------------------------------

struct Game {
    map: RlGrid,
    fov: FOV,
    seen: Vec<bool>,
    player: Point,
    log: Vec<String>,
}

impl Game {
    fn new() -> Self {
        let rng = rand::rngs::StdRng::seed_from_u64(42);
        let map = RlGrid::new(WIDTH, HEIGHT);
        map.fill(WALL);

        // Generate cave using cellular automata
        let mut mg = MapGen::with_grid(map.clone(), rng.clone());
        let rules = vec![
            CellularAutomataRule {
                w_cutoff1: 5,
                w_cutoff2: 25, // disabled
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
            log: vec!["Welcome to the gruid-rs roguelike demo!".into(),
                      "Use arrow keys or hjkl to move. Press 'q' or Esc to quit.".into()],
        };
        game.compute_fov();
        game
    }

    fn compute_fov(&mut self) {
        let lighter = MapLighter { map: &self.map };
        self.fov.vision_map(&lighter, self.player, 8);
        // Mark seen cells
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
            } else {
                self.log.push("Blocked by a wall.".into());
            }
        }
    }
}

impl Model for Game {
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
        // Background
        let bg_cell = Cell::default().with_style(Style::default().with_bg(COL_BG));
        grid.fill(bg_cell);

        // Draw map
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let p = Point::new(x, y);
                let terrain = self.map.at(p);
                let idx = (y * WIDTH + x) as usize;
                let seen = idx < self.seen.len() && self.seen[idx];
                let lit = self.fov.at(p).is_some();

                if !seen && !lit {
                    continue; // unexplored
                }

                let (ch, fg, bg) = if terrain == Some(WALL) {
                    if lit {
                        ('#', COL_WALL_LIT, COL_BG)
                    } else {
                        ('#', COL_WALL_DARK, COL_BG)
                    }
                } else {
                    if lit {
                        ('.', COL_FLOOR_LIT, COL_BG)
                    } else {
                        ('.', COL_FLOOR_DARK, COL_BG)
                    }
                };

                let style = Style::default().with_fg(fg).with_bg(bg);
                grid.set(p, Cell::default().with_char(ch).with_style(style));
            }
        }

        // Draw player
        let player_style = Style::default().with_fg(COL_PLAYER).with_bg(COL_BG);
        grid.set(self.player, Cell::default().with_char('@').with_style(player_style));
    }
}

fn main() {
    let game = Game::new();
    let driver = CrosstermDriver::new();
    let mut app = App::new(AppConfig {
        model: game,
        driver,
        width: WIDTH,
        height: HEIGHT,
        frame_writer: None,
    });

    if let Err(e) = app.run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
