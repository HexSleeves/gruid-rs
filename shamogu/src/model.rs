//! Elm-architecture Model implementation.

use gruid_core::{
    Cell, Point, Range,
    app::Effect,
    grid::Grid,
    messages::{Key, Msg},
    style::{AttrMask, Color, Style},
};
use gruid_ui::{Pager, PagerAction, PagerConfig, PagerKeys, PagerStyle, StyledText};

use crate::colors::*;
use crate::entity::*;
use crate::game::Game;
use crate::gamemap::*;
use crate::terrain::*;
use crate::tiles::ATTR_IN_MAP;

pub const UI_WIDTH: i32 = 80;
pub const UI_HEIGHT: i32 = 24;

const HELP_TEXT: &str = "\
Movement:    arrows / hjkl / yubn (vi keys + diagonals)
Wait:        . or space
Examine:     x or mouse move
Help:        ? to show this screen
Quit:        Q or Ctrl+C\n\
\n\
Bump into monsters to attack them.\n\
Explore the cave and survive!";

/// UI mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    GameOver,
    Help,
}

/// The Shamogu game model.
pub struct ShamoguModel {
    game: Game,
    mode: Mode,
    pager: Option<Pager>,
}

impl Default for ShamoguModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ShamoguModel {
    pub fn new() -> Self {
        Self {
            game: Game::new(),
            mode: Mode::Normal,
            pager: None,
        }
    }
}

impl gruid_core::app::Model for ShamoguModel {
    fn update(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::Init => {
                self.game.init();
                None
            }
            _ => match self.mode {
                Mode::Normal => self.update_normal(msg),
                Mode::GameOver => self.update_game_over(msg),
                Mode::Help => self.update_help(msg),
            },
        }
    }

    fn draw(&self, grid: &mut Grid) {
        grid.fill(Cell::default());

        match self.mode {
            Mode::Help => {
                if let Some(pager) = &self.pager {
                    let area = grid.slice(Range::new(0, 0, UI_WIDTH, UI_HEIGHT));
                    pager.draw(&area);
                }
            }
            _ => {
                self.draw_log(grid);
                self.draw_map(grid);
                self.draw_status(grid);
            }
        }
    }
}

impl ShamoguModel {
    // -------------------------------------------------------------------
    // Update
    // -------------------------------------------------------------------

    fn update_normal(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::KeyDown { key, .. } => self.handle_key(key),
            Msg::Mouse { action, pos, .. } => {
                // Mouse click to move/attack
                if action == gruid_core::messages::MouseAction::Main {
                    let map_pos = Point::new(pos.x, pos.y - 2);
                    if map_pos.y >= 0 && map_pos.y < MAP_HEIGHT {
                        let pp = self.game.pp();
                        let dx = (map_pos.x - pp.x).signum();
                        let dy = (map_pos.y - pp.y).signum();
                        if (dx != 0 || dy != 0) && self.game.move_player(dx, dy) {
                            self.game.end_turn();
                            self.check_death();
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn handle_key(&mut self, key: Key) -> Option<Effect> {
        let (dx, dy) = match key {
            // Arrow keys
            Key::ArrowUp => (0, -1),
            Key::ArrowDown => (0, 1),
            Key::ArrowLeft => (-1, 0),
            Key::ArrowRight => (1, 0),
            // Vi keys
            Key::Char('h') => (-1, 0),
            Key::Char('j') => (0, 1),
            Key::Char('k') => (0, -1),
            Key::Char('l') => (1, 0),
            Key::Char('y') => (-1, -1),
            Key::Char('u') => (1, -1),
            Key::Char('b') => (-1, 1),
            Key::Char('n') => (1, 1),
            // Wait
            Key::Char('.') | Key::Char(' ') => {
                self.game.end_turn();
                self.check_death();
                return None;
            }
            // Help
            Key::Char('?') => {
                self.show_help();
                return None;
            }
            // Quit
            Key::Char('Q') => return Some(Effect::End),
            Key::Escape => return Some(Effect::End),
            _ => return None,
        };

        if self.game.move_player(dx, dy) {
            self.game.end_turn();
            self.check_death();
        }
        None
    }

    fn check_death(&mut self) {
        if !self.game.player_alive() {
            self.mode = Mode::GameOver;
        }
    }

    fn show_help(&mut self) {
        let stt = StyledText::text(HELP_TEXT);
        let grid = Grid::new(UI_WIDTH, UI_HEIGHT);
        let pager = Pager::new(PagerConfig {
            grid,
            content: stt,
            keys: PagerKeys::default(),
            box_: None,
            style: PagerStyle::default(),
        });
        self.pager = Some(pager);
        self.mode = Mode::Help;
    }

    fn update_game_over(&mut self, msg: Msg) -> Option<Effect> {
        match msg {
            Msg::KeyDown {
                key: Key::Escape | Key::Char(' ') | Key::Char('Q'),
                ..
            } => Some(Effect::End),
            _ => None,
        }
    }

    fn update_help(&mut self, msg: Msg) -> Option<Effect> {
        if let Some(pager) = &mut self.pager {
            let action = pager.update(msg);
            if action == PagerAction::Quit {
                self.mode = Mode::Normal;
                self.pager = None;
            }
        }
        None
    }

    // -------------------------------------------------------------------
    // Drawing
    // -------------------------------------------------------------------

    fn draw_log(&self, grid: &mut Grid) {
        let log_area = grid.slice(Range::new(0, 0, UI_WIDTH, 2));
        let lines = self.game.log.recent_lines(UI_WIDTH as usize - 1, 2);

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                if x >= UI_WIDTH as usize {
                    break;
                }
                let style = Style::default().with_fg(FG_EMPH);
                log_area.set(
                    Point::new(x as i32, y as i32),
                    Cell::default().with_char(ch).with_style(style),
                );
            }
        }
    }

    fn draw_map(&self, grid: &mut Grid) {
        let map_area = grid.slice(Range::new(0, 2, UI_WIDTH, 2 + MAP_HEIGHT));

        // --- Pass 1: terrain ---
        let sz = self.game.map.terrain.size();
        for y in 0..sz.y {
            for x in 0..sz.x {
                let p = Point::new(x, y);
                let known = self.game.map.known_terrain.at(p).unwrap_or(UNKNOWN);
                if known == UNKNOWN {
                    continue;
                }

                let in_fov = self.game.in_fov(p);

                // Interior walls (no passable neighbour) render as space.
                let ch = if known == WALL && !has_passable_neighbor(&self.game.map, p) {
                    ' '
                } else {
                    terrain_rune(known)
                };

                // Per-terrain foreground when lit; dimmed when out of FOV.
                let fg = if in_fov {
                    terrain_fg(known)
                } else {
                    dim_color(terrain_fg(known))
                };

                let bg = if in_fov { BG_LIT } else { BG };

                let attrs = if in_fov && matches!(known, WALL | TRANSLUCENT_WALL) {
                    AttrMask::BOLD | ATTR_IN_MAP
                } else {
                    ATTR_IN_MAP
                };

                let style = Style { fg, bg, attrs };
                map_area.set(p, Cell::default().with_char(ch).with_style(style));
            }
        }

        // --- Pass 2: entities ---
        for (id, entity) in self.game.alive_actors() {
            let in_fov_at_pos = self.game.in_fov(entity.pos);
            let pos = if id == PLAYER_ID || in_fov_at_pos {
                entity.pos
            } else if entity.known_pos != INVALID_POS {
                entity.known_pos
            } else {
                continue;
            };

            let fg = if id == PLAYER_ID {
                PLAYER_FG
            } else if in_fov_at_pos {
                monster_color(entity.ch)
            } else {
                FG_DIM
            };

            let bg = if self.game.in_fov(pos) { BG_LIT } else { BG };

            let style = Style {
                fg,
                bg,
                attrs: ATTR_IN_MAP,
            };
            map_area.set(pos, Cell::default().with_char(entity.ch).with_style(style));
        }
    }

    fn draw_status(&self, grid: &mut Grid) {
        let status_area = grid.slice(Range::new(0, UI_HEIGHT - 1, UI_WIDTH, UI_HEIGHT));

        let bar_bg = BG_LIT;
        let bar_fg = FG_EMPH;

        // Fill entire status line with background.
        let base = Style {
            fg: bar_fg,
            bg: bar_bg,
            attrs: AttrMask::NONE,
        };
        for x in 0..UI_WIDTH {
            status_area.set(
                Point::new(x, 0),
                Cell::default().with_char(' ').with_style(base),
            );
        }

        // Coloured segments: (text, fg_colour).
        let mut segs: Vec<(String, Color)> = Vec::new();

        segs.push((format!(" L:{} ", self.game.map.level), GREEN));
        segs.push((format!("T:{} ", self.game.turn), bar_fg));

        if let Some(actor) = self.game.player_actor() {
            let hp_color = if actor.hp <= actor.max_hp / 3 {
                HP_CRIT
            } else if actor.hp <= 3 * actor.max_hp / 4 {
                HP_WARN
            } else {
                HP_GOOD
            };
            segs.push(("HP:".into(), bar_fg));
            segs.push((format!("{}/{}", actor.hp, actor.max_hp), hp_color));
            segs.push((" ".into(), bar_fg));

            segs.push(("A:".into(), bar_fg));
            segs.push((format!("{}", actor.attack), STAT_BLUE));
            segs.push((" ".into(), bar_fg));

            segs.push(("D:".into(), bar_fg));
            segs.push((format!("{}", actor.defense), bar_fg));
            segs.push((" ".into(), bar_fg));
        }

        if self.mode == Mode::GameOver {
            segs.push(("*** DEAD *** ".into(), RED));
        }

        // Render coloured segments onto the bar.
        let mut x: i32 = 0;
        for (text, fg) in &segs {
            let style = Style {
                fg: *fg,
                bg: bar_bg,
                attrs: AttrMask::NONE,
            };
            for ch in text.chars() {
                if x >= UI_WIDTH {
                    break;
                }
                status_area.set(
                    Point::new(x, 0),
                    Cell::default().with_char(ch).with_style(style),
                );
                x += 1;
            }
        }
    }
}

/// Map terrain type to its foreground colour when lit.
fn terrain_fg(t: gruid_rl::grid::Cell) -> Color {
    match t {
        WALL => WALL_FG,
        FLOOR => FLOOR_FG,
        FOLIAGE => FOLIAGE_FG,
        RUBBLE => RUBBLE_FG,
        TRANSLUCENT_WALL => TLWALL_FG,
        _ => FG_DIM,
    }
}

/// Dim an RGB colour for out-of-FOV display.
fn dim_color(c: Color) -> Color {
    if c == Color::DEFAULT {
        return FG_DIM;
    }
    // Halve brightness.
    Color::from_rgb(c.r() / 2, c.g() / 2, c.b() / 2)
}

/// Check if a wall cell has any passable neighbor (for interior wall detection).
fn has_passable_neighbor(map: &GameMap, p: Point) -> bool {
    for &d in &[
        Point::new(1, 0),
        Point::new(-1, 0),
        Point::new(0, 1),
        Point::new(0, -1),
        Point::new(1, 1),
        Point::new(1, -1),
        Point::new(-1, 1),
        Point::new(-1, -1),
    ] {
        let np = p.shift(d.x, d.y);
        let known = map.known_terrain.at(np).unwrap_or(UNKNOWN);
        if passable(known) {
            return true;
        }
    }
    false
}
