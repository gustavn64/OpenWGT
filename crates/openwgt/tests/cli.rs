//! `openwgt --headless --script FILE`: the exit codes and the output, on
//! the port's own packs (no CD needed).

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A fresh folder for one test's files.
fn folder(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("openwgt-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Runs `script` headless in `dir` with `extra` options.
fn run(dir: &Path, script: &str, extra: &[&str]) -> Output {
    let file = dir.join("test.txt");
    std::fs::write(&file, script).unwrap();
    Command::new(env!("CARGO_BIN_EXE_openwgt"))
        .args([
            "--open",
            "--lang=en",
            "--no-audio",
            "--headless",
            "--seed",
            "1",
            "--script",
        ])
        .arg(&file)
        .arg("--out")
        .arg(dir.join("out"))
        .args(extra)
        .current_dir(dir)
        // The player's settings file isn't read.
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("HOME", dir)
        .env("APPDATA", dir)
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// A Switch and a Light Bulb placed from the tray and wired, by mouse.
const SETUP: &str = "\
click 330 53
wait 400
drag 160 92 200 260
wait 400
click 148 53
wait 400
drag 176 90 360 250
wait 400
drag 214 259 359 275
";

#[test]
fn a_script_that_passes_exits_with_0() {
    let dir = folder("pass");
    let out = run(
        &dir,
        &format!(
            "{SETUP}name LightBulb Lamp\nrun\nset Switch on\nuntil Lamp == on max 10\nexpect Lamp\nlog lamp {{Lamp}}\nshot lit\n"
        ),
        &[],
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("lamp on"), "{}", text(&out.stdout));
    let png = std::fs::read(dir.join("out/lit.png")).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_expect_exits_with_1() {
    let dir = folder("fail");
    let out = run(
        &dir,
        &format!("{SETUP}run\nexpect LightBulb == on\nlog still running\n"),
        &[],
    );
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(
        err.contains("test.txt:11:8  expected LightBulb == on; LightBulb is off"),
        "{err}"
    );
    assert!(err.contains("1 check failed"), "{err}");
    // An expect doesn't stop the script.
    assert!(text(&out.stdout).contains("still running"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn errors_exit_with_1_and_say_where() {
    let dir = folder("error");
    let out = run(&dir, "run\npress Lamb\n", &[]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("test.txt:2:7  no part called \"Lamb\""), "{err}");
    assert!(err.contains(" 2 | press Lamb\n   |       ^^^^"), "{err}");
    // A syntax error: nothing runs.
    let out = run(&dir, "log first\nfly away\n", &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("test.txt:2:1  no command fly"));
    assert!(!text(&out.stdout).contains("first"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_timeout_stops_a_long_wait() {
    let dir = folder("timeout");
    let out = run(
        &dir,
        &format!("{SETUP}run\nuntil LightBulb == on max 100000000\n"),
        &["--timeout", "0.5"],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("ran out of time"), "{}", text(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_script_exits_with_2() {
    let dir = folder("missing");
    let out = Command::new(env!("CARGO_BIN_EXE_openwgt"))
        .args(["--open", "--lang=en", "--no-audio", "--headless", "--script"])
        .arg(dir.join("nothing.txt"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("HOME", &dir)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}
