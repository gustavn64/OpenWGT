//! The player's own text editor for the script editor's texts (natively,
//! `engine::editor::debug::External`): a script written to a file in the
//! temporary folder's `openwgt-scripts` and opened there, and read again
//! each time it's saved. The editor is `$OPENWGT_EDITOR` (a command, the
//! file added at its end) if it's set, else the system's: `xdg-open`,
//! `open -t` on macOS, Notepad on Windows (which would run a `.js` file
//! rather than open it).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use engine::editor::debug::External;

/// How often a file is looked at.
const EVERY: Duration = Duration::from_millis(300);

/// Each file opened: when it was last changed, and last looked at.
type Seen = HashMap<String, (Option<SystemTime>, Instant)>;

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// The command that opens `path` in `editor`, else the system's.
fn command(path: &Path, editor: Option<&str>) -> Command {
    if let Some(cmd) = editor {
        let mut words = cmd.split_whitespace();
        let mut c = Command::new(words.next().unwrap_or_default());
        c.args(words).arg(path);
        return c;
    }
    let (program, args): (&str, &[&str]) = if cfg!(target_os = "windows") {
        ("notepad", &[])
    } else if cfg!(target_os = "macos") {
        ("open", &["-t"])
    } else {
        ("xdg-open", &[])
    };
    let mut c = Command::new(program);
    c.args(args).arg(path);
    c
}

/// The player's editor, as the script editor reaches it.
#[must_use]
pub fn external() -> External {
    let editor = std::env::var("OPENWGT_EDITOR").ok().filter(|c| !c.trim().is_empty());
    with_editor(editor)
}

/// `editor` (a command), else the system's, as the script editor reaches
/// it.
fn with_editor(editor: Option<String>) -> External {
    let seen: Rc<RefCell<Seen>> = Rc::default();
    let opened = seen.clone();
    External {
        open: Box::new(move |name, text| {
            let dir = std::env::temp_dir().join("openwgt-scripts");
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let path: PathBuf = dir.join(name);
            std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
            let key = path.display().to_string();
            opened
                .borrow_mut()
                .insert(key.clone(), (modified(&path), Instant::now()));
            let mut c = command(&path, editor.as_deref());
            c.spawn()
                .map_err(|e| format!("{}: {e}", c.get_program().to_string_lossy()))?;
            Ok(key)
        }),
        changed: Box::new(move |file| {
            let mut seen = seen.borrow_mut();
            let (was, looked) = seen.get_mut(file)?;
            if looked.elapsed() < EVERY {
                return None;
            }
            *looked = Instant::now();
            let path = Path::new(file);
            let now = modified(path);
            if now == *was {
                return None;
            }
            *was = now;
            std::fs::read_to_string(path).ok()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn a_saved_file_comes_back() {
        // `true` as the editor: it opens nothing.
        let mut x = with_editor(Some("true".into()));
        let file = (x.open)("openwgt-test-part.js", "one").unwrap();
        assert_eq!((x.changed)(&file), None, "not changed yet");
        std::thread::sleep(EVERY + Duration::from_millis(20));
        let later = SystemTime::now() + Duration::from_secs(2);
        std::fs::write(&file, "two").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert_eq!((x.changed)(&file).as_deref(), Some("two"));
        std::thread::sleep(EVERY + Duration::from_millis(20));
        assert_eq!((x.changed)(&file), None, "read once");
        let _ = std::fs::remove_file(&file);
    }
}
