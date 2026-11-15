//! Draws a saved widget's gadgets into a PPM picture:
//! `cargo run -p engine --example render_widget -- CD_DIR WIDGET.WGT OUT.ppm [--opengfx]`
//! (with the port's own graphics, `engine::opengfx`).

use std::io::Write;

use engine::assets::{Assets, Dir};
use engine::doc::Document;
use engine::gadget::DrawCx;
use engine::gfx::{Pictures, Surface};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cd = Dir(args[1].clone().into());
    let graphics = engine::opengfx::builtin();
    let own = args.iter().any(|a| a == "--opengfx");
    let packs = engine::assets::Packs {
        graphics: own.then_some(&graphics as _),
        sounds: None,
        data: None,
    };
    let assets = Assets::load_with(Some(&cd), packs, None).unwrap();
    let bytes = std::fs::read(&args[2]).unwrap();
    let doc = Document::load(&assets, &bytes).unwrap();
    let ext = doc.extent();
    let w = u32::try_from(ext.x1.max(1) + 10).unwrap();
    let h = u32::try_from(ext.y1.max(1) + 10).unwrap();
    let mut s = Surface::new(w, h);
    s.fill(s.bounds(), 0xff);
    let pictures = Pictures::default();
    let mut cx = DrawCx {
        surface: &mut s,
        assets: &assets,
        pictures: &pictures,
        running: false,
        ticks: 0,
        lights: true,
    };
    doc.draw_gadgets(&mut cx);
    for wire in &doc.wires {
        let a = doc.gadgets[wire.from].core.knot_at(wire.from_knot).unwrap();
        let b = doc.gadgets[wire.to].core.knot_at(wire.to_knot).unwrap();
        let mut at = a;
        for &p in wire.points.iter().chain(std::iter::once(&b)) {
            s.line(at, p, 0);
            at = p;
        }
    }
    let pal = assets.palette();
    let mut f = std::fs::File::create(&args[3]).unwrap();
    write!(f, "P6\n{w} {h}\n255\n").unwrap();
    for p in &s.pixels {
        f.write_all(&pal[usize::from(*p)]).unwrap();
    }
    println!(
        "{} gadgets, {} wires, extent {ext:?}",
        doc.gadgets.len(),
        doc.wires.len()
    );
}
