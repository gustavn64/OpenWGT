//! Builds the port's own graphics, sounds and data (`opengfx`, `opensfx`,
//! `opendata`, `opennet`: the folders of the same names) into the program:
//! a table of every file under each, path and bytes.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn files(dir: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, base, out);
        } else if p.file_name().is_some_and(|n| n != "README.md") {
            let rel = p.strip_prefix(base).unwrap_or(&p);
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push((rel, p));
        }
    }
}

fn main() {
    for pack in ["opengfx", "opensfx", "opendata"] {
        embed(pack, "BUILTIN");
    }
    // The Swedish ones: OpenWGFX's pictures with words, OpenWDATA whole.
    for pack in ["opengfx-sv", "opendata-sv"] {
        embed(pack, "BUILTIN_SV");
    }
    // The French ones, the same.
    for pack in ["opengfx-fr", "opendata-fr"] {
        embed(pack, "BUILTIN_FR");
    }
    // The network parts' own (`net::pack`).
    embed("opennet", "BUILTIN");
}

/// Writes `$OUT_DIR/<pack>_files.rs`: the constant `name`, the files of
/// folder `pack` (none if there's no such folder).
fn embed(pack: &str, name: &str) {
    let dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(pack);
    // A folder: Cargo looks at everything in it.
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut found = Vec::new();
    files(&dir, &dir, &mut found);
    found.sort();
    let mut src = format!("/// The pack's files (`build.rs`).\nconst {name}: &[(&str, &[u8])] = &[\n");
    for (rel, path) in &found {
        let _ = writeln!(src, "    ({rel:?}, include_bytes!({:?})),", path.display().to_string());
    }
    src.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join(format!("{pack}_files.rs"));
    std::fs::write(out, src).unwrap();
}
