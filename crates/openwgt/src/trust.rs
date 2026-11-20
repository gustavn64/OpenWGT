//! The scripts from other people's widgets the player let run
//! (`engine::script::trust`): their texts' SHA-256 in the settings file
//! (`platform::settings`), on one line, the newest last:
//!
//! ```text
//! trusted_scripts=3f2a…,9b07…
//! ```
//!
//! In the browser the settings file is in the browser's storage, so the
//! trust stays there.

use std::path::PathBuf;

use engine::script::trust::{Digest, Trust, add_to_list, parse_list};

/// The settings file's key.
pub const KEY: &str = "trusted_scripts";

/// The trust kept in the settings file at the path.
pub struct SettingsTrust(pub PathBuf);

impl Trust for SettingsTrust {
    fn trusted(&self, d: &Digest) -> bool {
        platform::settings::load_value(&self.0, KEY).is_some_and(|l| parse_list(&l).contains(d))
    }

    fn trust(&mut self, d: &Digest) {
        let list = platform::settings::load_value(&self.0, KEY).unwrap_or_default();
        if let Err(e) = platform::settings::save_value(&self.0, KEY, &add_to_list(&list, d)) {
            eprintln!("{}: {e}", self.0.display());
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn trust_is_kept_in_the_settings_file() {
        let dir = std::env::temp_dir().join(format!("openwgt-trust-{}", std::process::id()));
        let path = dir.join("settings.ini");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "ui_scale=2\n").unwrap();
        let (a, b) = (Digest::of("run();"), Digest::of("stop();"));
        let mut t = SettingsTrust(path.clone());
        assert!(!t.trusted(&a));
        t.trust(&a);
        t.trust(&b);
        let again = SettingsTrust(path.clone());
        assert!(again.trusted(&a) && again.trusted(&b));
        assert!(!again.trusted(&Digest::of("run(); ")));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("ui_scale=2\ntrusted_scripts="), "{text}");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
