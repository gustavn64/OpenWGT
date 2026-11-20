//! The open parts: the port's own parts for every player, with nothing
//! but the game's own wires in and out (no network, no files): so far
//! the Oscilloscope ([`scope`]). Their pictures, dialogs and texts are the
//! port's own too ([`pack`]), there whether the game's data is a CD's or
//! OpenWDATA. They are one family of `crate::port_parts`, and need no
//! other family.

pub mod pack;
pub mod scope;

use crate::gadgets::Class;
use crate::port_parts::{Family, TrayPart};

/// The open parts' classes, after the game's (`gadgets::class`).
pub static CLASSES: &[Class] = &[Class {
    tag: scope::SCOPE,
    name: "cOscilloscope",
    gadget_id: 2902,
    sheet: 1903,
    knots: 1903,
    make: scope::oscilloscope,
}];

/// The sheet of their pictures in the tray: frame 1 the Oscilloscope's.
pub const TRAY_SHEET: u32 = 1904;

/// The open parts in the tray: their tray indexes, after the game's 64
/// and the network parts' 65 and 66, tags and pictures. Their help lines
/// are strings `2500 +` the index, as the game's.
pub const TRAY: [TrayPart; 1] = [TrayPart {
    index: 67,
    tag: scope::SCOPE,
    picture: (TRAY_SHEET, 1),
}];

/// The Oscilloscope's folder, the Strip Chart's ("Display Parts"), where
/// it comes last while the port's enhancements are on.
pub const SCOPE_FOLDER: usize = 0;
const SCOPE_LIST: [i8; 1] = [67];

/// The open parts as a family of the port's own parts.
pub static FAMILY: Family = Family {
    name: "openparts",
    classes: CLASSES,
    tray: &TRAY,
    folders: &[(SCOPE_FOLDER, &SCOPE_LIST)],
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
        assert_eq!(crate::port_parts::tray_extra(SCOPE_FOLDER), [67]);
        let c = crate::gadgets::class(scope::SCOPE).unwrap();
        assert_eq!((c.gadget_id, c.sheet, c.knots), (2902, 1903, 1903));
    }
}
