//! Example widgets for the network parts: prime testers. A Receiver takes
//! a number N; a Sender answers 1 if N is prime, 0 if it isn't and -1
//! while that isn't known yet. Between them only the original game's
//! parts. Built here, run against a relay in memory and checked, and
//! written to `examples/net/` (`cargo test -p engine -- --ignored
//! write_prime_examples`).

use super::*;
use crate::assets::Packs;
use crate::gadget::{Core, Gadget, Tag};
use crate::lang::Lang;
use crate::net::NetValue;
use crate::net::parts::{RECEIVER, Receiver, SENDER, Sender};
use crate::sim::Env;

/// An output or an input: a part (by creation) and its knot.
type Knot = (usize, usize);

/// Builds a widget part by part. The parts tick in the order they are
/// made (the document ticks its list from the end, so the list is that
/// order backwards): a part made before another sees its outputs a tick
/// late, one made after sees them the same tick.
struct Builder<'a> {
    assets: &'a Assets,
    parts: Vec<Gadget>,
    wires: Vec<(Knot, Knot)>,
    /// Each part's band, and each band's widest row.
    band_of: Vec<usize>,
    bands: Vec<i32>,
}

/// The work area the parts are laid out in: Fit Work Area's biggest.
const WIDTH: i32 = 1280;
const MARGIN: i32 = 6;
const GAP: i32 = 5;
/// The toolbars cover the work area's corner: the title goes right of
/// them, the parts below.
const TOOLBARS: (i32, i32) = (512, 112);

impl<'a> Builder<'a> {
    fn new(assets: &'a Assets) -> Builder<'a> {
        Builder {
            assets,
            parts: Vec::new(),
            wires: Vec::new(),
            band_of: Vec::new(),
            bands: vec![WIDTH - TOOLBARS.0 - MARGIN],
        }
    }

    /// A new band of parts, `title` first, its rows at most `width` wide.
    /// The bands are packed side by side where they fit ([`Builder::finish`]).
    fn band(&mut self, title: &str, width: i32) {
        self.bands.push(width);
        self.label(title, false);
    }

    /// A new part of class `tag`, set by `set`, in the last band.
    fn part(&mut self, tag: [u8; 4], set: impl FnOnce(&mut Core)) -> usize {
        let mut g = doc::new_gadget(self.assets, Tag(tag), 0, 0, 0);
        set(&mut g.core);
        g.behavior.settings_changed(&mut g.core);
        g.behavior.layout(&mut g.core, self.assets);
        self.parts.push(g);
        self.band_of.push(self.bands.len() - 1);
        self.parts.len() - 1
    }

    /// Lays the parts out: each band's in rows, left to right in the order
    /// made, then the bands on shelves, the first right of the toolbars.
    fn lay_out(&mut self) {
        let mut place = (MARGIN, TOOLBARS.1, 0);
        for band in 0..self.bands.len() {
            let max = self.bands[band];
            // The band's own rows, from 0, 0.
            let (mut x, mut y, mut row_h, mut w) = (0, 0, 0, 0);
            let mut at = Vec::new();
            for (i, g) in self.parts.iter().enumerate().filter(|(i, _)| self.band_of[*i] == band) {
                let r = g.core.rect();
                let (gw, gh) = (r.x1 - r.x0, r.y1 - r.y0);
                if x + gw > max && x > 0 {
                    (x, y, row_h) = (0, y + row_h + GAP, 0);
                }
                at.push((i, x - r.x0, y - r.y0));
                x += gw + GAP;
                w = w.max(x - GAP);
                row_h = row_h.max(gh);
            }
            let h = y + row_h;
            let origin = if band == 0 {
                (TOOLBARS.0, MARGIN)
            } else {
                if place.0 + w > WIDTH - MARGIN && place.0 > MARGIN {
                    place = (MARGIN, place.1 + place.2 + GAP + 2, 0);
                }
                let o = (place.0, place.1);
                place = (place.0 + w + 4 * GAP, place.1, place.2.max(h));
                o
            };
            for (i, dx, dy) in at {
                let g = &mut self.parts[i];
                g.core.h += origin.0 + dx;
                g.core.v += origin.1 + dy;
                g.behavior.layout(&mut g.core, self.assets);
            }
        }
    }

    fn wire(&mut self, from: Knot, to: Knot) {
        self.wires.push((from, to));
    }

    /// A Mini Number of `n`.
    fn number(&mut self, n: u32) -> Knot {
        (
            self.part(*b"MiCo", |c| c.settings.set_label(0, n.to_string().as_bytes())),
            0,
        )
    }

    /// A Comparer of `mode` (0 <, 1 >, 2 =, 3 ≠, 4 ≤, 5 ≥) of `a` and `b`.
    fn compare(&mut self, mode: u8, a: Option<Knot>, b: Option<Knot>) -> Knot {
        let p = self.part(*b"Comp", |c| c.settings.radios[0] = mode);
        if let Some(a) = a {
            self.wire(a, (p, 0));
        }
        if let Some(b) = b {
            self.wire(b, (p, 1));
        }
        (p, 2)
    }

    /// A gate (`AndG`, `OrGa`) of `a` and `b`.
    fn gate(&mut self, tag: [u8; 4], a: Knot, b: Knot) -> Knot {
        let p = self.part(tag, |_| {});
        self.wire(a, (p, 0));
        self.wire(b, (p, 1));
        (p, 2)
    }

    /// A Switcher: `off` while `sel` is off, `on` while it's on.
    fn switch(&mut self, sel: Option<Knot>, off: Knot, on: Option<Knot>) -> usize {
        let p = self.part(*b"Mplx", |_| {});
        if let Some(sel) = sel {
            self.wire(sel, (p, 0));
        }
        self.wire(off, (p, 1));
        if let Some(on) = on {
            self.wire(on, (p, 2));
        }
        p
    }

    /// A Splitter of `n` (2 to 4) ways fed by `from`; its outputs.
    fn split(&mut self, from: Knot, n: usize) -> Vec<Knot> {
        let tag = match n {
            0..=2 => b"Spl2",
            3 => b"Spl3",
            _ => b"Spl4",
        };
        let p = self.part(*tag, |_| {});
        self.wire(from, (p, 0));
        (1..=n.max(2)).map(|k| (p, k)).collect()
    }

    /// A number cruncher (`Addr`, `Divr`...) of `a` and `b`.
    fn crunch(&mut self, tag: [u8; 4], a: Option<Knot>, b: Option<Knot>) -> Knot {
        let p = self.part(tag, |_| {});
        if let Some(a) = a {
            self.wire(a, (p, 0));
        }
        if let Some(b) = b {
            self.wire(b, (p, 1));
        }
        (p, 2)
    }

    /// Math Functions' Int of `x`.
    fn int(&mut self, x: Knot) -> Knot {
        let p = self.part(*b"1ncr", |c| c.settings.numbers[0] = 6);
        self.wire(x, (p, 0));
        (p, 1)
    }

    /// A Label.
    fn label(&mut self, text: &str, big: bool) {
        self.part(*b"TxLb", |c| {
            c.settings.set_label(0, &crate::text::encode(text));
            c.settings.radios[0] = u8::from(big);
        });
    }

    /// The widget: the list in tick order backwards, ids in list order.
    fn finish(mut self) -> Document {
        self.lay_out();
        let n = self.parts.len();
        let mut doc = Document::new();
        for mut g in self.parts.into_iter().rev() {
            g.core.id = doc.new_id();
            doc.gadgets.push(g);
        }
        for ((a, ak), (b, bk)) in self.wires {
            doc.connect(n - 1 - a, ak, n - 1 - b, bk, Vec::new(), 0)
                .expect("a wire");
        }
        doc
    }
}

/// One prime tester: the divisors it tries.
struct Variant {
    /// The file under `examples/net/`.
    file: &'static str,
    /// The Sender's channel.
    channel: &'static str,
    title: &'static str,
    /// Divisors tried once.
    fixed: &'static [u32],
    /// Chains of divisors: first and step.
    chains: &'static [(u32, u32)],
}

/// The channel all of them listen on for N.
const ASK: &str = "prime-quiz-ask";

const VARIANTS: &[Variant] = &[
    Variant {
        file: "prime-1-simple.wgt",
        channel: "prime-quiz-easy",
        title: "Prime or not?",
        fixed: &[],
        chains: &[(2, 1)],
    },
    Variant {
        file: "prime-2-odd.wgt",
        channel: "prime-quiz-odd",
        title: "Prime or not? Odd divisors",
        fixed: &[2],
        chains: &[(3, 2)],
    },
    Variant {
        file: "prime-3-wheel.wgt",
        channel: "prime-quiz-quick",
        title: "Prime or not? 6k-1 and 6k+1",
        fixed: &[2, 3],
        chains: &[(5, 6), (7, 6)],
    },
    Variant {
        file: "prime-4-parallel.wgt",
        channel: "prime-quiz-swift",
        title: "Prime or not? Four at a time",
        fixed: &[2, 3],
        chains: &[(5, 12), (7, 12), (11, 12), (13, 12)],
    },
];

/// The seconds a new N waits for the parts to settle before the answer
/// counts.
const SETTLE_SECONDS: u32 = 3;

/// How wide a chain of divisors' band is: two side by side.
const CHAIN_W: i32 = 620;

/// `n` ways from `from`: a tree of splitters where one isn't enough.
fn fan(b: &mut Builder<'_>, from: Knot, n: usize) -> Vec<Knot> {
    if n <= 1 {
        return vec![from];
    }
    if n <= 4 {
        return b.split(from, n);
    }
    let outs = b.split(from, 4);
    let mut left = n;
    let mut all = Vec::new();
    for (i, o) in outs.into_iter().enumerate() {
        let take = left.div_ceil(4 - i);
        left -= take;
        all.extend(fan(b, o, take));
    }
    all
}

/// A tree of gates `tag` over `ins`.
fn tree(b: &mut Builder<'_>, tag: [u8; 4], ins: &[Knot]) -> Knot {
    if let [one] = ins {
        return *one;
    }
    let (l, r) = ins.split_at(ins.len() / 2);
    let l = tree(b, tag, l);
    let r = tree(b, tag, r);
    b.gate(tag, l, r)
}

#[allow(clippy::too_many_lines)]
fn build(assets: &Assets, v: &Variant) -> Document {
    let mut b = Builder::new(assets);
    let chains = v.chains.len();
    let fixed = v.fixed.len();
    b.label(v.title, true);
    b.label(&format!("Ask on: {ASK}"), false);
    b.label(&format!("Answer on: {}", v.channel), false);
    b.label("1 prime, 0 not, -1 not yet", false);
    b.band("N comes in", WIDTH);
    // N comes in. A copy a tick late tells when it changes.
    let rcv = b.part(*b"NRcv", |c| c.settings.set_label(0, ASK.as_bytes()));
    let late_n = 3 + chains + fixed;
    let lag = b.part(*b"Spl4", |_| {});
    let n_now = b.split((rcv, 0), 2);
    b.wire(n_now[1], (lag, 0));
    let changed = b.compare(3, Some(n_now[0]), Some((lag, 1)));
    let mut n_late: Vec<Knot> = (2..=4).map(|k| (lag, k)).collect();
    if late_n > 3 {
        let more = fan(&mut b, n_late.pop().unwrap(), late_n - 2);
        n_late.extend(more);
    }
    // The Stopwatch counts the seconds since N changed; it stops when
    // the answer is known.
    b.band("Wait for the parts to settle", WIDTH);
    let watch = b.part(*b"Wtch", |_| {});
    b.wire(changed, (watch, 2));
    let secs = b.number(SETTLE_SECONDS);
    let settled = b.compare(5, Some((watch, 3)), Some(secs));
    let settled = fan(&mut b, settled, 4);
    // Is N a whole number from 2 up?
    b.band("Is N whole and 2 or more?", CHAIN_W);
    let whole = b.int(n_late[0]);
    let is_whole = b.compare(2, Some(whole), Some(n_late[1]));
    let two = b.number(2);
    let big = b.compare(5, Some(n_late[2]), Some(two));
    let valid = b.gate(*b"AndG", is_whole, big);
    // Running: settled and not done.
    let run = b.compare(1, Some(settled[0]), None);
    let run_to = fan(&mut b, run, chains);
    let mut found = Vec::new();
    let mut prime = Vec::new();
    for (i, &(first, step)) in v.chains.iter().enumerate() {
        b.band(
            &format!("Try {first}, {}, {}...", first + step, first + 2 * step),
            CHAIN_W,
        );
        // The divisor d: the first while not running, then d + step.
        let start = b.number(first);
        let dm = b.switch(Some(run_to[i]), start, None);
        let d = b.split((dm, 3), 3);
        let step = b.number(step);
        let next = b.crunch(*b"Addr", Some(d[0]), Some(step));
        b.wire(next, (dm, 2));
        // q = N / d; f = Int(q); q again, as late as f.
        let q = b.crunch(*b"Divr", Some(n_late[3 + i]), Some(d[1]));
        let q = b.split(q, 2);
        let f = b.int(q[0]);
        let q2 = b.crunch(*b"Addr", Some(q[1]), None);
        let f = b.split(f, 2);
        let q2 = b.split(q2, 2);
        // d divides N if q is whole, unless q is 1 (d is N).
        let eq = b.compare(2, Some(q2[0]), Some(f[0]));
        let two = b.number(2);
        let ge = b.compare(5, Some(q2[1]), Some(two));
        found.push(b.gate(*b"AndG", eq, ge));
        // Past the square root: Int(N / d) < d + 2 steps.
        prime.push(b.compare(0, Some(f[1]), Some(d[2])));
    }
    if fixed > 0 {
        let list: Vec<String> = v.fixed.iter().map(u32::to_string).collect();
        b.band(&format!("Does {} divide N?", list.join(" or ")), CHAIN_W);
    }
    for (i, &c) in v.fixed.iter().enumerate() {
        let c = b.number(c);
        let q = b.crunch(*b"Divr", Some(n_late[3 + chains + i]), Some(c));
        let q = b.split(q, 3);
        let f = b.int(q[0]);
        let eq = b.compare(2, Some(q[1]), Some(f));
        let two = b.number(2);
        let ge = b.compare(5, Some(q[2]), Some(two));
        found.push(b.gate(*b"AndG", eq, ge));
    }
    // Latches: found a divisor; past the root. Cleared while settling.
    b.band("Remember until the next N", CHAIN_W);
    let any_found = tree(&mut b, *b"OrGa", &found);
    let all_prime = tree(&mut b, *b"AndG", &prime);
    let fl_or = b.part(*b"OrGa", |_| {});
    b.wire(any_found, (fl_or, 0));
    let fl_and = b.gate(*b"AndG", settled[1], (fl_or, 2));
    let fl = b.split(fl_and, 3);
    b.wire(fl[0], (fl_or, 1));
    let pl_or = b.part(*b"OrGa", |_| {});
    b.wire(all_prime, (pl_or, 0));
    let pl_and = b.gate(*b"AndG", settled[2], (pl_or, 2));
    let pl = b.split(pl_and, 3);
    b.wire(pl[0], (pl_or, 1));
    let done = b.gate(*b"OrGa", fl[1], pl[1]);
    let done = b.split(done, 3);
    b.wire(done[0], (run.0, 1));
    b.wire(done[1], (watch, 1));
    let not = b.part(*b"NotG", |_| {});
    b.wire(done[2], (not, 0));
    b.wire((not, 1), (watch, 0));
    // The answer.
    b.band("The answer", CHAIN_W);
    let minus = b.part(*b"Sldr", |c| {
        c.settings.radios[0] = 1;
        c.settings.set_label(0, b"-1");
        c.settings.set_label(1, b"-1");
    });
    let minus = b.split((minus, 0), 2);
    let one = b.number(1);
    let m1 = b.switch(Some(pl[2]), minus[0], Some(one));
    let zero = b.number(0);
    let m2 = b.switch(Some(fl[2]), (m1, 3), Some(zero));
    let zero = b.number(0);
    let m3 = b.switch(Some(valid), zero, Some((m2, 3)));
    let m4 = b.switch(Some(settled[3]), minus[1], Some((m3, 3)));
    let snd = b.part(*b"NSnd", |c| c.settings.set_label(0, v.channel.as_bytes()));
    b.wire((m4, 3), (snd, 0));
    b.finish()
}

/// The widget that asks: a Numeric Keypad's number to all of them, and
/// their answers on Ten-Digit Displays.
const ASK_FILE: &str = "prime-ask.wgt";

fn build_ask(assets: &Assets) -> Document {
    let mut b = Builder::new(assets);
    b.label("Is it a prime? Ask them", true);
    b.label("Type a number, then ENTER", false);
    b.label("1 prime, 0 not, -1 not yet", false);
    b.band("The number", WIDTH);
    let pad = b.part(*b"NmKP", |_| {});
    let snd = b.part(*b"NSnd", |c| c.settings.set_label(0, ASK.as_bytes()));
    b.wire((pad, 0), (snd, 0));
    for v in VARIANTS {
        b.band(v.file.trim_end_matches(".wgt"), CHAIN_W);
        let rcv = b.part(*b"NRcv", |c| c.settings.set_label(0, v.channel.as_bytes()));
        let show = b.part(*b"di10", |_| {});
        b.wire((rcv, 0), (show, 0));
    }
    b.finish()
}

fn assets() -> Assets {
    let graphics = crate::opengfx::builtin_for(Lang::En);
    let sounds = crate::opensfx::builtin();
    let data = crate::opendata::builtin_for(Lang::En);
    let packs = Packs {
        graphics: Some(graphics.as_ref()),
        sounds: Some(&sounds),
        data: Some(&data),
    };
    Assets::load_with(None, packs, None).unwrap()
}

/// A widget running, N put into its Receiver as the hub would.
struct Run<'a> {
    assets: &'a Assets,
    doc: Document,
    env: Env,
    rcv: usize,
    snd: usize,
}

impl<'a> Run<'a> {
    fn new(assets: &'a Assets, mut doc: Document) -> Run<'a> {
        let find = |doc: &Document, tag| doc.gadgets.iter().position(|g| g.core.tag == tag).unwrap();
        let (rcv, snd) = (find(&doc, RECEIVER), find(&doc, SENDER));
        let mut env = Env::default();
        doc.start(assets, &mut env);
        Run {
            assets,
            doc,
            env,
            rcv,
            snd,
        }
    }

    fn put(&mut self, n: f64) {
        let r = self.doc.gadgets[self.rcv]
            .behavior
            .as_any_mut()
            .downcast_mut::<Receiver>()
            .unwrap();
        r.inbox = Some(NetValue::Number(n));
        r.inbox_gen += 1;
    }

    fn tick(&mut self) {
        self.doc.tick(self.assets, &mut self.env);
        self.env.ticks += 1;
    }

    fn out(&self) -> Option<NetValue> {
        self.doc.gadgets[self.snd]
            .behavior
            .as_any()
            .downcast_ref::<Sender>()
            .unwrap()
            .last
    }

    /// Puts `n` in and ticks until the answer, then `after` more; the
    /// values the Sender had, in order, and the ticks to the answer.
    #[allow(clippy::float_cmp)]
    fn ask(&mut self, n: f64, after: u32) -> (Vec<f64>, u32) {
        self.put(n);
        let mut seen: Vec<f64> = Vec::new();
        let mut t = 0;
        let mut answered = None;
        loop {
            self.tick();
            t += 1;
            let v = match self.out() {
                Some(NetValue::Number(x)) => x,
                other => panic!("N = {n}: {other:?}"),
            };
            if seen.last() != Some(&v) {
                seen.push(v);
            }
            if answered.is_none() && v != -1.0 {
                answered = Some(t);
            }
            if let Some(a) = answered
                && t >= a + after
            {
                return (seen, a);
            }
            assert!(t < 10_000_000, "N = {n}: no answer");
        }
    }
}

fn is_prime(n: f64) -> bool {
    if n.fract() != 0.0 || n < 2.0 {
        return false;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = n as u64;
    (2..=n.isqrt()).all(|d| !n.is_multiple_of(d))
}

/// Where the widgets are kept: `examples/net/` in the repository.
fn example(file: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/net")
        .join(file)
}

/// Each widget as saved and opened again, in its file, answers.
#[test]
fn prime_testers_answer() {
    let assets = assets();
    for v in VARIANTS {
        let bytes = build(&assets, v).save();
        let file = std::fs::read(example(v.file)).unwrap_or_default();
        assert!(
            file == bytes,
            "examples/net/{} is out of date: cargo test -p engine -- --ignored write_prime_examples",
            v.file
        );
        let mut run = Run::new(&assets, Document::load(&assets, &bytes).unwrap());
        for n in [
            0.0, 1.0, 2.0, 3.0, 4.0, 9.0, 17.0, 25.0, 97.0, 91.0, 7919.0, 7920.0, 2.5, -7.0,
        ] {
            let (seen, _) = run.ask(n, 300);
            let want = if is_prime(n) { 1.0 } else { 0.0 };
            assert_eq!(seen, [-1.0, want], "{} N = {n}", v.file);
        }
    }
}

#[test]
#[ignore = "writes the widgets to examples/net/"]
fn write_prime_examples() {
    let assets = assets();
    for v in VARIANTS {
        let path = example(v.file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, build(&assets, v).save()).unwrap();
    }
    std::fs::write(example(ASK_FILE), build_ask(&assets).save()).unwrap();
}

/// The asking widget, as saved, sends what's typed on its keypad and
/// shows what its Receivers get.
#[test]
fn prime_ask_widget() {
    let assets = assets();
    let bytes = build_ask(&assets).save();
    assert!(
        std::fs::read(example(ASK_FILE)).unwrap_or_default() == bytes,
        "examples/net/{ASK_FILE} is out of date: cargo test -p engine -- --ignored write_prime_examples"
    );
    let doc = Document::load(&assets, &bytes).unwrap();
    let count = |tag| doc.gadgets.iter().filter(|g| g.core.tag == tag).count();
    assert_eq!(
        (count(SENDER), count(RECEIVER), count(Tag(*b"di10"))),
        (1, VARIANTS.len(), VARIANTS.len())
    );
    for w in &doc.wires {
        let (from, to) = (&doc.gadgets[w.from].core, &doc.gadgets[w.to].core);
        assert!(
            (from.tag, to.tag) == (Tag(*b"NmKP"), SENDER) || (from.tag, to.tag) == (RECEIVER, Tag(*b"di10")),
            "{:?} -> {:?}",
            from.tag,
            to.tag
        );
    }
}

#[test]
#[ignore = "slow: every N up to 3000 and interruptions, in each variant"]
fn prime_testers_stress() {
    let assets = assets();
    for v in VARIANTS {
        let mut run = Run::new(&assets, build(&assets, v));
        let mut worst = (0, 0.0);
        for n in 0..=3000 {
            let n = f64::from(n);
            let (seen, ticks) = run.ask(n, 200);
            let want = if is_prime(n) { 1.0 } else { 0.0 };
            assert_eq!(seen, [-1.0, want], "{} N = {n}", v.file);
            if ticks > worst.0 {
                worst = (ticks, n);
            }
        }
        eprintln!("{}: slowest up to 3000: N = {} in {} ticks", v.file, worst.1, worst.0);
        // Interrupted after k ticks by another N: only -1 before, then
        // the new answer.
        let mut k = 0;
        for (a, b) in [(7919.0, 7907.0), (104_729.0, 49.0), (1009.0, 1013.0), (961.0, 841.0)] {
            for _ in 0..40 {
                k = (k * 37 + 11) % 997;
                run.ask(4.0, 0);
                run.put(a);
                for _ in 0..k {
                    run.tick();
                    let v = run.out();
                    assert!(
                        v == Some(NetValue::Number(-1.0))
                            || v == Some(NetValue::Number(if is_prime(a) { 1.0 } else { 0.0 })),
                        "{v:?}"
                    );
                }
                let (seen, _) = run.ask(b, 100);
                let want = if is_prime(b) { 1.0 } else { 0.0 };
                assert_eq!(seen, [-1.0, want], "{} {a} then {b} after {k}", v.file);
            }
        }
    }
}

#[test]
#[ignore = "slow: the ticks to an answer for big N"]
fn prime_testers_timing() {
    let assets = assets();
    for v in VARIANTS {
        let doc = build(&assets, v);
        let mut count = std::collections::BTreeMap::new();
        for g in &doc.gadgets {
            *count.entry(g.core.tag.to_string()).or_insert(0) += 1;
        }
        eprintln!("{}: {} parts {count:?}", v.file, doc.gadgets.len());
        let mut run = Run::new(&assets, doc);
        for n in [97.0, 7919.0, 104_729.0, 999_983.0, 1_018_081.0, 62_615_533.0] {
            let (seen, ticks) = run.ask(n, 0);
            assert_eq!(seen, [-1.0, if is_prime(n) { 1.0 } else { 0.0 }]);
            eprintln!("  N = {n}: {ticks} ticks ({:.1} s)", f64::from(ticks) / 60.0);
        }
    }
}

#[test]
#[ignore = "writes pictures of the widgets to PRIME_LOOK"]
fn prime_testers_look() {
    let Some(dir) = std::env::var_os("PRIME_LOOK") else {
        return;
    };
    let assets = assets();
    let docs = VARIANTS
        .iter()
        .map(|v| (v.file, build(&assets, v)))
        .chain([(ASK_FILE, build_ask(&assets))]);
    for (file, doc) in docs {
        let mut e = Editor::new(self::assets(), doc, 1280, 1200);
        e.draw();
        let pal = e.assets.palette();
        let s = &e.surface;
        let mut out = format!("P6 {} {} 255\n", s.width, s.height).into_bytes();
        for &i in &s.pixels {
            out.extend_from_slice(&pal[usize::from(i)]);
        }
        std::fs::write(std::path::Path::new(&dir).join(file).with_extension("ppm"), out).unwrap();
    }
}
