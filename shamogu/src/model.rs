//! Elm-architecture Model implementation.

use gruid_core::{
    Cell, Point, Range,
    app::Effect,
    grid::Grid,
    messages::{Key, Msg},
    style::{AttrMask, Style},
};
use gruid_ui::{Pager, PagerAction, PagerConfig, PagerKeys, PagerStyle, StyledText};

use crate::colors::*;
use crate::entity::*;
use crate::game::Game;
use crate::gamemap::*;
use crate::terrain::*;

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
        let log_fg = FG_SECONDARY;

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                if x >= UI_WIDTH as usize {
                    break;
                }
                let style = Style::default().with_fg(log_fg);
                log_area.set(
                    Point::new(x as i32, y as i32),
                    Cell::default().with_char(ch).with_style(style),
                );
            }
        }

        // Color log entries based on style
        // (simplified: just use grey for now, colored log entries in future)
    }

    fn draw_map(&self, grid: &mut Grid) {
        let map_area = grid.slice(Range::new(0, 2, UI_WIDTH, 2 + MAP_HEIGHT));

        // Draw terrain
        let sz = self.game.map.terrain.size();
        for y in 0..sz.y {
            for x in 0..sz.x {
                let p = Point::new(x, y);
                let known = self.game.map.known_terrain.at(p).unwrap_or(UNKNOWN);
                if known == UNKNOWN {
                    continue; // Leave blank
                }

                let in_fov = self.game.in_fov(p);
                let ch = terrain_rune(known);

                // Interior walls render as space
                let ch = if known == WALL && !has_passable_neighbor(&self.game.map, p) {
                    ' '
                } else {
                    ch
                };

                let (fg, bg) = if in_fov {
                    (FG, BG_SECONDARY)
                } else {
                    (FG_SECONDARY, BG)
                };

                let mut attrs = AttrMask::NONE;
                if matches!(known, WALL | TRANSLUCENT_WALL) {
                    attrs = AttrMask::BOLD;
                }

                let style = Style { fg, bg, attrs };
                map_area.set(p, Cell::default().with_char(ch).with_style(style));
            }
        }

        // Draw entities (sorted by render order)
        let mut draw_list: Vec<(Point, char, Style)> = Vec::new();

        for (id, entity) in self.game.alive_actors() {
            let pos = if id == PLAYER_ID || self.game.in_fov(entity.pos) {
                entity.pos
            } else if entity.known_pos != INVALID_POS {
                entity.known_pos
            } else {
                continue;
            };

            let fg = if id == PLAYER_ID {
                FG_EMPH
            } else if self.game.in_fov(entity.pos) {
                monster_color(entity.ch)
            } else {
                FG_SECONDARY
            };

            let bg = if self.game.in_fov(pos) {
                BG_SECONDARY
            } else {
                BG
            };

            let style = Style {
                fg,
                bg,
                attrs: AttrMask::NONE,
            };
            draw_list.push((pos, entity.ch, style));
        }

        // Draw entities with higher render order on top
        for (pos, ch, style) in &draw_list {
            map_area.set(*pos, Cell::default().with_char(*ch).with_style(*style));
        }
    }

    fn draw_status(&self, grid: &mut Grid) {
        let status_area = grid.slice(Range::new(0, UI_HEIGHT - 1, UI_WIDTH, UI_HEIGHT));

        // Build status text
        let mut status = String::new();

        // Level
        status.push_str(&format!(" L:{} ", self.game.map.level));

        // Turn
        status.push_str(&format!("T:{} ", self.game.turn));

        // HP
        if let Some(actor) = self.game.player_actor() {
            status.push_str(&format!("HP:{}/{} ", actor.hp, actor.max_hp));
            status.push_str(&format!("A:{} ", actor.attack));
            status.push_str(&format!("D:{} ", actor.defense));
        }

        // Game over indicator
        if self.mode == Mode::GameOver {
            status.push_str("*** DEAD *** Press Q/ESC to quit");
        }

        // Draw status bar
        let style = Style {
            fg: FG_EMPH,
            bg: BG_SECONDARY,
            attrs: AttrMask::NONE,
        };

        // Fill entire status line with background
        for x in 0..UI_WIDTH {
            status_area.set(
                Point::new(x, 0),
                Cell::default().with_char(' ').with_style(style),
            );
        }

        // Write status text
        for (x, ch) in status.chars().enumerate() {
            if x >= UI_WIDTH as usize {
                break;
            }
            status_area.set(
                Point::new(x as i32, 0),
                Cell::default().with_char(ch).with_style(style),
            );
        }
    }
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
