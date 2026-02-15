# gruid Core API Checklist: Go → Rust Port

Legend: ✅ = present in Rust, ❌ = missing, ⚠️ = partial/different

---

## 1. Point / Geometry (`geom.rs` ← `grid.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Point struct { X, Y int }` | `pub struct Point { pub x: i32, pub y: i32 }` | ✅ |
| `Point.String()` | `impl Display for Point` | ✅ |
| `Point.Shift(x, y int) Point` | `Point::shift(dx, dy)` | ✅ |
| `Point.Add(q Point) Point` | `impl Add for Point` | ✅ |
| `Point.Sub(q Point) Point` | `impl Sub for Point` | ✅ |
| `Point.Mul(k int) Point` | `impl Mul<i32> for Point` | ✅ |
| `Point.Div(k int) Point` | `impl Div<i32> for Point` | ✅ |
| `Point.In(rg Range) bool` | `Point::in_range(&Range)` | ✅ |
| _(no Go equivalent)_ | `Point::ZERO` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Point::neighbors_4()` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Point::neighbors_8()` | ✅ (Rust extra) |

## 2. Range / Geometry (`geom.rs` ← `grid.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Range struct { Min, Max Point }` | `pub struct Range { pub min: Point, pub max: Point }` | ✅ |
| `NewRange(x0, y0, x1, y1) Range` (auto-canonicalize) | `Range::new(x0, y0, x1, y1)` (auto-canonicalize) | ✅ |
| `Range.String()` | `impl Display for Range` | ✅ |
| `Range.Size() Point` | `Range::size()` | ✅ |
| `Range.Shift(x0, y0, x1, y1) Range` | `Range::shift(dx0, dy0, dx1, dy1)` | ⚠️ Go version returns empty if result is empty; Rust version does not check |
| `Range.Line(y int) Range` | `Range::line(y)` | ⚠️ Go uses relative y and returns empty if OOB; Rust takes absolute y, no OOB check |
| `Range.Lines(y0, y1 int) Range` | `Range::lines(y0, y1)` | ⚠️ Go uses relative y and intersects; Rust takes absolute coords, no intersect |
| `Range.Column(x int) Range` | `Range::column(x)` | ⚠️ Go uses relative x and returns empty if OOB; Rust takes absolute x, no OOB check |
| `Range.Columns(x0, x1 int) Range` | `Range::columns(x0, x1)` | ⚠️ Go uses relative x and intersects; Rust takes absolute coords, no intersect |
| `Range.Empty() bool` | `Range::is_empty()` | ✅ |
| `Range.Eq(r Range) bool` (empty equality) | `impl PartialEq` (derived; doesn't treat all empties as equal) | ⚠️ Go treats all empties as equal; Rust's derived PartialEq does not |
| `Range.Sub(p Point) Range` (translate by -p) | _(missing)_ | ❌ |
| `Range.Add(p Point) Range` (translate by +p) | _(missing)_ | ❌ |
| `Range.RelMsg(msg Msg) Msg` | _(missing)_ | ❌ |
| `Range.Intersect(r Range) Range` | `Range::intersect(other)` | ✅ |
| `Range.Union(r Range) Range` | `Range::union(other)` | ✅ |
| `Range.Overlaps(r Range) bool` | `Range::overlaps(other)` | ✅ |
| `Range.In(r Range) bool` (containment) | _(missing)_ | ❌ |
| `Range.Iter(fn(Point))` _(deprecated)_ | `Range::iter()` returns `RangeIter` | ✅ (improved) |
| `Range.Points() iter.Seq[Point]` | `Range::iter()` / `impl IntoIterator` | ✅ |
| _(no Go equivalent)_ | `Range::width()` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Range::height()` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Range::len()` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Range::contains(p)` | ✅ (Rust extra; Go uses `Point.In`) |

## 3. Style / Color / AttrMask (`style.rs` ← `grid.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Color uint32` | `pub struct Color(pub u32)` | ✅ |
| `const ColorDefault Color = 0` | `Color::DEFAULT` | ✅ |
| _(no Go equivalent)_ | `Color::from_rgb(r, g, b)` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Color::r()`, `Color::g()`, `Color::b()` | ✅ (Rust extra) |
| `type AttrMask uint32` | `pub struct AttrMask(pub u32)` | ✅ |
| `const AttrsDefault AttrMask = 0` | `AttrMask::NONE` | ✅ |
| _(no Go equivalent — user defines)_ | `AttrMask::BOLD`, `ITALIC`, `UNDERLINE`, `BLINK`, `REVERSE`, `DIM` | ✅ (Rust extra: named attrs) |
| _(no Go equivalent)_ | `AttrMask::contains()`, `is_empty()` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `impl BitOr`, `BitAnd` for `AttrMask` | ✅ (Rust extra) |
| `type Style struct { Fg, Bg Color; Attrs AttrMask }` | `pub struct Style { pub fg, pub bg, pub attrs }` | ✅ |
| `Style.WithFg(Color) Style` | `Style::with_fg(Color)` | ✅ |
| `Style.WithBg(Color) Style` | `Style::with_bg(Color)` | ✅ |
| `Style.WithAttrs(AttrMask) Style` | `Style::with_attrs(AttrMask)` | ✅ |

## 4. Cell (`cell.rs` ← `grid.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Cell struct { Rune rune; Style Style }` | `pub struct Cell { pub ch: char, pub style: Style }` | ✅ |
| `Cell.WithRune(r rune) Cell` | `Cell::with_char(ch)` | ✅ |
| `Cell.WithStyle(st Style) Cell` | `Cell::with_style(style)` | ✅ |
| _(implicit zero value)_ | `impl Default for Cell` (ch=' ') | ✅ |

## 5. Grid (`grid.rs` ← `grid.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Grid struct` (slice semantics, shared underlying) | `pub struct Grid` (Rc<RefCell<GridBuffer>>, slice semantics) | ✅ |
| `NewGrid(w, h int) Grid` | `Grid::new(width, height)` | ✅ |
| `Grid.String() string` | _(missing)_ | ❌ |
| `Grid.Bounds() Range` | `Grid::bounds()` | ✅ |
| `Grid.Range() Range` (0-based) | `Grid::range_()` | ✅ |
| `Grid.Size() Point` | `Grid::size()` | ✅ |
| `Grid.Slice(rg Range) Grid` | `Grid::slice(rg)` | ✅ |
| `Grid.Resize(w, h int) Grid` | _(missing)_ | ❌ |
| `Grid.Contains(p Point) bool` | `Grid::contains(p)` | ✅ |
| `Grid.Set(p Point, c Cell)` | `Grid::set(p, cell)` | ✅ |
| `Grid.At(p Point) Cell` | `Grid::at(p)` | ✅ |
| `Grid.Fill(c Cell)` | `Grid::fill(cell)` | ✅ |
| `Grid.Iter(fn(Point, Cell))` _(deprecated)_ | `Grid::iter()` returns `GridIter` | ✅ (improved) |
| `Grid.Map(fn(Point, Cell) Cell)` | `Grid::map_cells(f)` | ✅ |
| `Grid.Copy(src Grid) Point` | `Grid::copy_from(&Grid)` | ✅ |
| `Grid.All() iter.Seq2[Point, Cell]` | `Grid::iter()` returns `GridIter` | ✅ |
| `Grid.Points() iter.Seq[Point]` | _(missing — can use `.iter().map(|(p,_)| p)`)_ | ⚠️ No dedicated method |
| `type GridIterator struct` (stateful) | `pub struct GridIter` (standard Iterator) | ✅ |
| `GridIterator.Next() bool` | `Iterator::next()` | ✅ |
| `GridIterator.P() Point` | _(returned as tuple element)_ | ✅ |
| `GridIterator.Cell() Cell` | _(returned as tuple element)_ | ✅ |
| `GridIterator.SetCell(c Cell)` | _(missing — use `grid.set()` instead)_ | ⚠️ No mutable iterator |
| `GridIterator.SetP(p Point)` | _(missing — no random-access seek on iterator)_ | ❌ |
| `GridIterator.Reset()` | _(missing — recreate iterator instead)_ | ❌ |
| _(no Go equivalent)_ | `Grid::width()`, `Grid::height()` | ✅ (Rust extra) |

## 6. Frame / FrameCell (`grid.rs` ← `grid.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Frame struct { Time, Cells, Width, Height }` | `pub struct Frame { time_ms, cells, width, height }` | ⚠️ Time is `time.Time` in Go, `u64` ms in Rust |
| `type FrameCell struct { Cell Cell; P Point }` | `pub struct FrameCell { cell, pos }` | ✅ |
| `(*App).computeFrame()` (private) | `pub fn compute_frame(prev, curr)` (public, standalone) | ✅ (made public) |

## 7. Messages (`messages.rs` ← `messages.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Msg interface{}` | `pub enum Msg { ... }` | ✅ (enum instead of interface) |
| `type Key string` | `pub enum Key { ... }` | ✅ (enum instead of string) |
| `Key.In(keys []Key) bool` | _(missing)_ | ❌ |
| `Key.IsRune() bool` | _(not needed — `Key::Char(c)` variant is explicit)_ | ✅ (by design) |
| `const KeyArrowDown Key` | `Key::ArrowDown` | ✅ |
| `const KeyArrowLeft Key` | `Key::ArrowLeft` | ✅ |
| `const KeyArrowRight Key` | `Key::ArrowRight` | ✅ |
| `const KeyArrowUp Key` | `Key::ArrowUp` | ✅ |
| `const KeyBackspace Key` | `Key::Backspace` | ✅ |
| `const KeyDelete Key` | `Key::Delete` | ✅ |
| `const KeyEnd Key` | `Key::End` | ✅ |
| `const KeyEnter Key` | `Key::Enter` | ✅ |
| `const KeyEscape Key` | `Key::Escape` | ✅ |
| `const KeyHome Key` | `Key::Home` | ✅ |
| `const KeyInsert Key` | `Key::Insert` | ✅ |
| `const KeyPageDown Key` | `Key::PageDown` | ✅ |
| `const KeyPageUp Key` | `Key::PageUp` | ✅ |
| `const KeySpace Key` | `Key::Space` | ✅ |
| `const KeyTab Key` | `Key::Tab` | ✅ |
| `type ModMask int16` | `pub struct ModMask(pub u8)` | ✅ |
| `ModMask.String()` | `impl Display for ModMask` | ⚠️ Display is present but shows single values, not combos like Go |
| `const ModShift` | `ModMask::SHIFT` | ✅ |
| `const ModCtrl` | `ModMask::CTRL` | ✅ |
| `const ModAlt` | `ModMask::ALT` | ✅ |
| `const ModMeta` | `ModMask::META` | ✅ |
| `const ModNone` | `ModMask::NONE` | ✅ |
| `type MsgKeyDown struct { Key, Mod, Time }` | `Msg::KeyDown { key, modifiers, time }` | ✅ |
| `type MouseAction int` | `pub enum MouseAction` | ✅ |
| `MouseAction.String()` | `impl Display for MouseAction` | ✅ |
| `const MouseMain` | `MouseAction::Main` | ✅ |
| `const MouseAuxiliary` | `MouseAction::Auxiliary` | ✅ |
| `const MouseSecondary` | `MouseAction::Secondary` | ✅ |
| `const MouseWheelUp` | `MouseAction::WheelUp` | ✅ |
| `const MouseWheelDown` | `MouseAction::WheelDown` | ✅ |
| `const MouseRelease` | `MouseAction::Release` | ✅ |
| `const MouseMove` | `MouseAction::Move` | ✅ |
| `type MsgMouse struct { Action, P, Mod, Time }` | `Msg::Mouse { action, pos, modifiers, time }` | ✅ |
| `type MsgScreen struct { Width, Height, Time }` | `Msg::Screen { width, height, time }` | ✅ |
| `type MsgInit struct{}` | `Msg::Init` | ✅ |
| `type MsgQuit time.Time` | `Msg::Quit` | ⚠️ Go carries a `time.Time`, Rust is a bare variant |
| _(no Go equivalent)_ | `Msg::Custom(Arc<dyn Any>)` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Msg::key()`, `Msg::key_mod()` convenience ctors | ✅ (Rust extra) |
| _(no Go equivalent)_ | `Msg::custom()`, `Msg::downcast_ref()` | ✅ (Rust extra) |

## 8. App / Model / Driver (`app.rs` ← `ui.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type Model interface { Update(Msg) Effect; Draw() Grid }` | `pub trait Model { update(&mut self, Msg) -> Option<Effect>; draw(&self, &mut Grid) }` | ⚠️ Rust `draw` takes a `&mut Grid` param instead of returning one |
| `type Driver interface { Init, PollMsgs, Flush, Close }` | `pub trait Driver { init, poll_msgs, flush, close }` | ✅ |
| `type DriverPollMsg interface { PollMsg() (Msg, error) }` | _(not implemented — `EventLoopDriver` takes different approach)_ | ⚠️ Different design: `EventLoopDriver` replaces this |
| `type Effect interface { implementsEffect() }` | `pub enum Effect { Cmd, Sub, Batch, End }` | ✅ (enum instead of interface) |
| `type Cmd func() Msg` | `Effect::Cmd(Box<dyn FnOnce() -> Option<Msg>>)` | ✅ |
| `type Sub func(context.Context, chan<- Msg)` | `Effect::Sub(Box<dyn FnOnce(Context, Sender<Msg>)>)` | ✅ |
| `func End() Cmd` | `Effect::End` | ✅ |
| `func Batch(effs ...Effect) Effect` | `Effect::Batch(Vec<Effect>)` | ✅ |
| `type App struct` | `pub struct App<M, D>` | ✅ |
| `App.CatchPanics bool` | _(missing)_ | ❌ |
| `type AppConfig struct { Model, Driver, FrameWriter, Logger }` | `pub struct AppConfig<M, D> { model, driver, width, height, frame_writer }` | ⚠️ Missing `Logger`; added `width`/`height` |
| `NewApp(cfg AppConfig) *App` | `App::new(config)` | ✅ |
| `App.Start(ctx context.Context) error` | `App::run(&mut self)` | ⚠️ No context param (uses internal `Context`) |
| _(no Go equivalent)_ | `pub struct Context` (cancellation token) | ✅ (Rust extra) |
| _(no Go equivalent)_ | `pub trait EventLoopDriver { run(AppRunner) }` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `pub struct AppRunner` (for event-loop drivers) | ✅ (Rust extra) |
| _(no Go equivalent)_ | `AppRunner::new()`, `init()`, `handle_msg()`, `should_quit()`, `draw_frame()`, `resize()`, `process_pending_msgs()` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `pub fn cmd<F>(f) -> Effect` convenience ctor | ✅ (Rust extra) |

## 9. Recording (`recording.rs` ← `recording.go`)

| Go API | Rust API | Status |
|--------|----------|--------|
| `type FrameDecoder struct` | `pub struct FrameDecoder<R: Read>` | ✅ |
| `NewFrameDecoder(r io.Reader) (*FrameDecoder, error)` | `FrameDecoder::new(reader)` | ✅ |
| `FrameDecoder.Decode(*Frame) error` | `FrameDecoder::decode() -> io::Result<Option<Frame>>` | ✅ |
| _(private: `frameEncoder`)_ | `pub struct FrameEncoder<W: Write>` (made public) | ✅ (Rust promotes) |
| _(private: `newFrameEncoder`)_ | `FrameEncoder::new(writer)` | ✅ (Rust promotes) |
| _(private: `frameEncoder.encode`)_ | `FrameEncoder::encode(&Frame)` | ✅ (Rust promotes) |
| _(no Go equivalent)_ | `FrameEncoder::flush()` | ✅ (Rust extra) |
| _(no Go equivalent)_ | `FrameEncoder::into_inner()`, `FrameDecoder::into_inner()` | ✅ (Rust extra) |

**Note**: Go uses `gob+gzip` encoding; Rust uses a custom length-prefixed LE binary format. The wire formats are **not compatible**.

---

## Summary of Missing Items (❌)

1. **`Range.Sub(p Point) Range`** — translate range by -p
2. **`Range.Add(p Point) Range`** — translate range by +p
3. **`Range.RelMsg(msg Msg) Msg`** — make mouse coordinates range-relative
4. **`Range.In(r Range) bool`** — check if range is fully contained in another
5. **`Grid.String()`** — text representation of grid runes
6. **`Grid.Resize(w, h)`** — grow underlying buffer while preserving content
7. **`Grid.Points()`** — dedicated point-only iterator  
8. **`GridIterator.SetP(p)`** — seek iterator to a position
9. **`GridIterator.Reset()`** — reset iterator to start
10. **`Key.In(keys)`** — check if key is in a list
11. **`App.CatchPanics`** — panic recovery flag

## Summary of Partial Items (⚠️)

1. **`Range.Shift`** — Rust version doesn't check for resulting empty range
2. **`Range.Line / Lines / Column / Columns`** — Rust uses absolute coords without OOB checks; Go uses relative coords with intersection
3. **`Range.Eq`** — Rust derived `PartialEq` doesn't normalize empty ranges
4. **`GridIterator.SetCell`** — no mutable iteration (use `grid.set()` instead)
5. **`Frame.Time`** — Go uses `time.Time`; Rust uses `u64` milliseconds
6. **`MsgQuit`** — Go carries `time.Time`; Rust is a bare variant
7. **`ModMask.String()`** — Rust Display doesn't show combined modifier strings
8. **`Model.Draw()`** — Rust takes `&mut Grid` param instead of returning `Grid`
9. **`DriverPollMsg`** — Rust replaces with `EventLoopDriver` pattern
10. **`AppConfig`** — Rust missing `Logger`, adds `width`/`height`
11. **`App.Start`** — Rust `App::run` has no `Context` parameter
