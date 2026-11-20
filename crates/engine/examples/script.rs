//! Plays a script of mouse and key events into the editor, with no window,
//! and writes a PNG of the screen (the game's pointer drawn on it) at
//! each `shot`: for comparing the port with the original running the same
//! script (`tools/compare/`).
//!
//! `cargo run -p engine --example script -- CD_DIR SCRIPT OUT_DIR [--opengfx] [--enhanced] [--opendata=DIR] [--lang=CODE]`
//! (`-` for `CD_DIR`: no CD, the port's own packs alone; `--opendata=DIR`
//! a data pack from a folder, such as the Swedish `crates/engine/opendata-sv`;
//! `--lang=fr` the port's own words in that language and its built-in packs:
//! its data, its pictures with words over the English ones). Settings ▸
//! Language switches as the game does: the packs are built again in the
//! language it asks for and handed to the editor.
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
//!
//! The same files take the commands that work with parts by name and with
//! the run (`run`, `press Switch`, `set Slider 5`, `until Lamp == on`,
//! `expect Counter > 3`, `step 60`...): `engine::script::cmd`, whose
//! `help` lists them all. A failed `expect` or `assert` makes it exit with
//! 1. `tools/compare/run_vm.py` plays only the commands above.

use std::path::PathBuf;

use engine::assets::{Assets, Dir, Packs};
use engine::doc::Document;
use engine::editor::Editor;
use engine::script::runtime::run_headless;
use engine::script::{Caps, Host, Level, Shot, Sink, cmd};

/// The log on the standard output (errors on the standard error), the
/// shots as PNGs in a folder.
struct Out(PathBuf);

impl Sink for Out {
    fn log(&mut self, level: Level, msg: &str) {
        match level {
            Level::Info => println!("{msg}"),
            Level::Warn | Level::Error => eprintln!("{msg}"),
        }
    }

    fn shot(&mut self, name: &str, shot: &Shot) -> Result<(), String> {
        let path = self.0.join(format!("{name}.png"));
        std::fs::write(&path, shot.png()).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The game's assets: the CD's at `args[1]` (`-`: none, the port's own
/// packs alone), with the packs and language the options ask for; or,
/// with `switch`, the packs alone in that language (Settings ▸ Language).
fn load(args: &[String], switch: Option<engine::lang::Lang>) -> Assets {
    let open = args[1] == "-" || switch.is_some();
    let cd = Dir(PathBuf::from(&args[1]));
    let lang = switch.unwrap_or_else(|| {
        args.iter()
            .find_map(|a| a.strip_prefix("--lang="))
            .map(|c| engine::lang::Lang::from_code(c).unwrap_or_else(|| panic!("no language {c}")))
            .unwrap_or_default()
    });
    let graphics = engine::opengfx::builtin_for(lang);
    let data: Box<dyn engine::assets::Source> = match args.iter().find_map(|a| a.strip_prefix("--opendata=")) {
        Some(dir) => Box::new(Dir(PathBuf::from(dir))),
        None => Box::new(engine::opendata::builtin_for(lang)),
    };
    let packs = Packs {
        graphics: (open || args.iter().any(|a| a == "--opengfx")).then_some(graphics.as_ref()),
        sounds: None,
        data: open.then_some(data.as_ref()),
    };
    let mut assets = Assets::load_with((!open).then_some(&cd as _), packs, None).unwrap();
    // The editor speaks the assets' language (a CD's, else this one).
    if switch.is_some() || args.iter().any(|a| a.starts_with("--lang=")) {
        assets.lang = lang;
    }
    assets
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let script = std::fs::read_to_string(&args[2]).unwrap();
    let out = PathBuf::from(&args[3]);
    std::fs::create_dir_all(&out).unwrap();
    let assets = load(&args, None);
    let size = cmd::size_of(&script).unwrap_or((640, 433));
    let mut editor = Editor::new(assets, Document::new(), size.0, size.1);
    editor.set_enhanced(args.iter().any(|a| a == "--enhanced"));
    editor.debug.sink = Box::new(Out(out));
    // The host's part of Settings ▸ Language.
    let again = args.clone();
    editor.debug.assets_for = Some(Box::new(move |lang| Some(load(&again, Some(lang)))));
    editor.wait(500).unwrap();
    let mut rt = cmd::CommandRuntime::new();
    match run_headless(&mut rt, &args[2], &script, Caps::ALL, &mut editor) {
        Ok(report) if report.passed() => {}
        Ok(_) => std::process::exit(1),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
