//! Plays a script of mouse and key events into the editor, with no window,
//! and writes a PNG of the screen (the game's pointer drawn on it) at
//! each `shot`: for comparing the port with the original running the same
//! script (`tools/compare/`).
//!
//! `cargo run -p engine --example script -- CD_DIR SCRIPT OUT_DIR [--opengfx] [--enhanced]`
//! (`-` for `CD_DIR`: no CD, the port's own packs alone)
//!
//! The script, a command a line (`#` starts a comment), in the editor's
//! coordinates (the menu bar's top left is 0, 0):
//!
//! ```text
//! size W H              the editor's size (default 640 433, the original's
//!                       client area maximized at 640×480)
//! move X Y              the mouse there
//! down X Y [right]      a button pressed there
//! up X Y [right]        released
//! click X Y [right]     pressed and released
//! dclick X Y            a double click
//! drag X0 Y0 X1 Y1      pressed at the first point, moved, released at the second
//! key VK                a key pressed and released (a Windows virtual-key code,
//!                       decimal or 0x..)
//! wait MS               time passes
//! shot NAME             OUT_DIR/NAME.png
//! open FILE.WGT         opens a widget (as dropped on the window)
//! puzzle N              opens puzzle N (as File ▸ Load Puzzle does)
//! dump                  prints the parts: tag, box and knots, in these
//!                       coordinates (to write the next lines with)
//! ```

use std::path::PathBuf;

use engine::assets::{Assets, Dir, Packs};
use engine::doc::Document;
use engine::editor::{Button, Editor, Pointer};

/// Milliseconds per update, as a window's frames come.
const FRAME_MS: u64 = 16;

struct Run {
    editor: Editor,
    palette: [[u8; 3]; 256],
    mouse: (i32, i32),
    out: PathBuf,
}

impl Run {
    fn wait(&mut self, ms: u64) {
        let mut left = ms;
        while left > 0 {
            let step = left.min(FRAME_MS);
            self.editor.update(step, (12, 0, 0));
            left -= step;
        }
        self.editor.draw();
    }

    fn to(&mut self, x: i32, y: i32) {
        // In steps, as a real mouse moves (drags see the way).
        let (x0, y0) = self.mouse;
        let n = ((x - x0).abs().max((y - y0).abs()) + 7) / 8;
        let n = n.max(1);
        for i in 1..=n {
            self.editor.mouse_move(x0 + (x - x0) * i / n, y0 + (y - y0) * i / n);
            self.wait(FRAME_MS);
        }
        self.mouse = (x, y);
    }

    /// Prints the parts: tag, box and knots, in the editor's coordinates
    /// (the worksheet starts under the menu bar).
    fn dump(&self) {
        let bar = engine::editor::menu::BAR_H;
        for (i, g) in self.editor.doc.gadgets.iter().enumerate() {
            let r = g.core.rect();
            let knots: Vec<String> = (0..g.core.knots.len())
                .filter_map(|k| g.core.knot_at(k))
                .map(|(x, y)| format!("({x},{})", y + bar))
                .collect();
            println!(
                "{i} {} box ({},{})-({},{}) knots {}",
                String::from_utf8_lossy(&g.core.tag.0),
                r.x0,
                r.y0 + bar,
                r.x1,
                r.y1 + bar,
                knots.join(" ")
            );
        }
    }

    /// Opens the widget `path`, as dropped on the window.
    fn open(&mut self, path: &str) {
        let data = std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
        self.editor.drop_file(engine::editor::Dropped {
            name: path.rsplit('/').next().unwrap_or(path).to_owned(),
            path: Some(path.to_owned()),
            data,
        });
        self.wait(300);
    }

    fn shot(&mut self, name: &str) {
        self.editor.draw();
        let s = &self.editor.surface;
        let (w, h) = (s.width, s.height);
        let mut px: Vec<[u8; 4]> = s
            .pixels
            .iter()
            .map(|&i| {
                let c = self.palette[usize::from(i)];
                [c[0], c[1], c[2], 255]
            })
            .collect();
        // The game's pointer, as the original draws it (the system arrow
        // isn't drawn: the original's screenshots don't have it either).
        if let Pointer::Game(i) = self.editor.pointer()
            && let Some(c) = self.editor.cursor_picture(i)
        {
            let (cw, ch) = (i32::from(c.width), i32::from(c.height));
            for cy in 0..ch {
                for cx in 0..cw {
                    let a = usize::try_from((cy * cw + cx) * 4).unwrap();
                    if c.rgba[a + 3] == 0 {
                        continue;
                    }
                    let x = self.mouse.0 - i32::from(c.hot.0) + cx;
                    let y = self.mouse.1 - i32::from(c.hot.1) + cy;
                    if (0..w).contains(&x) && (0..h).contains(&y) {
                        px[usize::try_from(y * w + x).unwrap()] = [c.rgba[a], c.rgba[a + 1], c.rgba[a + 2], 255];
                    }
                }
            }
        }
        let png = formats::png::encode(&formats::png::Rgba {
            width: w.unsigned_abs(),
            height: h.unsigned_abs(),
            pixels: px,
        });
        std::fs::write(self.out.join(format!("{name}.png")), png).unwrap();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `-` for no CD: the port's own packs alone.
    let open = args[1] == "-";
    let cd = Dir(PathBuf::from(&args[1]));
    let script = std::fs::read_to_string(&args[2]).unwrap();
    let out = PathBuf::from(&args[3]);
    std::fs::create_dir_all(&out).unwrap();
    let (graphics, data) = (engine::opengfx::builtin(), engine::opendata::builtin());
    let packs = Packs {
        graphics: (open || args.iter().any(|a| a == "--opengfx")).then_some(&graphics as _),
        sounds: None,
        data: open.then_some(&data as _),
    };
    let assets = Assets::load_with((!open).then_some(&cd as _), packs, None).unwrap();
    let palette = assets.palette();
    let mut size = (640, 433);
    if let Some(l) = script.lines().find(|l| l.trim_start().starts_with("size ")) {
        let n: Vec<u32> = l.split_whitespace().skip(1).filter_map(|v| v.parse().ok()).collect();
        if let [w, h] = n[..] {
            size = (w, h);
        }
    }
    let mut editor = Editor::new(assets, Document::new(), size.0, size.1);
    editor.set_enhanced(args.iter().any(|a| a == "--enhanced"));
    let mut run = Run {
        editor,
        palette,
        mouse: (0, 0),
        out,
    };
    run.wait(500);
    for (n, line) in script.lines().enumerate() {
        let line = line.split('#').next().unwrap_or_default().trim();
        let w: Vec<&str> = line.split_whitespace().collect();
        let Some(&cmd) = w.first() else { continue };
        let num = |i: usize| -> i32 {
            let v = w.get(i).unwrap_or_else(|| panic!("line {}: too few numbers", n + 1));
            v.strip_prefix("0x")
                .map_or_else(|| v.parse().ok(), |h| i32::from_str_radix(h, 16).ok())
                .unwrap_or_else(|| panic!("line {}: bad number {v}", n + 1))
        };
        let button = if w.contains(&"right") {
            Button::Right
        } else {
            Button::Left
        };
        match cmd {
            "size" => {}
            "move" => run.to(num(1), num(2)),
            "down" => {
                run.to(num(1), num(2));
                run.editor.mouse_down(num(1), num(2), button);
                run.wait(FRAME_MS);
            }
            "up" => {
                run.to(num(1), num(2));
                run.editor.mouse_up(num(1), num(2), button);
                run.wait(FRAME_MS);
            }
            "click" | "dclick" => {
                run.to(num(1), num(2));
                for _ in 0..if cmd == "dclick" { 2 } else { 1 } {
                    run.editor.mouse_down(num(1), num(2), button);
                    run.wait(80);
                    run.editor.mouse_up(num(1), num(2), button);
                    run.wait(120);
                }
            }
            "drag" => {
                run.to(num(1), num(2));
                run.editor.mouse_down(num(1), num(2), button);
                run.wait(200);
                run.to(num(3), num(4));
                run.wait(200);
                run.editor.mouse_up(num(3), num(4), button);
                run.wait(200);
            }
            "key" => {
                let vk = u16::try_from(num(1)).unwrap();
                run.editor.key_down(vk);
                run.wait(80);
                run.editor.key_up(vk);
                run.wait(80);
            }
            "wait" => run.wait(u64::try_from(num(1)).unwrap()),
            "shot" => run.shot(w.get(1).copied().unwrap_or("shot")),
            "dump" => run.dump(),
            "open" => run.open(w.get(1).copied().unwrap_or_default()),
            "puzzle" => {
                let n = i16::try_from(num(1)).unwrap();
                assert!(run.editor.load_puzzle(n), "no puzzle {n}");
                run.wait(300);
            }
            _ => panic!("line {}: unknown command {cmd}", n + 1),
        }
    }
}
