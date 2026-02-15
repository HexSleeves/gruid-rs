//! Replay widget — plays back recorded [`Frame`]s.
//!
//! [`Replay`] implements [`Model`] and can serve as the main application
//! model for session playback with speed control, pause, seeking, and undo.

use std::io::Read;
use std::time::Duration;

use gruid_core::app::Effect;
use gruid_core::grid::{Frame, FrameCell, Grid};
use gruid_core::messages::{Key, Msg};
use gruid_core::recording::FrameDecoder;

// ---------------------------------------------------------------------------
// Key bindings
// ---------------------------------------------------------------------------

/// Key bindings for the replay widget.
#[derive(Debug, Clone)]
pub struct ReplayKeys {
    pub quit: Vec<Key>,
    pub pause: Vec<Key>,
    pub speed_more: Vec<Key>,
    pub speed_less: Vec<Key>,
    pub frame_next: Vec<Key>,
    pub frame_prev: Vec<Key>,
    pub forward: Vec<Key>,
    pub backward: Vec<Key>,
}

impl Default for ReplayKeys {
    fn default() -> Self {
        Self {
            quit: vec![Key::Escape, Key::Char('q'), Key::Char('Q')],
            pause: vec![Key::Char(' '), Key::Char('p'), Key::Char('P')],
            speed_more: vec![Key::Char('+'), Key::Char('}')],
            speed_less: vec![Key::Char('-'), Key::Char('{')],
            frame_next: vec![Key::ArrowRight, Key::Char('l')],
            frame_prev: vec![Key::ArrowLeft, Key::Char('h')],
            forward: vec![Key::ArrowUp, Key::Char('k')],
            backward: vec![Key::ArrowDown, Key::Char('j')],
        }
    }
}

fn key_in(key: &Key, keys: &[Key]) -> bool {
    keys.contains(key)
}

// ---------------------------------------------------------------------------
// ReplayAction
// ---------------------------------------------------------------------------

/// Actions the replay can perform on each update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayAction {
    None,
    Next,
    Previous,
    TogglePause,
    SpeedMore,
    SpeedLess,
    Forward,
    Backward,
}

// ---------------------------------------------------------------------------
// Internal tick message
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Replay
// ---------------------------------------------------------------------------

/// Configuration for creating a [`Replay`].
pub struct ReplayConfig<R: Read> {
    pub grid: Grid,
    pub decoder: FrameDecoder<R>,
    pub keys: ReplayKeys,
}

/// Replays a recorded session frame-by-frame.
///
/// Implements the [`gruid_core::app::Model`] trait and can be used as the
/// main application model. Supports auto-play with adjustable speed,
/// pause/resume, frame stepping, and time-based seeking.
pub struct Replay<R: Read> {
    decoder: FrameDecoder<R>,
    frames: Vec<Frame>,
    grid: Grid,
    undo: Vec<Vec<FrameCell>>,
    fidx: usize,
    auto_play: bool,
    /// Speed multiplier (1 = normal, 2 = 2x, etc.)
    speed: u32,
    action: ReplayAction,
    is_init: bool,
    keys: ReplayKeys,
    dirty: bool,
}

impl<R: Read> Replay<R> {
    /// Create a new replay from configuration.
    pub fn new(cfg: ReplayConfig<R>) -> Self {
        Self {
            decoder: cfg.decoder,
            frames: Vec::new(),
            grid: cfg.grid,
            undo: Vec::new(),
            fidx: 0,
            auto_play: true,
            speed: 1,
            action: ReplayAction::None,
            is_init: false,
            keys: cfg.keys,
            dirty: true,
        }
    }

    /// Set the current frame index.
    pub fn set_frame(&mut self, n: usize) {
        while self.fidx < n {
            self.decode_next();
            if self.fidx >= self.frames.len() {
                break;
            }
            self.fidx += 1;
            self.apply_next();
        }
        while self.fidx > n {
            if self.fidx == 0 {
                break;
            }
            self.fidx -= 1;
            self.apply_previous();
        }
        self.dirty = true;
    }

    /// Seek forward/backward by the given duration (in milliseconds).
    /// Positive = forward, negative = backward.
    pub fn seek_ms(&mut self, delta_ms: i64) {
        self.decode_next();
        if self.frames.is_empty() {
            return;
        }
        if self.fidx == 0 || self.fidx > self.frames.len() {
            return;
        }
        let current_time = self.frames[self.fidx - 1].time_ms as i64;
        let target_time = current_time + delta_ms;

        if delta_ms > 0 {
            while self.fidx < self.frames.len()
                && (self.frames[self.fidx - 1].time_ms as i64) < target_time
            {
                self.decode_next();
                if self.fidx >= self.frames.len() {
                    break;
                }
                self.fidx += 1;
                self.apply_next();
            }
        } else {
            while self.fidx > 1
                && (self.frames[self.fidx - 1].time_ms as i64) > target_time
            {
                self.fidx -= 1;
                self.apply_previous();
            }
        }
        self.dirty = true;
    }

    /// The current frame index.
    pub fn frame_index(&self) -> usize {
        self.fidx
    }

    /// Whether auto-play is active.
    pub fn is_auto_play(&self) -> bool {
        self.auto_play
    }

    /// Current speed multiplier.
    pub fn speed(&self) -> u32 {
        self.speed
    }

    fn decode_next(&mut self) {
        if self.fidx >= self.frames.len() {
            // Try to read one more frame.
            if let Ok(Some(frame)) = self.decoder.decode() {
                self.frames.push(frame);
            }
        }
    }

    fn apply_next(&mut self) {
        if self.fidx == 0 || self.fidx > self.frames.len() {
            return;
        }
        let frame = &self.frames[self.fidx - 1];
        // Save undo info.
        let mut undo_cells = Vec::with_capacity(frame.cells.len());
        for fc in &frame.cells {
            let old_cell = self.grid.at(fc.pos);
            undo_cells.push(FrameCell {
                cell: old_cell,
                pos: fc.pos,
            });
            self.grid.set(fc.pos, fc.cell);
        }
        self.undo.push(undo_cells);
    }

    fn apply_previous(&mut self) {
        if let Some(undo_cells) = self.undo.pop() {
            for fc in &undo_cells {
                self.grid.set(fc.pos, fc.cell);
            }
        }
    }

    fn handle_action(&mut self) {
        match self.action {
            ReplayAction::Next => {
                self.decode_next();
                if self.fidx >= self.frames.len() {
                    self.action = ReplayAction::None;
                    return;
                }
                self.fidx += 1;
            }
            ReplayAction::Previous => {
                if self.fidx == 0 {
                    self.action = ReplayAction::None;
                    return;
                }
                self.fidx -= 1;
            }
            ReplayAction::TogglePause => {
                self.auto_play = !self.auto_play;
            }
            ReplayAction::SpeedMore => {
                self.speed = (self.speed * 2).min(64);
            }
            ReplayAction::SpeedLess => {
                self.speed = (self.speed / 2).max(1);
            }
            _ => {}
        }
    }

    fn apply_draw_action(&mut self) {
        match self.action {
            ReplayAction::Next => self.apply_next(),
            ReplayAction::Previous => self.apply_previous(),
            ReplayAction::Forward => self.seek_ms(60_000),
            ReplayAction::Backward => self.seek_ms(-60_000),
            _ => {}
        }
        if self.action != ReplayAction::None {
            self.dirty = true;
        }
    }

    fn tick_effect(&self) -> Option<Effect> {
        if !self.auto_play || self.fidx > self.frames.len() {
            return None;
        }

        let delay_ms = if self.fidx > 0 && self.fidx < self.frames.len() {
            let prev_t = self.frames[self.fidx - 1].time_ms;
            let curr_t = self.frames[self.fidx].time_ms;
            let d = curr_t.saturating_sub(prev_t);
            // Cap at 2 seconds
            let d = d.min(2000);
            // Apply speed
            let d = d / self.speed as u64;
            // Minimum interval ~4ms
            d.max(4)
        } else {
            4
        };

        let fidx = self.fidx;
        Some(Effect::Cmd(Box::new(move || {
            std::thread::sleep(Duration::from_millis(delay_ms));
            Some(Msg::Tick { frame: fidx })
        })))
    }

    /// Process a message, returning an optional effect.
    pub fn update(&mut self, msg: Msg) -> Option<Effect> {
        self.action = ReplayAction::None;

        match msg {
            Msg::Init => {
                self.is_init = true;
                self.decode_next();
                return self.tick_effect();
            }
            Msg::KeyDown { key, .. } => {
                if key_in(&key, &self.keys.quit) {
                    if self.is_init {
                        return Some(Effect::End);
                    }
                } else if key_in(&key, &self.keys.pause) {
                    self.action = ReplayAction::TogglePause;
                } else if key_in(&key, &self.keys.speed_more) {
                    self.action = ReplayAction::SpeedMore;
                } else if key_in(&key, &self.keys.speed_less) {
                    self.action = ReplayAction::SpeedLess;
                } else if key_in(&key, &self.keys.frame_next) {
                    self.action = ReplayAction::Next;
                    self.auto_play = false;
                } else if key_in(&key, &self.keys.frame_prev) {
                    self.action = ReplayAction::Previous;
                    self.auto_play = false;
                } else if key_in(&key, &self.keys.forward) {
                    self.action = ReplayAction::Forward;
                } else if key_in(&key, &self.keys.backward) {
                    self.action = ReplayAction::Backward;
                }
            }
            Msg::Tick { frame } => {
                if self.auto_play && self.fidx == frame {
                    self.action = ReplayAction::Next;
                }
            }
            _ => {}
        }

        self.handle_action();
        self.apply_draw_action();

        if self.auto_play && self.fidx <= self.frames.len() && self.action != ReplayAction::None {
            self.tick_effect()
        } else {
            None
        }
    }

    /// Render the current replay state into the grid.
    pub fn draw(&self, grid: &mut Grid) {
        grid.copy_from(&self.grid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gruid_core::cell::Cell;
    use gruid_core::geom::Point;
    use gruid_core::recording::FrameEncoder;
    fn make_test_frames() -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut enc = FrameEncoder::new(&mut buf);
            for i in 0..5 {
                let frame = Frame {
                    cells: vec![FrameCell {
                        pos: Point::new(i, 0),
                        cell: Cell::default()
                            .with_char(char::from(b'A' + i as u8)),
                    }],
                    width: 10,
                    height: 5,
                    time_ms: i as u64 * 100,
                };
                enc.encode(&frame).unwrap();
            }
        }
        buf
    }

    #[test]
    fn replay_step_through_frames() {
        let data = make_test_frames();
        let decoder = FrameDecoder::new(data.as_slice());
        let grid = Grid::new(10, 5);

        let mut replay = Replay::new(ReplayConfig {
            grid,
            decoder,
            keys: ReplayKeys::default(),
        });

        // Init
        let _ = replay.update(Msg::Init);
        assert_eq!(replay.frame_index(), 0);

        // Step forward
        replay.action = ReplayAction::Next;
        replay.handle_action();
        replay.apply_draw_action();
        assert_eq!(replay.frame_index(), 1);
        // Check the cell was applied
        assert_eq!(replay.grid.at(Point::new(0, 0)).ch, 'A');

        // Step forward again
        replay.action = ReplayAction::Next;
        replay.handle_action();
        replay.apply_draw_action();
        assert_eq!(replay.frame_index(), 2);
        assert_eq!(replay.grid.at(Point::new(1, 0)).ch, 'B');

        // Step backward
        replay.action = ReplayAction::Previous;
        replay.handle_action();
        replay.apply_draw_action();
        assert_eq!(replay.frame_index(), 1);
        // Undo should have reverted the 'B' cell
        assert_eq!(replay.grid.at(Point::new(1, 0)).ch, ' ');
    }

    #[test]
    fn replay_speed_control() {
        let data = make_test_frames();
        let decoder = FrameDecoder::new(data.as_slice());
        let grid = Grid::new(10, 5);

        let mut replay = Replay::new(ReplayConfig {
            grid,
            decoder,
            keys: ReplayKeys::default(),
        });

        assert_eq!(replay.speed(), 1);
        replay.action = ReplayAction::SpeedMore;
        replay.handle_action();
        assert_eq!(replay.speed(), 2);
        replay.action = ReplayAction::SpeedMore;
        replay.handle_action();
        assert_eq!(replay.speed(), 4);
        replay.action = ReplayAction::SpeedLess;
        replay.handle_action();
        assert_eq!(replay.speed(), 2);
    }

    #[test]
    fn replay_set_frame() {
        let data = make_test_frames();
        let decoder = FrameDecoder::new(data.as_slice());
        let grid = Grid::new(10, 5);

        let mut replay = Replay::new(ReplayConfig {
            grid,
            decoder,
            keys: ReplayKeys::default(),
        });

        replay.set_frame(3);
        assert_eq!(replay.frame_index(), 3);

        // Cells from frames 0, 1, 2 should be applied
        assert_eq!(replay.grid.at(Point::new(0, 0)).ch, 'A');
        assert_eq!(replay.grid.at(Point::new(1, 0)).ch, 'B');
        assert_eq!(replay.grid.at(Point::new(2, 0)).ch, 'C');

        // Seek back
        replay.set_frame(1);
        assert_eq!(replay.frame_index(), 1);
        assert_eq!(replay.grid.at(Point::new(2, 0)).ch, ' ');
    }
}
