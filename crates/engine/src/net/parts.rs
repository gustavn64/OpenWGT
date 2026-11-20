//! The Sender (`NSnd`) and the Receiver (`NRcv`): the network parts. A
//! Sender keeps what is wired into it for the host to send; a Receiver
//! gives what came in. Each has its channel name in its first text
//! setting, a lamp for the link and a display with the name.

use crate::gadget::{Behavior, Core, DrawCx, Object, Tag, Value};
use crate::gadgets::Class;
use crate::sim::Io;

use super::proto::Role;
use super::{NetValue, Status, clean_channel};

/// The Sender's tag.
pub const SENDER: Tag = Tag::new(b"NSnd");
/// The Receiver's tag.
pub const RECEIVER: Tag = Tag::new(b"NRcv");

/// The network parts' classes, after the game's (`gadgets::class`).
pub static CLASSES: &[Class] = &[
    Class {
        tag: SENDER,
        name: "cNetSender",
        gadget_id: 2900,
        sheet: 1900,
        knots: 1900,
        make: sender,
    },
    Class {
        tag: RECEIVER,
        name: "cNetReceiver",
        gadget_id: 2901,
        sheet: 1901,
        knots: 1901,
        make: receiver,
    },
];

fn sender() -> Box<dyn Behavior> {
    Box::new(Sender::default())
}

fn receiver() -> Box<dyn Behavior> {
    Box::new(Receiver::default())
}

/// The side of a channel a part of class `tag` is on, if it's a network
/// part.
#[must_use]
pub fn role(tag: Tag) -> Option<Role> {
    match tag {
        SENDER => Some(Role::Send),
        RECEIVER => Some(Role::Recv),
        _ => None,
    }
}

/// A network part's channel name: its first text setting, cleaned
/// ([`clean_channel`]); empty for none.
#[must_use]
pub fn channel(core: &Core) -> String {
    clean_channel(&core.settings.label_str(0))
}

/// Keeps the channel name as the relay takes it, in the settings too, so
/// that the dialog and a saved widget show what is used.
fn keep_channel_clean(core: &mut Core) {
    let clean = channel(core);
    if core.settings.label(0) != clean.as_bytes() {
        core.settings.set_label(0, clean.as_bytes());
    }
}

/// The string that a Sender waiting for the channel shows.
pub const BUSY_STRING: u32 = 20100;

/// The display's window in the parts' pictures (`h`, `v`, width, height),
/// where the code writes the channel name.
pub const SENDER_LCD: (i32, i32, i32, i32) = (14, 24, 66, 22);
pub const RECEIVER_LCD: (i32, i32, i32, i32) = (8, 24, 66, 22);

/// The display's font: the game's small one.
const LCD_FACE: &str = "Small Fonts";
const LCD_SIZE: u16 = 11;
/// From one line's top to the next's.
const LCD_PITCH: i32 = 10;
const LCD_GREEN: [u8; 3] = [120, 255, 150];
const LCD_RED: [u8; 3] = [255, 80, 60];
/// What the display shows with no channel.
const NO_CHANNEL: &str = "--------";

/// `text` in lines that fit a width, as [`lcd_lines`] measures them: one
/// line if it fits, else two, broken after the hyphen that evens them out
/// best (in the middle if there's none that fits); a line still too wide
/// loses characters off its end.
#[must_use]
pub fn lcd_lines(text: &str, width: i32, measure: impl Fn(&str) -> i32) -> Vec<String> {
    if measure(text) <= width {
        return vec![text.to_owned()];
    }
    let widest = |at: usize| measure(&text[..at]).max(measure(&text[at..]));
    let at = text
        .match_indices('-')
        .map(|(i, _)| i + 1)
        .filter(|&i| i < text.len() && widest(i) <= width)
        .min_by_key(|&i| widest(i))
        .unwrap_or(text.len().div_ceil(2));
    [&text[..at], &text[at..]]
        .into_iter()
        .map(|line| {
            let mut line = line.to_owned();
            while measure(&line) > width && line.pop().is_some() {}
            line
        })
        .collect()
}

/// Draws a network part: its lamp's frame, and on its display the
/// channel name, or BUSY in red for a waiting Sender.
fn draw_part(core: &Core, cx: &mut DrawCx<'_>, status: Status, lcd: (i32, i32, i32, i32)) {
    cx.frame_at_rect(core.sheet, status.lamp(), core.h, core.v);
    let (text, rgb) = if status.busy {
        let busy = cx.assets.string(BUSY_STRING).filter(|s| !s.is_empty());
        (busy.unwrap_or_else(|| "BUSY".to_owned()), LCD_RED)
    } else {
        let name = channel(core);
        (if name.is_empty() { NO_CHANNEL.to_owned() } else { name }, LCD_GREEN)
    };
    let c = cx.color(rgb);
    let (x, y, w, h) = lcd;
    let measure = |s: &str| cx.text_width(LCD_FACE, LCD_SIZE, false, &crate::text::encode(s));
    let lines = lcd_lines(&text, w - 2, measure);
    let n = i32::try_from(lines.len()).unwrap_or(1);
    let height = i32::from(LCD_SIZE) + (n - 1) * LCD_PITCH;
    let top = core.v + y + (h - height) / 2;
    for (i, line) in (0..).zip(&lines) {
        let bytes = crate::text::encode(line);
        let lw = cx.text_width(LCD_FACE, LCD_SIZE, false, &bytes);
        let left = core.h + x + (w - lw) / 2;
        cx.text(LCD_FACE, LCD_SIZE, false, left, top + i * LCD_PITCH, &bytes, c);
    }
}

/// The Sender (`NSnd`): sends what is wired into it, on/off, a number or
/// a picture, when it changes while the widget runs, and once on GO. A
/// sound or nothing wired sends nothing.
#[derive(Debug, Default)]
pub struct Sender {
    /// The link, as the host last told it.
    pub status: Status,
    /// A value to send, set when the input changes and on GO; the host
    /// takes it.
    pub outbox: Option<NetValue>,
    /// The input as last read: what the Sender has to send now (a Sender
    /// that gets the channel sends it at once).
    pub last: Option<NetValue>,
}

impl Sender {
    /// The input as a value to send, if it is one.
    fn read(io: &Io<'_>, core: &Core) -> Option<NetValue> {
        match io.input(core, 0) {
            Value::Bool(b) => Some(NetValue::Bool(b)),
            Value::Number(x) if x.is_finite() => Some(NetValue::Number(x)),
            Value::Object(Some(Object::Picture(n))) => Some(NetValue::Picture(n)),
            // No picture.
            Value::Object(None) => Some(NetValue::Picture(0)),
            _ => None,
        }
    }
}

impl Behavior for Sender {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        draw_part(core, cx, self.status, SENDER_LCD);
    }

    fn start(&mut self, core: &mut Core, io: &mut Io<'_>) {
        io.base_start(core);
        self.last = Sender::read(io, core);
        self.outbox = self.last;
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        let now = Sender::read(io, core);
        if now != self.last {
            self.last = now;
            if now.is_some() {
                self.outbox = now;
            }
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        self.outbox = None;
    }

    fn settings_changed(&mut self, core: &mut Core) {
        keep_channel_clean(core);
    }

    fn layout(&mut self, core: &mut Core, _assets: &crate::assets::Assets) {
        keep_channel_clean(core);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The Receiver (`NRcv`): gives what the Sender on its channel sent last,
/// of the kind it sent; off and 0 before anything comes and after the
/// Sender leaves.
#[derive(Debug, Default)]
pub struct Receiver {
    /// The link, as the host last told it.
    pub status: Status,
    /// What came in last (`None`: nothing, or the Sender left), put here by
    /// the host with [`Receiver::inbox_gen`] counted up.
    pub inbox: Option<NetValue>,
    pub inbox_gen: u32,
    /// The [`Receiver::inbox_gen`] the outputs give.
    seen_gen: u32,
    /// What the outputs give.
    value: Option<NetValue>,
}

impl Receiver {
    /// Takes what came in as what the outputs give, and tells the wire.
    fn take(&mut self, core: &mut Core, io: &mut Io<'_>) {
        self.seen_gen = self.inbox_gen;
        self.value = match self.inbox {
            // A picture the game doesn't have is no picture.
            Some(NetValue::Picture(n))
                if n != 0
                    && io
                        .assets
                        .get(crate::assets::Module::Gadget, crate::assets::rt::PICTURES, n)
                        .is_none() =>
            {
                Some(NetValue::Picture(0))
            }
            v => v,
        };
        core.state = false;
        core.number = 0.0;
        core.object = None;
        match self.value {
            Some(NetValue::Bool(b)) => {
                core.state = b;
                core.number = f64::from(u8::from(b));
            }
            Some(NetValue::Number(x)) => {
                core.state = x != 0.0 && !x.is_nan();
                core.number = x;
            }
            Some(NetValue::Picture(n)) => {
                core.state = n != 0;
                core.object = (n != 0).then_some(Object::Picture(n));
            }
            None => {}
        }
        io.fire(core, 0);
    }
}

impl Behavior for Receiver {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        draw_part(core, cx, self.status, RECEIVER_LCD);
    }

    fn start(&mut self, core: &mut Core, io: &mut Io<'_>) {
        io.base_start(core);
        self.take(core, io);
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        if self.inbox_gen != self.seen_gen {
            self.take(core, io);
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        self.value = None;
    }

    fn has_bool(&self, _core: &Core, _knot: usize) -> bool {
        matches!(self.value, None | Some(NetValue::Bool(_)))
    }

    fn has_number(&self, _core: &Core, _knot: usize) -> bool {
        matches!(self.value, None | Some(NetValue::Number(_)))
    }

    fn has_object(&self, _core: &Core, _knot: usize) -> bool {
        matches!(self.value, Some(NetValue::Picture(_)))
    }

    fn settings_changed(&mut self, core: &mut Core) {
        keep_channel_clean(core);
    }

    fn layout(&mut self, core: &mut Core, _assets: &crate::assets::Assets) {
        keep_channel_clean(core);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gadget::SoundData;
    use crate::gadgets::bench::{Bench, Source};
    use formats::tables::KnotKind;

    fn sender_bench() -> (Bench, Sender) {
        let mut b = Bench::with_knots(&[KnotKind::Input]);
        let mut s = Sender::default();
        b.fresh(&mut s);
        (b, s)
    }

    #[test]
    fn a_sender_sends_once_on_go_then_on_change() {
        let (mut b, mut s) = sender_bench();
        b.source(0, Source::number(42.0));
        b.start(&mut s);
        assert_eq!(s.outbox.take(), Some(NetValue::Number(42.0)));
        b.tick_at(&mut s, 0);
        b.tick_at(&mut s, 1);
        assert_eq!(s.outbox, None, "unchanged: nothing more");
        b.set_input(7.5);
        b.tick_at(&mut s, 2);
        assert_eq!(s.outbox.take(), Some(NetValue::Number(7.5)));
        b.set(0, Source::boolean(true));
        b.tick_at(&mut s, 3);
        assert_eq!(s.outbox.take(), Some(NetValue::Bool(true)));
        b.set(0, Source::object(Some(Object::Picture(1050))));
        b.tick_at(&mut s, 4);
        assert_eq!(s.outbox.take(), Some(NetValue::Picture(1050)));
        b.set(0, Source::object(None));
        b.tick_at(&mut s, 5);
        assert_eq!(s.outbox.take(), Some(NetValue::Picture(0)));
        // Stopping drops what wasn't sent; the input is still known.
        b.set_input(1.0);
        b.tick_at(&mut s, 6);
        s.reset(&mut b.core);
        assert_eq!((s.outbox, s.last), (None, Some(NetValue::Number(1.0))));
    }

    #[test]
    fn a_sender_sends_no_sound_nor_nothing() {
        let (mut b, mut s) = sender_bench();
        // Unwired.
        b.start(&mut s);
        b.tick_at(&mut s, 0);
        assert_eq!((s.outbox, s.last), (None, None));
        let sound = SoundData {
            rate: 11025,
            samples: vec![0x80; 10].into(),
        };
        b.set(0, Source::object(Some(Object::Sound(sound))));
        b.start(&mut s);
        b.tick_at(&mut s, 0);
        assert_eq!((s.outbox, s.last), (None, None));
        for x in [f64::NAN, f64::INFINITY] {
            b.set_input(x);
            b.tick_at(&mut s, 1);
            assert_eq!(s.outbox, None);
        }
        // A part that gives nothing on that knot.
        b.set(0, Source::default());
        b.tick_at(&mut s, 2);
        assert_eq!(s.outbox, None);
    }

    fn receiver_bench() -> (Bench, Receiver) {
        let mut b = Bench::with_knots(&[KnotKind::Output]);
        b.wire(0);
        let mut r = Receiver::default();
        b.fresh(&mut r);
        (b, r)
    }

    fn put(r: &mut Receiver, v: Option<NetValue>) {
        r.inbox = v;
        r.inbox_gen += 1;
    }

    #[test]
    fn a_receiver_gives_what_came_in() {
        let (mut b, mut r) = receiver_bench();
        b.start(&mut r);
        assert!(b.fired(0));
        assert_eq!(b.out(&r, 0), Value::Number(0.0));
        assert!(!r.bool_out(&b.core, 0));
        b.tick_at(&mut r, 0);
        assert!(!b.fired(0), "nothing new");
        put(&mut r, Some(NetValue::Number(42.0)));
        b.tick_at(&mut r, 1);
        assert!(b.fired(0));
        assert_eq!(b.out(&r, 0), Value::Number(42.0));
        assert!(!r.has_bool(&b.core, 0));
        put(&mut r, Some(NetValue::Bool(true)));
        b.tick_at(&mut r, 2);
        assert!(b.fired(0));
        assert_eq!(b.out(&r, 0), Value::Bool(true));
        // The Sender left: off, 0.
        put(&mut r, None);
        b.tick_at(&mut r, 3);
        assert!(b.fired(0));
        assert_eq!(b.out(&r, 0), Value::Number(0.0));
        // Stopped and started again: what came in last is given again.
        put(&mut r, Some(NetValue::Number(-3.0)));
        b.start(&mut r);
        assert!(b.fired(0));
        assert_eq!(b.out(&r, 0), Value::Number(-3.0));
        r.reset(&mut b.core);
        assert_eq!(b.out(&r, 0), Value::Number(0.0));
    }

    #[test]
    fn a_receiver_gives_only_pictures_the_game_has() {
        let files: Vec<(&'static str, &'static [u8])> = vec![("WWGADGET/1004/1050.bin", b"x")];
        let pack = crate::assets::Embedded::new(Box::leak(files.into_boxed_slice()));
        let (mut b, mut r) = receiver_bench();
        b.assets = crate::assets::Assets::load(None, Some(&pack)).unwrap();
        b.start(&mut r);
        put(&mut r, Some(NetValue::Picture(1050)));
        b.tick_at(&mut r, 0);
        assert_eq!(b.out(&r, 0), Value::Object(Some(Object::Picture(1050))));
        put(&mut r, Some(NetValue::Picture(4321)));
        b.tick_at(&mut r, 1);
        assert_eq!(b.out(&r, 0), Value::Object(None));
        put(&mut r, Some(NetValue::Picture(0)));
        b.tick_at(&mut r, 2);
        assert_eq!(b.out(&r, 0), Value::Object(None));
    }

    #[test]
    fn the_channel_is_kept_clean() {
        let mut core = Core::new(SENDER, 2900, 1900);
        core.settings.set_label(0, b"Cat Sun_Oak!");
        let mut s = Sender::default();
        s.settings_changed(&mut core);
        assert_eq!(core.settings.label(0), b"cat-sun-oak");
        assert_eq!(channel(&core), "cat-sun-oak");
        core.settings.set_label(0, b"  ");
        let mut r = Receiver::default();
        r.layout(&mut core, &crate::assets::Assets::default());
        assert_eq!(core.settings.label(0), b"");
        assert_eq!(role(SENDER), Some(Role::Send));
        assert_eq!(role(RECEIVER), Some(Role::Recv));
        assert_eq!(role(Tag::new(b"cnst")), None);
    }

    #[test]
    fn the_classes_are_found() {
        for (tag, id) in [(SENDER, 2900), (RECEIVER, 2901)] {
            let c = crate::gadgets::class(tag).unwrap();
            assert_eq!((c.tag, c.gadget_id), (tag, id));
        }
        // None of the game's has their tags or numbers.
        for c in crate::gadgets::CLASSES {
            assert!(
                CLASSES
                    .iter()
                    .all(|n| n.tag != c.tag && n.gadget_id != c.gadget_id && n.sheet != c.sheet)
            );
        }
    }

    #[test]
    fn long_names_take_two_lines() {
        // Six pixels a character.
        let m = |s: &str| i32::try_from(s.len()).unwrap() * 6;
        assert_eq!(lcd_lines("cat-sun-oak", 66, m), ["cat-sun-oak"]);
        assert_eq!(lcd_lines("swamp-quilt-zebra", 66, m), ["swamp-", "quilt-zebra"]);
        assert_eq!(lcd_lines("swamp-quilt-oak", 66, m), ["swamp-", "quilt-oak"]);
        assert_eq!(lcd_lines("cat-swamp-quilt", 66, m), ["cat-swamp-", "quilt"]);
        assert_eq!(lcd_lines("abcdefghijklmnopqrst", 66, m), ["abcdefghij", "klmnopqrst"]);
        assert_eq!(lcd_lines("abcdefghijklmnopqrst", 30, m), ["abcde", "klmno"]);
    }

    /// The parts draw inside their bounds, their names inside their
    /// displays, long ones too; with OpenWDATA, in every language.
    #[test]
    fn the_parts_draw_their_names_on_their_displays() {
        use crate::gfx::{IRect, Pictures, Surface};
        let names = [
            "cat-sun-oak",
            "swamp-quilt-zebra",
            "mmmmm-wwwww-mmmmm",
            "abcdefghijklmnopqrst",
            "kiwi42",
            "",
        ];
        let (size, at) = (200_i32, 50);
        for lang in crate::lang::Lang::ALL {
            let data = crate::opendata::builtin_for(lang);
            let packs = crate::assets::Packs {
                data: Some(&data),
                ..crate::assets::Packs::default()
            };
            let assets = crate::assets::Assets::load_with(None, packs, None).unwrap();
            let pictures = Pictures::default();
            for (tag, lcd) in [(SENDER, SENDER_LCD), (RECEIVER, RECEIVER_LCD)] {
                for (col, name) in names.iter().enumerate() {
                    let mut g = crate::doc::new_gadget(&assets, tag, 1, at, at);
                    g.core.settings.set_label(0, name.as_bytes());
                    let busy = col == 5 && tag == SENDER;
                    let status = Status {
                        online: col > 0,
                        peer: [
                            super::super::Peer::None,
                            super::super::Peer::Open,
                            super::super::Peer::Run,
                        ][col % 3],
                        busy,
                    };
                    if let Some(s) = g.behavior.as_any_mut().downcast_mut::<Sender>() {
                        s.status = status;
                    } else if let Some(r) = g.behavior.as_any_mut().downcast_mut::<Receiver>() {
                        r.status = status;
                    }
                    let r = g.core.rect();
                    assert_eq!((r.x1 - r.x0, r.y1 - r.y0), (88, 50));
                    // Drawn on two backgrounds, and without the text: a
                    // pixel either changed is drawn.
                    let draw = |bg: u8, text: bool| {
                        let mut s = Surface::new(size.cast_unsigned(), size.cast_unsigned());
                        s.fill(IRect::new(0, 0, size, size), bg);
                        let mut cx = DrawCx {
                            surface: &mut s,
                            assets: &assets,
                            pictures: &pictures,
                            running: false,
                            ticks: 0,
                            lights: false,
                        };
                        if text {
                            g.behavior.draw(&g.core, &mut cx);
                        } else {
                            cx.frame_at_rect(g.core.sheet, status.lamp(), at, at);
                        }
                        s
                    };
                    let (a, b, bare) = (draw(3, true), draw(200, true), draw(3, false));
                    let mut text = 0;
                    for i in 0..a.pixels.len() {
                        let x = i32::try_from(i).unwrap() % size;
                        let y = i32::try_from(i).unwrap() / size;
                        if a.pixels[i] != 3 || b.pixels[i] != 200 {
                            assert!(r.contains(x, y), "{tag} {name}: {x}, {y} outside {r:?}");
                        }
                        if a.pixels[i] != bare.pixels[i] {
                            text += 1;
                            let (lx, ly, lw, lh) = lcd;
                            let (x, y) = (x - at, y - at);
                            assert!(
                                lx < x && x < lx + lw && ly < y && y < ly + lh,
                                "{tag} {name}: text at {x}, {y} outside the display"
                            );
                        }
                    }
                    assert!(text > 10, "{tag} {name:?}: no text");
                }
            }
        }
    }
}
