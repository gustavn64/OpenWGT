//! The open parts: the port's own parts for every player, with nothing
//! but the game's own wires in and out (no network, no files): the
//! Oscilloscope ([`scope`]) and the scripted part ([`scripted`], whose
//! behaviour is a script's). Their pictures, dialogs and texts are the
//! port's own too ([`pack`]), there whether the game's data is a CD's or
//! OpenWDATA. They are one family of `crate::port_parts`, and need no
//! other family.

pub mod pack;
pub mod scope;
pub mod scripted;

use crate::gadgets::Class;
use crate::port_parts::{Family, TrayPart};

/// The open parts' classes, after the game's (`gadgets::class`).
pub static CLASSES: &[Class] = &[
    Class {
        tag: scope::SCOPE,
        name: "cOscilloscope",
        gadget_id: 2902,
        sheet: 1903,
        knots: 1903,
        make: scope::oscilloscope,
    },
    Class {
        tag: scripted::SCRIPTED,
        name: "cScriptedPart",
        gadget_id: 2903,
        sheet: scripted::SHEET,
        knots: scripted::SHEET,
        make: scripted::scripted_part,
    },
];

/// The sheet of their pictures in the tray: frame 1 the Oscilloscope's,
/// 2 the scripted part's.
pub const TRAY_SHEET: u32 = 1904;

/// The open parts in the tray: their tray indexes, after the game's 64
/// and the network parts' 65 and 66, tags and pictures. Their help lines
/// are strings `2500 +` the index, as the game's. The scripted part is
/// there only with the debugging tools on, which give it its script.
pub const TRAY: [TrayPart; 2] = [
    TrayPart {
        index: 67,
        tag: scope::SCOPE,
        picture: (TRAY_SHEET, 1),
        tools: false,
    },
    TrayPart {
        index: 68,
        tag: scripted::SCRIPTED,
        picture: (TRAY_SHEET, 2),
        tools: true,
    },
];

/// The Oscilloscope's folder, the Strip Chart's ("Display Parts"), where
/// it comes last while the port's enhancements are on.
pub const SCOPE_FOLDER: usize = 0;
const SCOPE_LIST: [i8; 1] = [67];
/// The scripted part's, the numbers' and logic's, where it comes last.
pub const SCRIPTED_FOLDER: usize = 3;
const SCRIPTED_LIST: [i8; 1] = [68];

/// The open parts as a family of the port's own parts.
pub static FAMILY: Family = Family {
    name: "openparts",
    classes: CLASSES,
    tray: &TRAY,
    folders: &[(SCOPE_FOLDER, &SCOPE_LIST), (SCRIPTED_FOLDER, &SCRIPTED_LIST)],
    pack: pack::builtin_for,
    taken: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_oscilloscope_is_in_the_strip_charts_folder() {
        let strip_chart = crate::editor::tray::index_of(crate::gadget::Tag::new(b"StCh")).unwrap();
        let folder = crate::editor::tray::FOLDERS[SCOPE_FOLDER];
        assert!(folder.contains(&i8::try_from(strip_chart).unwrap()));
        assert_eq!(crate::port_parts::tray_tag(67), Some(scope::SCOPE));
        assert_eq!(crate::port_parts::tray_index(scope::SCOPE), Some(67));
        assert_eq!(crate::port_parts::tray_picture(67), Some((1904, 1)));
        assert_eq!(crate::port_parts::tray_extra(SCOPE_FOLDER, false), [67]);
        let c = crate::gadgets::class(scope::SCOPE).unwrap();
        assert_eq!((c.gadget_id, c.sheet, c.knots), (2902, 1903, 1903));
    }

    #[test]
    fn the_scripted_part_is_last_in_the_numbers_folder_with_the_tools() {
        let totaler = crate::editor::tray::index_of(crate::gadget::Tag::new(b"Ttlr")).unwrap();
        let folder = crate::editor::tray::FOLDERS[SCRIPTED_FOLDER];
        assert!(folder.contains(&i8::try_from(totaler).unwrap()));
        assert_eq!(crate::port_parts::tray_tag(68), Some(scripted::SCRIPTED));
        assert_eq!(crate::port_parts::tray_picture(68), Some((1904, 2)));
        assert_eq!(crate::port_parts::tray_extra(SCRIPTED_FOLDER, true), [68]);
        assert_eq!(crate::port_parts::tray_extra(SCRIPTED_FOLDER, false), []);
        let c = crate::gadgets::class(scripted::SCRIPTED).unwrap();
        assert_eq!((c.gadget_id, c.sheet, c.knots), (2903, 1905, 1905));
    }
}
