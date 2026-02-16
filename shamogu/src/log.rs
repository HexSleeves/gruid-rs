//! Game message log.

/// Style of a log entry, mapped to display color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogStyle {
    Normal,
    Confirm,
    Error,
    HurtMonster,
    HurtPlayer,
    Notable,
    Special,
    StatusEnd,
}

/// A single log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub text: String,
    pub style: LogStyle,
    pub tick: bool,
    pub dups: i32,
}

/// The game's message log.
pub struct GameLog {
    pub entries: Vec<LogEntry>,
    next_tick: bool,
}

impl GameLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_tick: false,
        }
    }

    /// Mark the start of a new turn (next entry gets a tick marker).
    pub fn new_turn(&mut self) {
        self.next_tick = true;
    }

    /// Add a message with a given style.
    pub fn log_styled(&mut self, text: &str, style: LogStyle) {
        // Uppercase first char
        let text = uppercase_first(text);

        // Dedup consecutive identical messages
        if let Some(last) = self.entries.last_mut() {
            if last.text == text && last.style == style && !self.next_tick {
                last.dups += 1;
                return;
            }
        }

        self.entries.push(LogEntry {
            text,
            style,
            tick: self.next_tick,
            dups: 0,
        });
        self.next_tick = false;

        // Trim if too many
        if self.entries.len() > 10000 {
            self.entries.drain(0..1000);
        }
    }

    /// Add a normal-style message.
    pub fn log(&mut self, text: &str) {
        self.log_styled(text, LogStyle::Normal);
    }

    /// Get the most recent entries for display, formatted into lines.
    /// Returns up to `max_lines` lines of width `width`.
    pub fn recent_lines(&self, width: usize, max_lines: usize) -> Vec<String> {
        let mut parts: Vec<String> = Vec::new();

        // Collect from newest to oldest
        for entry in self.entries.iter().rev() {
            let mut s = String::new();
            if entry.tick {
                s.push_str("• ");
            }
            s.push_str(&entry.text);
            if entry.dups > 0 {
                s.push_str(&format!(" ({}×)", entry.dups + 1));
            }
            parts.push(s);

            // Stop early if we have enough text
            let total_len: usize = parts.iter().map(|p| p.len() + 1).sum();
            if total_len > width * max_lines {
                break;
            }
        }

        // Reverse to chronological order and join
        parts.reverse();
        let joined = parts.join(" ");

        // Word wrap into lines
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();
        for word in joined.split_whitespace() {
            if !current.is_empty() && current.len() + 1 + word.len() > width {
                lines.push(current);
                current = String::new();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.is_empty() {
            lines.push(current);
        }

        // Take last max_lines
        if lines.len() > max_lines {
            lines = lines[lines.len() - max_lines..].to_vec();
        }
        lines
    }
}

fn uppercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
