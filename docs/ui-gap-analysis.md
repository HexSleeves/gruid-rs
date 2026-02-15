# gruid UI Package — Go vs Rust Gap Analysis

Legend: ✅ = present in Rust, ❌ = missing, ⚠️ = partial/different

---

## Box / Alignment

| Go | Rust | Status |
|---|---|---|
| `type Alignment` (int16) | `enum Alignment` | ✅ |
| `AlignCenter`, `AlignLeft`, `AlignRight` | `Center`, `Left`, `Right` | ✅ |
| `type Box` struct | `struct BoxDecor` | ✅ (renamed) |
| `Box.Style` | `BoxDecor.style` | ✅ |
| `Box.Title` (StyledText) | `BoxDecor.title` | ✅ |
| `Box.Footer` (StyledText) | `BoxDecor.footer` | ✅ |
| `Box.AlignTitle` | `BoxDecor.align_title` | ✅ |
| `Box.AlignFooter` | `BoxDecor.align_footer` | ✅ |
| `Box.Draw(gd Grid) Grid` | `BoxDecor.draw(&Grid) -> Range` | ⚠️ returns Range instead of Grid |
| `StyledText.drawTextLine(gd, align)` (internal) | inline in `BoxDecor.draw` | ⚠️ title/footer drawn char-by-char, ignores StyledText markup styles |

---

## Label

| Go | Rust | Status |
|---|---|---|
| `type Label` struct | `struct Label` | ✅ |
| `Label.Content` (StyledText) | `Label.content` | ✅ |
| `Label.Box` (*Box) | `Label.box_` (Option<BoxDecor>) | ✅ |
| `Label.AdjustWidth` (bool) | `Label.adjust_width` | ⚠️ field exists but not used in `draw()` |
| `NewLabel(StyledText) *Label` | `Label::new(StyledText)` | ✅ |
| `Label.SetText(string)` | `Label::set_text(&str)` | ✅ |
| `Label.Draw(gd Grid) Grid` | `Label::draw(&Grid) -> Range` | ⚠️ Go fills bg with style before content draw; Rust doesn't fill bg. Go adjusts grid slice size; Rust doesn't. |

---

## Menu

| Go | Rust | Status |
|---|---|---|
| `type MenuConfig` struct | `struct MenuConfig` | ✅ |
| `MenuConfig.Grid` | `MenuConfig.grid` | ✅ |
| `MenuConfig.Entries` | `MenuConfig.entries` | ✅ |
| `MenuConfig.Keys` | `MenuConfig.keys` | ✅ |
| `MenuConfig.Box` | `MenuConfig.box_` | ✅ |
| `MenuConfig.Style` | `MenuConfig.style` | ✅ |
| `type MenuEntry` struct | `struct MenuEntry` | ✅ |
| `MenuEntry.Text` | `MenuEntry.text` | ✅ |
| `MenuEntry.Disabled` | `MenuEntry.disabled` | ✅ |
| `MenuEntry.Keys` (shortcuts) | `MenuEntry.keys` | ✅ |
| `type MenuKeys` struct | `struct MenuKeys` | ✅ |
| `MenuKeys.Up/Down/Left/Right/PageDown/PageUp/Invoke/Quit` | all present | ✅ |
| `type MenuStyle` struct | `struct MenuStyle` | ✅ |
| `MenuStyle.Layout` (Point) | `MenuStyle.layout` | ✅ |
| `MenuStyle.Active` (Style) | `MenuStyle.active` | ✅ |
| `MenuStyle.PageNum` (Style) | `MenuStyle.page_num` | ✅ |
| — | `MenuStyle.disabled` (Style) | ✅ (Rust extra) |
| `type MenuAction` (int) | `enum MenuAction` | ✅ |
| `MenuPass/MenuMove/MenuInvoke/MenuQuit` | `Pass/Move/Invoke/Quit` | ✅ |
| `type Menu` struct | `struct Menu` | ✅ |
| `NewMenu(MenuConfig) *Menu` | `Menu::new(MenuConfig)` | ✅ |
| `Menu.Active() int` | `Menu::active() -> usize` | ⚠️ Go returns raw index (incl. disabled); Rust returns flat index |
| `Menu.ActiveInvokable() int` | — | ❌ |
| `Menu.ActiveBounds() Range` | `Menu::active_bounds() -> Range` | ✅ |
| `Menu.Bounds() Range` | `Menu::bounds() -> Range` | ✅ |
| `Menu.Action() MenuAction` | `Menu::action() -> MenuAction` | ✅ |
| `Menu.SetEntries([]MenuEntry)` | `Menu::set_entries(Vec<MenuEntry>)` | ✅ |
| `Menu.SetBox(*Box)` | `Menu::set_box(Option<BoxDecor>)` | ✅ |
| `Menu.SetActive(int)` (raw index) | `Menu::set_active(usize)` | ✅ |
| `Menu.SetActiveInvokable(int)` | — | ❌ |
| `Menu.Update(Msg) Effect` | `Menu::update(Msg) -> MenuAction` | ⚠️ returns action directly, no Effect |
| `Menu.Draw() Grid` | `Menu::draw(&Grid)` | ⚠️ Go returns Grid, uses dirty-checking; Rust takes external Grid, no dirty-checking |
| 2D grid layout (table, line, column) | flat 1D list only | ❌ multi-column/table layout not implemented |
| Page number display in box footer | — | ❌ |
| Mouse: click outside = Quit | — | ❌ click-outside-quit not implemented |
| Mouse: WheelUp/WheelDown for paging | — | ❌ |
| `Menu.pages` (multi-page X,Y tracking) | `Menu.page` (single linear page) | ⚠️ simplified pagination |

---

## Pager

| Go | Rust | Status |
|---|---|---|
| `type PagerConfig` struct | `struct PagerConfig` | ✅ |
| `PagerConfig.Grid` | `PagerConfig.grid` | ✅ |
| `PagerConfig.Lines` ([]StyledText) | `PagerConfig.content` (StyledText) | ⚠️ Go takes pre-split lines; Rust takes single text and auto-formats/splits |
| `PagerConfig.Box` | `PagerConfig.box_` | ✅ |
| `PagerConfig.Keys` | `PagerConfig.keys` | ✅ |
| `PagerConfig.Style` | `PagerConfig.style` | ✅ |
| `type PagerStyle` struct | `struct PagerStyle` | ✅ |
| `PagerStyle.LineNum` | `PagerStyle.page_num` | ✅ |
| `type PagerKeys` struct | `struct PagerKeys` | ✅ |
| `PagerKeys.Down/Up/Left/Right` | same | ✅ |
| `PagerKeys.Start` (go to col 0) | — | ❌ |
| `PagerKeys.PageDown/PageUp` | same | ✅ |
| `PagerKeys.HalfPageDown/HalfPageUp` | same | ✅ |
| `PagerKeys.Top/Bottom` | same | ✅ |
| `PagerKeys.Quit` | same | ✅ |
| `type PagerAction` (int) | `enum PagerAction` | ✅ |
| `PagerPass/PagerMove/PagerQuit` | `Pass/Scroll/Quit` | ✅ (renamed Move→Scroll) |
| `type Pager` struct | `struct Pager` | ✅ |
| `NewPager(PagerConfig) *Pager` | `Pager::new(PagerConfig)` | ✅ |
| `Pager.SetCursor(Point)` (x,y) | `Pager::set_cursor(y: i32)` | ⚠️ Rust: y only, no x |
| `Pager.SetBox(*Box)` | `Pager::set_box(Option<BoxDecor>)` | ✅ |
| `Pager.SetLines([]StyledText)` | `Pager::set_lines(Vec<StyledText>)` | ✅ |
| `Pager.View() Range` (min/max x,y) | `Pager::view() -> Point` (x,y scroll pos) | ⚠️ Go returns Range, Rust returns Point |
| `Pager.Lines() int` | — | ❌ |
| `Pager.Action() PagerAction` | `Pager::action() -> PagerAction` | ✅ |
| `Pager.Update(Msg) Effect` | `Pager::update(Msg) -> PagerAction` | ⚠️ Go returns Effect; Rust returns action |
| `Pager.Draw() Grid` | `Pager::draw(&Grid)` | ⚠️ different signature (see Menu) |
| MsgInit handling (main model mode) | — | ❌ no init/main-model support |
| Mouse: click top/bottom half = page up/down | — | ❌ mouse click paging |
| Line number display in box footer | — | ❌ |
| `Pager.right()` scrolls by 8 cols | `scroll_by_x(1)` | ⚠️ Go scrolls 8 cols; Rust scrolls 1 |

---

## StyledText / Markup

| Go | Rust | Status |
|---|---|---|
| `type StyledText` struct | `struct StyledText` | ✅ |
| `Text(string) StyledText` | `StyledText::text(&str)` | ✅ |
| `Textf(format, ...) StyledText` | `StyledText::textf(String)` | ⚠️ Rust takes pre-formatted String, no format args |
| `NewStyledText(text, style)` | `StyledText::new(&str, Style)` | ✅ |
| `stt.Text() string` | `stt.content() -> &str` | ✅ (renamed) |
| `stt.WithText(string)` | `stt.with_text(&str)` | ✅ |
| `stt.WithTextf(format, ...)` | — | ❌ |
| `stt.Style() Style` | `stt.style() -> Style` | ✅ |
| `stt.WithStyle(Style)` | `stt.with_style(Style)` | ✅ |
| `stt.With(text, style)` | — | ❌ (combine with_text + with_style) |
| `stt.WithMarkup(rune, Style)` | `stt.with_markup(char, Style)` | ✅ |
| `stt.WithMarkups(map)` | `stt.with_markups(HashMap)` | ✅ |
| `stt.Markups() map` | `stt.markups() -> HashMap` | ✅ |
| `stt.Iter(fn(Point, Cell)) Point` | `stt.iter(FnMut(Point, Cell)) -> Point` | ⚠️ Go markup uses `@r` two-char sequences; Rust treats single marker chars as style-switchers (different markup protocol) |
| `stt.Size() Point` | `stt.size() -> Point` | ✅ |
| `stt.Format(width) StyledText` | `stt.format(width) -> StyledText` | ✅ |
| `stt.Lines() []StyledText` | `stt.lines() -> Vec<StyledText>` | ⚠️ Go preserves inter-line markup state with `@r` prefix; Rust does not |
| `stt.Draw(Grid) Grid` | `stt.draw(&Grid) -> Range` | ⚠️ Go returns sub-Grid; Rust returns Range |
| `@` prefix markup (`@r` = switch, `@@` = literal `@`, `@N` = reset) | single-char markup (marker char = switch, no escape, no reset) | ⚠️ **Different markup protocol** — Go uses `@` prefix system; Rust uses bare marker chars |
| Carriage return (`\r`) handling | — | ❌ `\r` not stripped |

---

## TextInput

| Go | Rust | Status |
|---|---|---|
| `type TextInputConfig` struct | `struct TextInputConfig` | ✅ |
| `TextInputConfig.Grid` | `TextInputConfig.grid` | ✅ |
| `TextInputConfig.Text` (StyledText) | `TextInputConfig.content` (String) | ⚠️ Go uses StyledText; Rust uses plain String |
| `TextInputConfig.Prompt` (StyledText) | `TextInputConfig.prompt` (Option<StyledText>) | ✅ |
| `TextInputConfig.Box` | `TextInputConfig.box_` | ✅ |
| `TextInputConfig.Keys` | `TextInputConfig.keys` | ✅ |
| `TextInputConfig.Style` | `TextInputConfig.style` | ✅ |
| `type TextInputStyle` struct | `struct TextInputStyle` | ✅ |
| `TextInputStyle.Cursor` (Style) | `TextInputStyle.cursor` | ✅ |
| — | `TextInputStyle.text` (Style) | ✅ (Rust extra) |
| `type TextInputKeys` struct | `struct TextInputKeys` | ⚠️ |
| `TextInputKeys.Quit` | `TextInputKeys.cancel` | ✅ (renamed) |
| — | `TextInputKeys.confirm` | ✅ (Rust extra — Go hardcodes Enter) |
| `type TextInputAction` (int) | `enum TextInputAction` | ✅ |
| `TextInputPass/Change/Invoke/Quit` | `Pass/Change/Confirm/Cancel` | ✅ (renamed) |
| `type TextInput` struct | `struct TextInput` | ✅ |
| `NewTextInput(cfg) *TextInput` | `TextInput::new(cfg)` | ✅ |
| `TextInput.SetCursor(int)` | `TextInput::set_cursor(usize)` | ✅ |
| `TextInput.SetBox(*Box)` | `TextInput::set_box(Option<BoxDecor>)` | ✅ |
| `TextInput.Content() string` | `TextInput::content() -> &str` | ✅ |
| `TextInput.Action() TextInputAction` | `TextInput::action()` | ✅ |
| `TextInput.Update(Msg) Effect` | `TextInput::update(Msg) -> TextInputAction` | ⚠️ returns action directly |
| `TextInput.Draw() Grid` | `TextInput::draw(&Grid)` | ⚠️ different signature |
| — | `TextInput::set_content(&str)` | ✅ (Rust extra) |
| — | `TextInput::set_prompt(Option<StyledText>)` | ✅ (Rust extra) |
| Cursor auto-reverse default style | — | ❌ auto fg/bg swap for cursor |
| Key::Delete support | `Key::Delete` | ✅ (Rust extra — Go lacks Delete key) |
| dirty-checking / drawn caching | — | ❌ no dirty tracking |

---

## Replay

| Go | Rust | Status |
|---|---|---|
| `type ReplayConfig` struct | `struct ReplayConfig<R: Read>` | ✅ |
| `ReplayConfig.Grid` | `ReplayConfig.grid` | ✅ |
| `ReplayConfig.FrameDecoder` | `ReplayConfig.decoder` | ✅ |
| `ReplayConfig.Keys` | `ReplayConfig.keys` | ✅ |
| `type ReplayKeys` struct | `struct ReplayKeys` | ✅ |
| `ReplayKeys.Quit/Pause/SpeedMore/SpeedLess/FrameNext/FramePrev/Forward/Backward` | all present | ✅ |
| `ReplayKeys.Help` | — | ❌ |
| `type Replay` struct | `struct Replay<R: Read>` | ✅ |
| `NewReplay(cfg) *Replay` | `Replay::new(cfg)` | ✅ |
| `Replay.SetFrame(int)` | `Replay::set_frame(usize)` | ✅ |
| `Replay.Seek(time.Duration)` | `Replay::seek_ms(i64)` | ⚠️ Go uses Duration; Rust uses i64 ms |
| `Replay.Update(Msg) Effect` | `Replay::update(Msg) -> Option<Effect>` | ✅ |
| `Replay.Draw() Grid` | `Replay::draw(&mut Grid)` | ⚠️ Go returns grid; Rust copies into provided grid |
| Help overlay (embedded Pager) | — | ❌ |
| Mouse interaction (toggle pause, step) | — | ❌ |
| `Replay.frame_index() -> usize` | `Replay::frame_index()` | ✅ |
| `Replay.is_auto_play() -> bool` | `Replay::is_auto_play()` | ✅ |
| `Replay.speed() -> u32` | `Replay::speed()` | ✅ |
| Grid auto-resize on larger frames | — | ❌ |
| dirty-checking | `dirty` field exists | ⚠️ field exists but not fully utilized |

---

## Summary of Key Gaps

### Critical (behavioral differences):
1. **StyledText markup protocol** — Go uses `@r` two-char prefix system (`@@`=escape, `@N`=reset); Rust uses single marker chars as switches (no escape, no reset)
2. **Menu 2D layout** — Go supports table/column/line arrangements; Rust is flat 1D list only
3. **gruid.Model interface** — Go widgets implement `Update(Msg) Effect` + `Draw() Grid` with dirty-checking; Rust widgets use `update(Msg)->Action` + `draw(&Grid)` pattern (no Model trait impl, no dirty-caching)

### Missing features:
4. `Menu.ActiveInvokable()` / `SetActiveInvokable()` — index ignoring disabled entries
5. `PagerKeys.Start` — go to column 0
6. `Pager.Lines()` — get line count  
7. Page/line number display in box footers (Menu and Pager)
8. Replay help overlay with embedded Pager
9. Replay mouse interaction
10. MsgInit / main-model mode for Pager and Replay
11. `StyledText.WithTextf()` / `StyledText.With(text, style)`
12. `StyledText.Lines()` markup state preservation across line breaks
13. `\r` carriage return handling in StyledText
14. TextInput auto-reverse cursor style default
