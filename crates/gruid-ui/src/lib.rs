//! UI widgets for gruid: menu, pager, text input, label, styled text.

mod styled_text;
mod label;
mod box_;
mod menu;
mod pager;
mod text_input;

pub use styled_text::StyledText;
pub use label::Label;
pub use box_::{Alignment, BoxDecor};
pub use menu::{Menu, MenuAction, MenuConfig, MenuEntry, MenuKeys, MenuStyle};
pub use pager::{Pager, PagerAction, PagerConfig, PagerKeys, PagerStyle};
pub use text_input::{TextInput, TextInputAction, TextInputConfig, TextInputKeys, TextInputStyle};
