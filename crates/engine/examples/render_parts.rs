//! Draws every part of the tray, fresh as dragged out of it, into a PNG
//! each (`OUT/<tray index>.png`, transparent around it) with the port's
//! own graphics (`engine::opengfx`), and its knots' places in it
//! (`OUT/<tray index>.txt`: x, y, 1 for an output), from an export of the CD:
//! `cargo run -p engine --example render_parts -- EXPORT OUT`. The tray's
//! `OpenWGFX` icons and the puzzles' pictures are made from them
//! (`tools/opengfx/sheets/tray.py`, `puzzles.py`).

use std::fmt::Write as _;

use engine::assets::{Assets, Dir, Source, rt};
use engine::gadget::DrawCx;
use engine::gfx::{IRect, Pictures, Surface};

/// The export without its pictures, so that the pack's are drawn.
struct NoPictures(Dir);

impl Source for NoPictures {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0.read(path)
    }

    fn list(&self, path: &str) -> Vec<String> {
        let mut names = self.0.list(path);
        if path.eq_ignore_ascii_case("WWGADGET") {
            names.retain(|n| *n != rt::PICTURES.to_string());
        }
        names
    }
}

/// The part drawn on `background`, the box around it and its knots, and
/// its knots (where, and whether outputs).
fn draw(assets: &Assets, tag: engine::gadget::Tag, background: u8) -> (Surface, IRect, Vec<(i32, i32, bool)>) {
    const M: i32 = 40;
    let g = engine::doc::new_gadget(assets, tag, 1, M, M);
    // The part and its knots.
    let r = (0..g.core.knots.len())
        .filter_map(|k| g.core.knot_at(k))
        .fold(g.core.rect(), |r, (x, y)| {
            r.union(IRect::new(x - 4, y - 4, x + 5, y + 5))
        });
    let mut s = Surface::new(u32::try_from(r.x1 + M).unwrap(), u32::try_from(r.y1 + M).unwrap());
    s.fill(s.bounds(), background);
    let pictures = Pictures::default();
    let mut cx = DrawCx {
        surface: &mut s,
        assets,
        pictures: &pictures,
        running: false,
        ticks: 0,
        lights: true,
    };
    g.behavior.draw(&g.core, &mut cx);
    let knots = (0..g.core.knots.len())
        .filter_map(|k| {
            let (x, y) = g.core.knot_at(k)?;
            Some((x, y, g.core.knots[k].def.kind == formats::tables::KnotKind::Output))
        })
        .collect();
    (s, r, knots)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let export = NoPictures(Dir(args[1].clone().into()));
    let out = std::path::PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).unwrap();
    let graphics = engine::opengfx::builtin();
    let packs = engine::assets::Packs {
        graphics: Some(&graphics),
        sounds: None,
        data: None,
    };
    let assets = Assets::load_with(None, packs, Some(&export)).unwrap();
    let pal = assets.palette();
    for i in 1..=64u8 {
        let Some(tag) = engine::editor::tray::tag_of(i) else {
            continue;
        };
        // Drawn on two backgrounds: what differs is the background.
        let (a, r, knots) = draw(&assets, tag, 0);
        let (b, _, _) = draw(&assets, tag, 0xff);
        let r = r.intersect(a.bounds());
        let (w, h) = (r.width().max(1), r.height().max(1));
        let mut pixels = Vec::new();
        for y in r.y0..r.y0 + h {
            for x in r.x0..r.x0 + w {
                let (p, q) = (a.get(x, y).unwrap_or(0), b.get(x, y).unwrap_or(0xff));
                pixels.push(if p == q {
                    let c = pal[usize::from(p)];
                    [c[0], c[1], c[2], 0xff]
                } else {
                    [0; 4]
                });
            }
        }
        let png = formats::png::encode(&formats::png::Rgba {
            width: w.unsigned_abs(),
            height: h.unsigned_abs(),
            pixels,
        });
        std::fs::write(out.join(format!("{i}.png")), png).unwrap();
        // Its knots in the picture: x, y, and 1 for an output.
        let mut lines = String::new();
        for (x, y, o) in &knots {
            let _ = writeln!(lines, "{} {} {}", x - r.x0, y - r.y0, u8::from(*o));
        }
        std::fs::write(out.join(format!("{i}.txt")), lines).unwrap();
    }
    for p in &assets.pack_problems {
        eprintln!("opengfx: {p}");
    }
}
