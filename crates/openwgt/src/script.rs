//! Scripts from the command line (natively): `--script FILE` runs a
//! script of the built-in command language (`engine::script::cmd`), or
//! of JavaScript if FILE ends in `.js` (`script_js`, with the feature
//! `js`), in the game, in its window or, with `--headless`, with none, as
//! fast as it goes, and then exits: 0 if it ran through, 1 if a check
//! failed or it stopped on an error, 2 if it couldn't start.
//!
//! - `--out DIR`: where `shot NAME` writes `NAME.png` (default: here);
//! - `--seed N`: the parts' random numbers start from `N` (default: the
//!   time, as the game's);
//! - `--budget N`: the work it may do between two waits (commands; for
//!   JavaScript roughly operations);
//! - `--timeout S`: stops it after `S` seconds of real time (headless).
//! - `--watch`: runs it again each time its file is saved: in the window
//!   at once (what ran stopped), headless once the last run ended.
//!
//! What it logs goes to the standard output, errors and failed checks to
//! the standard error.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use engine::doc::Document;
use engine::editor::Editor;
use engine::script::cmd::CommandRuntime;
use engine::script::runtime::{Limits, Runtime, TaskId, TaskState, run_headless};
use engine::script::{Caps, Host, Level, Shot, Sink, cmd};

/// The command line's script options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub file: Option<PathBuf>,
    pub headless: bool,
    pub out: Option<PathBuf>,
    pub seed: Option<u64>,
    pub budget: Option<u64>,
    pub timeout: Option<f64>,
    /// `--watch`: again each time the file is saved.
    pub watch: bool,
}

impl Options {
    /// Takes option `key`'s `value`.
    ///
    /// # Errors
    /// What's wrong with it.
    pub fn set(&mut self, key: &str, value: Option<String>) -> Result<(), String> {
        let v = value.ok_or_else(|| format!("{key} needs a value"))?;
        let bad = || format!("{key}: bad value {v}");
        match key {
            "--script" => self.file = Some(PathBuf::from(&v)),
            "--out" => self.out = Some(PathBuf::from(&v)),
            "--seed" => self.seed = Some(v.parse().map_err(|_| bad())?),
            "--budget" => self.budget = Some(v.parse().ok().filter(|&n| n > 0).ok_or_else(bad)?),
            "--timeout" => {
                self.timeout = Some(
                    v.parse()
                        .ok()
                        .filter(|s: &f64| s.is_finite() && *s >= 0.0)
                        .ok_or_else(bad)?,
                );
            }
            _ => return Err(format!("unknown option {key}")),
        }
        Ok(())
    }
}

/// How often `--watch` looks at the file.
const WATCH_EVERY: Duration = Duration::from_millis(300);

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// After a headless run ended with `code`: waits for the script's file
/// to be saved again (`--watch`).
pub fn wait_for_change(o: &Options, code: i32) {
    let Some(file) = &o.file else { return };
    let was = modified(file);
    eprintln!(
        "-- {}: exit {code}; waiting for it to be saved again (Ctrl+C stops) --",
        file.display()
    );
    while modified(file) == was {
        std::thread::sleep(WATCH_EVERY);
    }
}

/// Whether the program runs without a window (`--headless`): then it
/// shows no message boxes either.
pub fn headless() -> bool {
    std::env::args_os().any(|a| a == "--headless")
}

/// The log on the standard output and error, the shots in a folder.
struct Out {
    dir: PathBuf,
}

impl Sink for Out {
    fn log(&mut self, level: Level, msg: &str) {
        match level {
            Level::Info => println!("{msg}"),
            Level::Warn | Level::Error => eprintln!("{msg}"),
        }
    }

    fn shot(&mut self, name: &str, shot: &Shot) -> Result<(), String> {
        let path = self.dir.join(format!("{name}.png"));
        std::fs::write(&path, shot.png()).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The script's file and text, or why not (and the exit code 2).
fn read(o: &Options) -> Result<(String, String), String> {
    let file = o.file.as_ref().ok_or("--headless needs --script FILE")?;
    let src = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok((file.display().to_string(), src))
}

/// Sets the editor up for the script: its output, the seed, the language
/// switch, the time limit.
fn prepare(o: &Options, editor: &mut Editor, args: &crate::Args) -> Result<(), String> {
    let dir = o.out.clone().unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    editor.debug.sink = Box::new(Out { dir });
    if let Some(s) = o.seed {
        editor.env.rng = s;
    }
    // Settings ▸ Language while it lets time pass, as the game does.
    let mut again = args.clone();
    again.without_cd();
    editor.debug.assets_for = Some(Box::new(move |lang| crate::assets_for(&again, None, lang).ok()));
    if let Some(t) = o.timeout {
        let end = Instant::now() + Duration::from_secs_f64(t);
        editor.debug.deadline = Some(Box::new(move || Instant::now() >= end));
    }
    Ok(())
}

/// Whether `file` is JavaScript (by its name).
fn is_js(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("js") || e.eq_ignore_ascii_case("mjs"))
}

/// The language `file` is in.
///
/// # Errors
/// JavaScript, in a program built without it.
#[cfg_attr(feature = "js", allow(clippy::unnecessary_wraps))]
fn runtime(o: &Options, file: &str) -> Result<Box<dyn Runtime>, String> {
    let mut rt: Box<dyn Runtime> = if is_js(file) {
        #[cfg(feature = "js")]
        {
            Box::new(script_js::Js::new())
        }
        #[cfg(not(feature = "js"))]
        return Err(format!(
            "{file}: this program was built without JavaScript (the feature js)"
        ));
    } else {
        Box::new(CommandRuntime::new())
    };
    if let Some(b) = o.budget {
        rt.set_limits(Limits { budget: b });
    }
    Ok(rt)
}

/// Runs the script with no window; the exit code.
pub fn run_without_window(args: &crate::Args, assets: engine::assets::Assets) -> i32 {
    let o = &args.script;
    let (file, src) = match read(o) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let mut doc = Document::new();
    let mut path = None;
    if let Some(p) = &args.widget {
        match std::fs::read(p)
            .map_err(|e| e.to_string())
            .and_then(|b| Document::load(&assets, &b).map_err(|e| e.to_string()))
        {
            Ok(d) => {
                doc = d;
                path = Some(p.display().to_string());
            }
            Err(e) => {
                eprintln!("{}: {e}", p.display());
                return 2;
            }
        }
    }
    let mut rt = match runtime(o, &file) {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let size = if is_js(&file) { None } else { cmd::size_of(&src) }.unwrap_or((640, 433));
    let mut editor = Editor::new(assets, doc, size.0, size.1);
    editor.path = path;
    editor.files = Box::new(crate::files::PlatformFiles);
    editor.env.rng = chrono::Utc::now().timestamp().rem_euclid(1 << 32).unsigned_abs();
    // The link opens before the enhancements are on: the script is in
    // charge, nothing asks whether the widget's own scripts may run (they
    // don't).
    if let Some(l) = &args.link {
        editor.open_link(l);
    }
    editor.set_enhanced(crate::enhanced(args.window.settings_file.as_deref()));
    if let Err(e) = prepare(o, &mut editor, args) {
        eprintln!("{e}");
        return 2;
    }
    // Settled, as the comparison scripts start.
    if let Err(e) = editor.wait(500) {
        eprintln!("{e}");
        return 1;
    }
    match run_headless(rt.as_mut(), &file, &src, Caps::ALL, &mut editor) {
        Ok(r) if r.passed() => 0,
        Ok(r) => {
            if r.failures > 0 {
                let s = if r.failures == 1 { "" } else { "s" };
                eprintln!("{file}: {} check{s} failed", r.failures);
            }
            // An error stopping it was shown as it happened; one from the
            // host's waits (its time ran out) only here.
            if let Some(e) = r.errors.last().filter(|e| e.line == 0) {
                eprintln!("{e}");
            }
            1
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

/// A script running in the game's window, a frame at a time.
pub struct Windowed {
    rt: Box<dyn Runtime>,
    task: TaskId,
    file: String,
    done: bool,
    /// With `--watch`: the file, when it last changed, and when it was
    /// last looked at.
    watch: Option<(PathBuf, Option<SystemTime>, Instant)>,
}

impl Windowed {
    /// Starts the script `--script` names in `editor`, if there's one;
    /// the error if it can't.
    ///
    /// # Errors
    /// The file can't be read or doesn't read as a script.
    pub fn start(args: &crate::Args, editor: &mut Editor) -> Result<Option<Windowed>, String> {
        let o = &args.script;
        if o.file.is_none() {
            return Ok(None);
        }
        let (file, src) = read(o)?;
        prepare(o, editor, args)?;
        let mut rt = runtime(o, &file)?;
        let task = rt.start(&file, &src, Caps::ALL, editor).map_err(|e| e.to_string())?;
        let watch = o
            .file
            .as_ref()
            .filter(|_| o.watch)
            .map(|p| (p.clone(), modified(p), Instant::now()));
        Ok(Some(Windowed {
            rt,
            task,
            file,
            done: false,
            watch,
        }))
    }

    /// With `--watch`, the file saved since it last started: it starts
    /// again (what ran is stopped).
    fn again(&mut self, editor: &mut Editor) {
        let Some((path, seen, looked)) = &mut self.watch else {
            return;
        };
        if looked.elapsed() < WATCH_EVERY {
            return;
        }
        *looked = Instant::now();
        let now = modified(path);
        if now == *seen {
            return;
        }
        *seen = now;
        let Ok(src) = std::fs::read_to_string(&*path) else {
            return;
        };
        self.rt.cancel(None);
        editor.log(Level::Info, &format!("-- {}: saved; running it again --", self.file));
        match self.rt.start(&self.file, &src, Caps::ALL, editor) {
            Ok(t) => {
                self.task = t;
                self.done = false;
            }
            Err(e) => {
                editor.log(Level::Error, &e.to_string());
                self.done = true;
            }
        }
    }

    /// A frame of `ms` passed.
    pub fn frame(&mut self, editor: &mut Editor, ms: u64) {
        self.again(editor);
        let stop = editor.debug.take_stop_scripts();
        editor.debug.scripts_running = !self.done;
        if self.done {
            return;
        }
        // Debug ▸ Stop Scripts.
        if stop {
            self.rt.cancel(None);
        }
        self.rt.frame(editor, ms);
        if let Some(t) = self.rt.tasks().into_iter().find(|t| t.id == self.task)
            && matches!(t.state, TaskState::Done | TaskState::Failed)
        {
            self.done = true;
            let msg = match (t.state, t.failures) {
                (TaskState::Done, 0) => format!("{}: done", self.file),
                (_, 0) => format!("{}: stopped", self.file),
                (_, n) => format!("{}: {n} check{} failed", self.file, if n == 1 { "" } else { "s" }),
            };
            editor.log(Level::Info, &msg);
        }
    }
}
