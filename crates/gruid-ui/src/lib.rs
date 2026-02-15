//! UI widgets for gruid: menu, pager, text input, label, styled text, replay.

mod box_;
mod label;
mod menu;
mod pager;
pub mod replay;
mod styled_text;
mod text_input;

pub use box_::{Alignment, BoxDecor};
pub use label::Label;
pub use menu::{Menu, MenuAction, MenuConfig, MenuEntry, MenuKeys, MenuStyle};
pub use pager::{Pager, PagerAction, PagerConfig, PagerKeys, PagerStyle};
pub use replay::{Replay, ReplayAction, ReplayConfig, ReplayKeys};
pub use styled_text::StyledText;
pub use text_input::{TextInput, TextInputAction, TextInputConfig, TextInputKeys, TextInputStyle};
