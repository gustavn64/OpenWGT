//! The example scripts of `examples/scripts/`, run headless as their
//! comments and `docs/scripting.md` say, in English and in Swedish (the
//! sample widgets' labels are in the language): each passes.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::process::Command;

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/scripts")
}

fn run(file: &str, lang: &str) {
    let dir = std::env::temp_dir().join(format!("openwgt-examples-{}-{lang}-{file}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_openwgt"))
        .args([
            "--open",
            &format!("--lang={lang}"),
            "--no-audio",
            "--headless",
            "--script",
        ])
        .arg(examples().join(file))
        .arg("--out")
        .arg(&dir)
        .current_dir(&dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("HOME", &dir)
        .env("APPDATA", &dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{file} ({lang}): {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if file == "egg-timer.txt" {
        assert!(dir.join("egg-timer.png").is_file());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_command_examples_pass() {
    for lang in ["en", "sv"] {
        run("adder-test.txt", lang);
        run("egg-timer.txt", lang);
    }
}

#[test]
#[cfg(feature = "js")]
fn the_javascript_examples_pass() {
    for lang in ["en", "sv"] {
        run("adder-test.js", lang);
    }
}
