// A window, not a console, on Windows (the release build).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! OpenWGT, an open engine for Widget Workshop. Opens straight into the
//! editor (the original's main menu is left out), with a new widget or
//! the one given.
//!
//! `openwgt [--open] [--lang=en|sv|fr] [--opengfx[=DIR]] [--opensfx[=DIR]]
//! [--opendata[=DIR]] [--overrides=DIR] [--export=DIR] [--no-audio]
//! [--window-size=WxH] [--fullscreen|--windowed] [--ui-scale=N|auto] [--help]
//! [CD] [WIDGET.WGT|LINK]` ([`USAGE`]), where `CD` is the game's CD: an
//! ISO image or a folder with its files (default: the one `.iso` in the
//! current directory, else `orig`), and `LINK` a widget link
//! (`engine::link`). `--overrides` names a folder whose
//! files replace or add to the CD's resources (`engine::assets`);
//! `--export` writes every resource there in that layout and quits.
//! `--opengfx`, `--opensfx` and `--opendata` use the port's own pictures,
//! sounds and data instead of the CD's (`engine::opengfx`,
//! `engine::opensfx`, `engine::opendata`); `--open` uses the three and no
//! CD. Started without a CD and with none here or picked before (`pick`),
//! it runs on the three packs, as with `--open`. In the browser (`web`,
//! `docs/web.md`) the page asks for the CD image or plays without it, and
//! its query is the command line.

/// In the browser `std`'s standard output and error go nowhere: what the
/// port says goes to the console (`platform::web::log`).
#[cfg(target_arch = "wasm32")]
macro_rules! eprintln {
    ($($t:tt)*) => {
        platform::web::log(&format!($($t)*))
    };
}
#[cfg(target_arch = "wasm32")]
macro_rules! println {
    ($($t:tt)*) => {
        platform::web::log(&format!($($t)*))
    };
}

mod cd;
mod files;
#[cfg(not(target_arch = "wasm32"))]
mod pick;
mod profile;
mod sound;
#[cfg(target_arch = "wasm32")]
mod web;

use std::path::PathBuf;

use web_time::Duration;

use engine::assets::{Assets, Source};
use engine::doc::Document;
use engine::editor::{self, Editor};
use platform::{App, Button, Cursor, CursorImage, Frame, Input, UiScale, WindowOptions};

use crate::sound::Sound;

/// The page widget links open natively: the browser build on the
/// project's site (`.github/workflows/build.yml`). In the browser they
/// open the page they were made on.
#[cfg(not(target_arch = "wasm32"))]
const LINK_PAGE: &str = "https://gustavn.se/openwgt/play/";

const USAGE: &str =
    "usage: openwgt [--open] [--lang=en|sv|fr] [--opengfx[=DIR]] [--opensfx[=DIR]] [--opendata[=DIR]] [--overrides=DIR] [--export=DIR] [--no-audio] [--window-size=WxH] \
[--fullscreen|--windowed] [--ui-scale=N|auto] [--help] [CD] [WIDGET.WGT|LINK]

  CD               the game's CD: an ISO image or a folder with its files
                   (default: the .iso in the current directory, else orig)
  WIDGET.WGT       a widget to open
  LINK             a widget link to open (File > Copy Link to Widget's)
  --opengfx        draw with the port's own graphics, OpenWGFX, instead
                   of the CD's pictures
  --opensfx        play the port's own sounds, OpenWSFX, instead of the
                   CD's
  --opendata       the port's own data (texts, dialogs, tables, help,
                   puzzles, samples) instead of the CD's
  --open           no CD at all: the three packs alone (also what it does
                   when no CD is given or found)
  --lang=en|sv|fr  the language of the port's own packs (default: the
                   system's); with a CD the CD's language is used
  --opengfx=DIR, --opensfx=DIR, --opendata=DIR
                   the same from a folder of that pack's layout
  --overrides=DIR  resources that replace or add to the CD's:
                   DIR/<MODULE>/<type>/<id>.<ext>, fonts in DIR/fonts,
                   puzzles in DIR/WIDGET.PZL, help in DIR/WIDGET.HLP
  --export=DIR     write every resource to DIR in that layout and quit
  --no-audio       no sound
  --window-size=WxH, --fullscreen, --windowed
                   the window (remembered in the settings file)
  --ui-scale=N|auto
                   window pixels per game pixel, such as 2 or 1.5, in
                   quarters; auto follows the display's scaling
                   (remembered, ui_scale in the settings file)";

struct Args {
    cd: Option<PathBuf>,
    widget: Option<PathBuf>,
    link: Option<String>,
    overrides: Option<PathBuf>,
    export: Option<PathBuf>,
    /// `--opengfx`: the port's own graphics.
    opengfx: Option<Pack>,
    /// `--opensfx`: the port's own sounds.
    opensfx: Option<Pack>,
    /// `--opendata`: the port's own data (everything else).
    opendata: Option<Pack>,
    /// `--open`: no CD at all, only the port's own packs.
    open: bool,
    /// `--lang`: the language of the port's own packs (with a CD, the
    /// CD's wins).
    lang: Option<engine::lang::Lang>,
    /// No CD given or found: the packs were switched on for it, not by
    /// the player (so Change CD... starts again without them).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    auto_open: bool,
    no_audio: bool,
    window: WindowOptions,
}

/// Where `--opengfx` or `--opensfx` takes the port's own graphics or
/// sounds from.
#[derive(Debug, Clone)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
enum Pack {
    /// The pack built into the port.
    Builtin,
    /// `--opengfx=DIR`: a folder of the pack's layout.
    Dir(PathBuf),
}

/// The browser build's way to stop: a panic with this message, which the
/// page's panic hook (`web::main`) leaves unshown, the reason already
/// being on the page.
#[cfg(target_arch = "wasm32")]
const STOPPED: &str = "OpenWGT stopped";

/// Ends the program with `code`. The browser build can't: the module
/// stops by panicking, its reason ([`fail`], [`usage_error`], `--help`'s
/// text) left on the page.
fn exit(code: i32) -> ! {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = code;
        std::panic::panic_any(STOPPED);
    }
    #[cfg(not(target_arch = "wasm32"))]
    std::process::exit(code)
}

/// `msg` where the player sees it (`pick::fatal`, in the browser the
/// page), and the end.
fn fail(msg: &str) -> ! {
    #[cfg(not(target_arch = "wasm32"))]
    pick::fatal(msg);
    #[cfg(target_arch = "wasm32")]
    {
        eprintln!("{msg}");
        platform::web::show_error(msg);
    }
    exit(1);
}

fn usage_error(msg: &str) -> ! {
    #[cfg(not(target_arch = "wasm32"))]
    pick::fatal(&format!("{msg}\n{USAGE}"));
    #[cfg(target_arch = "wasm32")]
    platform::web::show_error(msg);
    exit(2);
}

fn args(mut it: impl Iterator<Item = std::ffi::OsString>) -> Args {
    let mut a = Args {
        cd: None,
        widget: None,
        link: None,
        overrides: None,
        export: None,
        opengfx: None,
        opensfx: None,
        opendata: None,
        open: false,
        lang: None,
        auto_open: false,
        no_audio: false,
        window: WindowOptions {
            settings_file: platform::settings::settings_file(),
            ..WindowOptions::default()
        },
    };
    while let Some(arg) = it.next() {
        let s = arg.to_string_lossy().into_owned();
        let (key, value) = match s.split_once('=') {
            Some((k, v)) if k.starts_with("--") => (k.to_owned(), Some(v.to_owned())),
            _ => (s.clone(), None),
        };
        let mut value = || {
            value
                .clone()
                .or_else(|| it.next().map(|v| v.to_string_lossy().into_owned()))
        };
        match key.as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                #[cfg(target_arch = "wasm32")]
                platform::web::show_status(USAGE, false);
                exit(0);
            }
            "--overrides" => a.overrides = value().map(PathBuf::from),
            "--lang" => {
                let v = value().unwrap_or_default();
                a.lang = engine::lang::Lang::from_code(&v);
                if a.lang.is_none() {
                    fail(&format!("--lang={v}: not a language of the port's (en, sv, fr)"));
                }
            }
            "--export" => a.export = value().map(PathBuf::from),
            // Its folder only after `=`: a bare `--opengfx` takes no value.
            "--opengfx" | "--opensfx" | "--opendata" => {
                let pack = match s.split_once('=') {
                    Some((_, dir)) => Pack::Dir(PathBuf::from(dir)),
                    None => Pack::Builtin,
                };
                match key.as_str() {
                    "--opengfx" => a.opengfx = Some(pack),
                    "--opensfx" => a.opensfx = Some(pack),
                    _ => a.opendata = Some(pack),
                }
            }
            "--open" => a.open = true,
            "--no-audio" => a.no_audio = true,
            "--fullscreen" => a.window.fullscreen = Some(true),
            "--windowed" => a.window.fullscreen = Some(false),
            "--window-size" => {
                let v = value().unwrap_or_default();
                let size = v
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
                a.window.size = Some(size.unwrap_or_else(|| usage_error(&format!("bad window size {v}"))));
            }
            "--ui-scale" => {
                let v = value().unwrap_or_default();
                let scale = platform::UiScale::parse(&v);
                a.window.ui_scale = Some(scale.unwrap_or_else(|| usage_error(&format!("bad UI scale {v}"))));
            }
            k if k.starts_with('-') => usage_error(&format!("unknown option {k}")),
            _ => {
                let p = PathBuf::from(&s);
                let is_widget = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("wgt"));
                let is_link = engine::link::decode(&s).is_some();
                if (is_widget || is_link) && a.widget.is_none() && a.link.is_none() {
                    if is_link {
                        a.link = Some(s);
                    } else {
                        a.widget = Some(p);
                    }
                } else if a.cd.is_none() {
                    a.cd = Some(p);
                } else {
                    usage_error(&format!("too many arguments: {s}"));
                }
            }
        }
    }
    a
}

/// The game in the window.
struct Workshop {
    editor: Editor,
    sound: Sound,
    palette: [[u8; 3]; 256],
    theme: platform::theme::SystemTheme,
    /// Where the enhancements' switch is kept (`original=`).
    settings_file: Option<PathBuf>,
    /// The command line's options to start over with on another CD
    /// (Settings ▸ Change CD...), natively.
    #[cfg(not(target_arch = "wasm32"))]
    options: Vec<std::ffi::OsString>,
    /// Started over on another CD: this one ends.
    done: bool,
    /// The time not yet handed to the editor, which counts whole
    /// milliseconds.
    carry: Duration,
    /// Change CD... where there's no file dialog: the window asks for the
    /// CD image to be dropped on it instead of showing the game.
    #[cfg(not(target_arch = "wasm32"))]
    asking: Option<pick::DropScreen>,
}

impl Workshop {
    /// Settings ▸ Language (or Play without the CD): kept in the settings
    /// file, and the game started again in it without a CD, even with one
    /// remembered or here (the page reloaded in the browser).
    fn change_lang(&mut self, lang: engine::lang::Lang) {
        if let Some(path) = &self.settings_file
            && let Err(e) = platform::settings::save_value(path, LANGUAGE_KEY, lang.code())
        {
            eprintln!("{}: {e}", path.display());
        }
        #[cfg(target_arch = "wasm32")]
        web::change_lang(lang.code());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let exe = std::env::current_exe();
            let started = exe.and_then(|exe| {
                std::process::Command::new(exe)
                    .args(&self.options)
                    .arg("--open")
                    .arg(format!("--lang={}", lang.code()))
                    .spawn()
            });
            match started {
                Ok(_) => self.done = true,
                Err(e) => pick::error_box(&engine::lang::trf("OpenWGT didn't start again: {}", &[&e.to_string()])),
            }
        }
    }

    /// Settings ▸ Play without the CD: from now on the game starts on the
    /// port's own packs, in the language chosen for them (the settings
    /// file's, else the system's), until another CD is picked; natively
    /// the CD here or picked before is set aside, in the browser the page
    /// reloads without the stored image (kept, for Change CD...).
    #[cfg_attr(target_arch = "wasm32", allow(clippy::unused_self))]
    fn play_without_cd(&mut self) {
        #[cfg(target_arch = "wasm32")]
        web::play_without_cd();
        #[cfg(not(target_arch = "wasm32"))]
        {
            pick::play_without();
            let exe = std::env::current_exe();
            let started = exe.and_then(|exe| {
                std::process::Command::new(exe)
                    .args(&self.options)
                    .arg("--open")
                    .spawn()
            });
            match started {
                Ok(_) => self.done = true,
                Err(e) => pick::error_box(&engine::lang::trf("OpenWGT didn't start again: {}", &[&e.to_string()])),
            }
        }
    }

    /// Settings ▸ Change CD..., the widget saved or let go: natively the
    /// player picks another CD image (or drops it on the window where
    /// there's no file dialog, `pick`) and the game starts again on it;
    /// in the browser the page reloads into its start form.
    /// (In the browser the page does it all.)
    #[cfg_attr(target_arch = "wasm32", allow(clippy::unused_self))]
    fn change_cd(&mut self) {
        #[cfg(target_arch = "wasm32")]
        web::change_cd();
        #[cfg(not(target_arch = "wasm32"))]
        if !pick::has_dialog() {
            let s = &self.editor.surface;
            let mut screen = pick::DropScreen::new(true, self.theme.dark());
            screen.resize(
                u32::try_from(s.width).unwrap_or(0),
                u32::try_from(s.height).unwrap_or(0),
            );
            self.asking = Some(screen);
        } else if let Some(cd) = pick::choose()
            && let Err(e) = self.restart(&cd)
        {
            pick::error_box(&e);
        }
    }

    /// The game started again on `cd`, this one ending; why not if it
    /// didn't.
    #[cfg(not(target_arch = "wasm32"))]
    fn restart(&mut self, cd: &std::path::Path) -> Result<(), String> {
        let exe = std::env::current_exe();
        let started = exe.and_then(|exe| std::process::Command::new(exe).args(&self.options).arg(cd).spawn());
        match started {
            Ok(_) => {
                self.done = true;
                Ok(())
            }
            Err(e) => Err(engine::lang::trf("OpenWGT didn't start again: {}", &[&e.to_string()])),
        }
    }
}

impl App for Workshop {
    fn update(&mut self, elapsed: Duration) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.asking.is_some() {
            return;
        }
        self.carry += elapsed;
        let ms = u64::try_from(self.carry.as_millis()).unwrap_or(u64::MAX);
        self.carry -= Duration::from_millis(ms);
        if let Some(dark) = self.theme.changed() {
            self.editor.set_system_dark(dark);
        }
        #[cfg(target_arch = "wasm32")]
        for l in web::take_links() {
            self.editor.open_link(&l);
        }
        self.editor.update(ms, sound::local_time());
        if self.editor.take_change_cd() {
            self.change_cd();
        }
        if let Some(lang) = self.editor.take_change_lang() {
            self.change_lang(lang);
        }
        if self.editor.take_play_without_cd() {
            self.play_without_cd();
        }
        if let Some(on) = self.editor.take_enhanced()
            && let Some(path) = &self.settings_file
            && let Err(e) = platform::settings::EnhanceSettings::save(path, Some(!on), &[], &[])
        {
            eprintln!("{}: {e}", path.display());
        }
        let cmds = self.editor.take_sounds();
        self.sound.play(cmds);
        self.editor.draw();
    }

    fn input(&mut self, input: Input) {
        // Asking for a CD: Esc goes back to the game, closing quits it.
        #[cfg(not(target_arch = "wasm32"))]
        if self.asking.is_some() {
            match input {
                Input::KeyUp(0x1b) => self.asking = None,
                Input::Close => {
                    self.asking = None;
                    self.editor.quit();
                }
                _ => {}
            }
            return;
        }
        let button = |b| match b {
            Button::Right => Some(editor::Button::Right),
            Button::Left => Some(editor::Button::Left),
            Button::Middle => None,
        };
        match input {
            Input::MouseMove { x, y } => self.editor.mouse_move(x, y),
            Input::MouseDown { x, y, button: b } => {
                if let Some(b) = button(b) {
                    self.editor.mouse_down(x, y, b);
                }
            }
            Input::MouseUp { x, y, button: b } => {
                if let Some(b) = button(b) {
                    self.editor.mouse_up(x, y, b);
                }
            }
            Input::Wheel { dx, dy, .. } => self.editor.wheel(dx, dy),
            Input::KeyDown(vk) => self.editor.key_down(vk),
            Input::KeyUp(vk) => self.editor.key_up(vk),
            Input::Close => self.editor.quit(),
        }
    }

    fn wants_text(&self) -> bool {
        self.editor.wants_text()
    }

    fn take_text_tapped(&mut self) -> Option<platform::TextEdit> {
        self.editor.take_text_tapped().map(|f| platform::TextEdit {
            text: f.text,
            multiline: f.multiline,
            max: f.max,
        })
    }

    fn text_edited(&mut self, text: String) {
        self.editor.set_focused_text(&text);
    }

    fn touch_width(&self) -> u32 {
        self.editor.bars_width()
    }

    fn long_press_doubles(&self) -> bool {
        !self.editor.running
    }

    fn frame(&self) -> Frame<'_> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(screen) = &self.asking {
            return screen.frame();
        }
        let s = &self.editor.surface;
        Frame {
            width: u32::try_from(s.width).unwrap_or(0),
            height: u32::try_from(s.height).unwrap_or(0),
            pixels: &s.pixels,
            palette: &self.palette,
            overlay: None,
            cursor: if self.editor.pointer() == editor::Pointer::Text {
                Cursor::Text
            } else {
                Cursor::Default
            },
            cursor_image: match self.editor.pointer() {
                editor::Pointer::Game(i) => self.editor.cursor_picture(i).map(|c| CursorImage {
                    key: u32::from(i),
                    width: c.width,
                    height: c.height,
                    hot: c.hot,
                    rgba: &c.rgba,
                }),
                editor::Pointer::Arrow | editor::Pointer::Text => None,
            },
            title: Some(self.editor.title()),
        }
    }

    fn finished(&self) -> bool {
        self.done || self.editor.finished()
    }

    fn resize(&mut self, width: u32, height: u32) {
        // On a touch screen the desktop may be as small as the toolbars.
        let least = if platform::touch_screen() {
            (self.editor.bars_width(), platform::TOUCH_MIN_HEIGHT)
        } else {
            platform::MIN_DESKTOP
        };
        let (w, h) = (width.max(least.0), height.max(least.1));
        self.editor.resize(w, h);
        self.editor.draw();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(screen) = &mut self.asking {
            screen.resize(w, h);
        }
    }

    fn dropped(&mut self, file: platform::DroppedFile) {
        self.editor.drop_file(editor::Dropped {
            name: file.name,
            path: file.path.map(|p| p.display().to_string()),
            data: file.data,
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn dropped_path(&mut self, path: &std::path::Path) -> bool {
        let Some(screen) = &mut self.asking else {
            return false;
        };
        if screen.take(path) {
            match self.restart(path) {
                Ok(()) => self.asking = None,
                Err(e) => {
                    if let Some(screen) = &mut self.asking {
                        screen.set_error(e);
                    }
                }
            }
        }
        true
    }

    fn pasted(&mut self, text: String) {
        self.editor.paste_link(&text);
    }

    fn take_fullscreen(&mut self) -> Option<bool> {
        self.editor.take_full_screen()
    }

    fn fullscreen_changed(&mut self, on: bool) {
        self.editor.set_full_screen(on);
    }

    fn take_ui_scale(&mut self) -> Option<UiScale> {
        self.editor.take_ui_scale().map(|s| match s {
            editor::settings::Scale::Auto => UiScale::Auto,
            editor::settings::Scale::Fixed(n) => UiScale::Fixed(n),
        })
    }

    fn ui_scale_changed(&mut self, asked: UiScale, scale: f64) {
        let asked = match asked {
            UiScale::Auto => editor::settings::Scale::Auto,
            UiScale::Fixed(n) => editor::settings::Scale::Fixed(n),
        };
        self.editor.set_ui_scale(asked, scale);
    }
}

/// The CD to read when none is given: of the `.iso`s here, the first with
/// the original fonts ([`engine::assets::FONT_DIRS`]: the Mac/PC CD, which
/// has the help too), else the first with the game, else the first that
/// opens, else `orig`.
#[cfg(not(target_arch = "wasm32"))]
fn default_cd() -> PathBuf {
    let mut isos: Vec<PathBuf> = std::fs::read_dir(".")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("iso")))
        .collect();
    isos.sort();
    let opened: Vec<(&PathBuf, Box<dyn Source>)> = isos.iter().filter_map(|p| Some((p, cd::open(p).ok()?))).collect();
    opened
        .iter()
        .find(|(_, cd)| cd::has_fonts(cd.as_ref()))
        .or_else(|| opened.iter().find(|(_, cd)| cd::has_game(cd.as_ref())))
        .or(opened.first())
        .map_or_else(|| PathBuf::from("orig"), |(p, _)| (*p).clone())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // A panic's message in a box too when started from the icon.
    let report = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        report(info);
        if !std::io::IsTerminal::is_terminal(&std::io::stderr()) {
            pick::error_box(&engine::lang::trf("OpenWGT stopped: {}", &[&info.to_string()]));
        }
    }));
    let mut args = args(std::env::args_os().skip(1));
    // The CD: the one given, else one here or picked before. With none
    // (or `--open`) the game runs on the port's own packs alone.
    let cd_path = if args.open {
        None
    } else {
        args.cd.clone().or_else(|| pick::found(&default_cd()))
    };
    if cd_path.is_none() {
        args.auto_open = !args.open;
        args.open = true;
        for p in [&mut args.opengfx, &mut args.opensfx, &mut args.opendata] {
            p.get_or_insert(Pack::Builtin);
        }
    }
    let cd = cd_path.as_ref().and_then(|p| match cd::open(p) {
        Ok(cd) => Some(cd),
        Err(e) if args.overrides.is_some() => {
            eprintln!("{}: {e}; using the overrides alone", p.display());
            None
        }
        Err(e) => fail(&format!("{}: {e}", p.display())),
    });
    let cd_path = cd_path.unwrap_or_else(|| PathBuf::from("(no CD)"));
    let overrides = args
        .overrides
        .clone()
        .map(|d| Box::new(engine::assets::Dir(d)) as Box<dyn Source>);
    let lang = language(&args, cd.as_deref());
    let graphics = args.opengfx.as_ref().map(|g| match g {
        Pack::Dir(d) => Box::new(engine::assets::Dir(d.clone())) as Box<dyn Source>,
        Pack::Builtin => engine::opengfx::builtin_for(lang),
    });
    let sounds = args.opensfx.as_ref().map(|g| match g {
        Pack::Dir(d) => Box::new(engine::assets::Dir(d.clone())) as Box<dyn Source>,
        Pack::Builtin => Box::new(engine::opensfx::builtin()),
    });
    let data = args.opendata.as_ref().map(|g| match g {
        Pack::Dir(d) => Box::new(engine::assets::Dir(d.clone())) as Box<dyn Source>,
        Pack::Builtin => Box::new(engine::opendata::builtin_for(lang)),
    });
    let packs = engine::assets::Packs {
        graphics: graphics.as_deref(),
        sounds: sounds.as_deref(),
        data: data.as_deref(),
    };
    let mut assets = load(&cd_path, cd.as_deref(), packs, overrides.as_deref());
    if cd.is_none() {
        // A data pack from a folder says its own language (its word for
        // Cancel): the parts reckon in it, whatever the system's.
        let own = matches!(args.opendata, Some(Pack::Dir(_)))
            .then(|| assets.string(0xf221))
            .flatten()
            .and_then(|w| engine::lang::Lang::of_cancel(&w));
        assets.lang = own.unwrap_or(lang);
    }
    if let Some(dir) = &args.export {
        match cd::export(&assets, dir) {
            Ok(n) => println!("{n} files written to {}", dir.display()),
            Err(e) => fail(&format!("{}: {e}", dir.display())),
        }
        return;
    }
    if let Err(e) = start(&args, assets) {
        fail(&e.to_string());
    }
}

/// The options of `args` to start again with on another CD: the window's
/// are in the settings file by then.
#[cfg(not(target_arch = "wasm32"))]
fn restart_options(args: &Args) -> Vec<std::ffi::OsString> {
    let mut o = Vec::new();
    if let Some(d) = &args.overrides {
        let mut a = std::ffi::OsString::from("--overrides=");
        a.push(d);
        o.push(a);
    }
    if args.no_audio {
        o.push("--no-audio".into());
    }
    // Another CD was chosen: not `--open`, nor the packs that stood in for
    // having none.
    if args.auto_open {
        return o;
    }
    for (flag, pack) in [
        ("--opengfx", &args.opengfx),
        ("--opensfx", &args.opensfx),
        ("--opendata", &args.opendata),
    ] {
        if let Some(p) = pack {
            let mut a = std::ffi::OsString::from(flag);
            if let Pack::Dir(d) = p {
                a.push("=");
                a.push(d);
            }
            o.push(a);
        }
    }
    o
}

/// The browser build: `web` sets up the files and then [`start`]s.
#[cfg(target_arch = "wasm32")]
fn main() {
    web::main();
}

/// The settings file's key for the language chosen in the Settings
/// window.
const LANGUAGE_KEY: &str = "language";

/// The language to play in: the CD's, else `--lang`'s, else the one
/// chosen in the Settings window (the settings file's), else the system's
/// (the browser's), else English.
fn language(args: &Args, cd: Option<&dyn Source>) -> engine::lang::Lang {
    use engine::lang::Lang;
    cd.and_then(engine::assets::cd_language)
        .or(args.lang)
        .or_else(|| {
            let path = args.window.settings_file.as_deref()?;
            Lang::from_code(&platform::settings::load_value(path, LANGUAGE_KEY)?)
        })
        .or_else(|| sys_locale::get_locale().and_then(|l| Lang::from_code(&l)))
        .unwrap_or_default()
}

/// The game's resources from `cd`, the port's own `packs` (`--opengfx`,
/// `--opensfx`) and `overrides`; the end if they don't load.
fn load(
    cd_path: &std::path::Path,
    cd: Option<&dyn Source>,
    packs: engine::assets::Packs<'_>,
    overrides: Option<&dyn Source>,
) -> Assets {
    let assets =
        Assets::load_with(cd, packs, overrides).unwrap_or_else(|e| fail(&format!("{}: {e}", cd_path.display())));
    for p in &assets.pack_problems {
        eprintln!("{p}");
    }
    assets
}

/// Opens the editor on `assets` as `args` say: the widget, the sound,
/// the window. Natively it returns when the game ends; in the browser at
/// once, the page's events running the game from then on.
///
/// # Errors
/// If the window or the GPU can't be set up.
fn start(args: &Args, assets: Assets) -> Result<(), platform::Error> {
    let mut doc = Document::new();
    let mut path = None;
    if let Some(p) = &args.widget {
        match platform::fs::read(p)
            .map_err(|e| e.to_string())
            .and_then(|b| Document::load(&assets, &b).map_err(|e| e.to_string()))
        {
            Ok(d) => {
                doc = d;
                path = Some(p.display().to_string());
            }
            Err(e) => eprintln!("{}: {e}", p.display()),
        }
    }
    let palette = assets.palette();
    let mut editor = Editor::new(assets, doc, platform::DEFAULT_DESKTOP.0, platform::DEFAULT_DESKTOP.1);
    editor.path = path;
    // The solved puzzles (`Done=`), kept between runs.
    if let Some(p) = profile::IniProfile::default_file() {
        editor.profile = Box::new(p);
    }
    editor.files = Box::new(files::PlatformFiles);
    // In the browser the dialogs see only its storage: widgets go out as
    // downloads too.
    #[cfg(target_arch = "wasm32")]
    editor.set_download(Box::new(platform::web::download));
    #[cfg(not(target_arch = "wasm32"))]
    let base = LINK_PAGE.to_owned();
    #[cfg(target_arch = "wasm32")]
    let base = web::page();
    editor.set_links(editor::Links {
        base,
        copy: Box::new(platform::clipboard::set_text),
        paste: Box::new(platform::clipboard::request_text),
    });
    if let Some(l) = &args.link {
        editor.open_link(l);
    }
    // `0x415018`: the random generator starts at `(u32)time(NULL)`.
    editor.env.rng = chrono::Utc::now().timestamp().rem_euclid(1 << 32).unsigned_abs();
    let audio = if args.no_audio {
        platform::audio::Audio::silent()
    } else {
        // Without sound there's no microphone either (`NoMic`).
        editor.mic = Box::new(sound::Microphone::default());
        platform::audio::Audio::open()
    };
    let theme = platform::theme::SystemTheme::new();
    editor.set_system_dark(theme.dark());
    // The port's enhancements only if the player turned them on
    // (`original=false` in the settings file): as the original otherwise.
    let settings_file = args.window.settings_file.clone();
    let enhanced = settings_file
        .as_deref()
        .is_some_and(|p| platform::settings::EnhanceSettings::load(p).original == Some(false));
    editor.set_enhanced(enhanced);
    let app = Workshop {
        editor,
        sound: Sound::new(audio),
        palette,
        theme,
        settings_file,
        #[cfg(not(target_arch = "wasm32"))]
        options: restart_options(args),
        done: false,
        carry: Duration::ZERO,
        #[cfg(not(target_arch = "wasm32"))]
        asking: None,
    };
    platform::run("OpenWGT", args.window.clone(), app)
}
