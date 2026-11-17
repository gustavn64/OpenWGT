//! Settings ▸ Language without a CD: the editor on the port's own packs
//! switched to another language in place ([`Editor::set_assets`]).

use super::*;
use crate::assets::Packs;
use crate::gadget::Tag;
use crate::lang::Lang;

/// The port's own packs in `lang`, as the host builds them without a CD.
fn packs(lang: Lang) -> Assets {
    let graphics = crate::opengfx::builtin_for(lang);
    let sounds = crate::opensfx::builtin();
    let data = crate::opendata::builtin_for(lang);
    let packs = Packs {
        graphics: Some(graphics.as_ref()),
        sounds: Some(&sounds),
        data: Some(&data),
    };
    let mut assets = Assets::load_with(None, packs, None).unwrap();
    assets.lang = lang;
    assets
}

/// The menu bar's titles.
fn titles(e: &Editor) -> Vec<String> {
    e.menu
        .items
        .iter()
        .filter_map(|i| match i {
            formats::dialog::MenuItem::Popup { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The widget as Save writes it.
fn saved(e: &mut Editor) -> Vec<u8> {
    e.doc.tray_limits = e.tray.limits();
    e.doc.save()
}

/// The language button's letter pressed, and the host's part: the packs
/// in the language it asks for handed over.
fn next_language(e: &mut Editor, letter: u8) -> Lang {
    e.key_down(u16::from(letter));
    let lang = e.take_change_lang().expect("a language to switch to");
    e.set_assets(packs(lang));
    e.update(16, (0, 0, 0));
    lang
}

#[test]
fn the_widget_stays_as_the_language_changes() {
    let _lang = crate::lang::hold();
    let mut e = Editor::new(packs(Lang::En), Document::new(), 800, 600);
    let id = e.doc.new_id();
    let label = doc::new_gadget(&e.assets, Tag(*b"TxLb"), id, 120, 200);
    e.doc.gadgets.push(label);
    e.snapshot();
    let id = e.doc.new_id();
    let lamp = doc::new_gadget(&e.assets, Tag(*b"bulb"), id, 300, 220);
    e.doc.gadgets.push(lamp);
    e.dirty = true;
    e.path = Some("/somewhere/MINE.WGT".to_owned());
    let before = saved(&mut e);
    let menu_en = titles(&e);
    e.command(cmd::SETTINGS);
    let at = e.settings.as_ref().unwrap().window();
    // No "Save changes?": the switch is at once, the window still open.
    assert_eq!(next_language(&mut e, b'L'), Lang::Sv);
    assert!(e.prompt.is_none());
    assert_eq!(crate::lang::current(), Lang::Sv);
    assert_eq!(e.assets.lang, Lang::Sv);
    let b = e.settings.as_ref().expect("the Settings window stays");
    assert_eq!(b.lang, Lang::Sv);
    assert_eq!((b.window().x0, b.window().y0), (at.x0, at.y0));
    let menu_sv = titles(&e);
    assert_ne!(menu_sv, menu_en, "the menu in Swedish");
    assert!(
        e.menu.items.iter().any(|i| matches!(
            i,
            formats::dialog::MenuItem::Popup { items, .. }
                if items.iter().any(|c| matches!(c, formats::dialog::MenuItem::Command { text, .. } if text == "&Inställningar..."))
        )),
        "the port's own items too"
    );
    // The widget, its name, its modified flag and its undo step stay.
    assert_eq!(saved(&mut e), before);
    assert_eq!(e.path.as_deref(), Some("/somewhere/MINE.WGT"));
    assert!(e.dirty);
    assert_eq!(e.title(), "OpenWGT - MINE.WGT");
    assert!(e.can_undo());
    // Swedish's letter for the button is S; on to French and back.
    assert_eq!(next_language(&mut e, b'S'), Lang::Fr);
    assert_eq!(crate::lang::current(), Lang::Fr);
    let menu_fr = titles(&e);
    assert!(menu_fr != menu_en && menu_fr != menu_sv);
    assert_eq!(next_language(&mut e, b'L'), Lang::En);
    assert_eq!(titles(&e), menu_en);
    assert_eq!(saved(&mut e), before);
    e.key_down(VK_ESCAPE);
    assert!(e.settings.is_none());
    assert_eq!(e.take_change_lang(), None, "closing changes nothing");
    // The undo step is the one from before the switches.
    e.undo();
    assert_eq!(e.doc.gadgets.len(), 1);
}

#[test]
fn a_puzzle_comes_again_in_the_new_language() {
    let _lang = crate::lang::hold();
    let mut e = Editor::new(packs(Lang::En), Document::new(), 800, 600);
    assert!(e.load_puzzle(3));
    let english = saved(&mut e);
    let mut f = Editor::new(packs(Lang::Fr), Document::new(), 800, 600);
    assert!(f.load_puzzle(3));
    let french = saved(&mut f);
    assert_ne!(english, french, "the puzzles' words differ");
    crate::lang::set(Lang::En);
    e.command(cmd::SETTINGS);
    next_language(&mut e, b'L');
    assert_eq!(next_language(&mut e, b'S'), Lang::Fr);
    assert_eq!(e.puzzle(), Some(3));
    assert_eq!(saved(&mut e), french);
    assert_eq!(e.title(), f.title());
    // Undo brings back the puzzle as it was before the last switch.
    e.key_down(VK_ESCAPE);
    e.undo();
    let mut s = Editor::new(packs(Lang::Sv), Document::new(), 800, 600);
    assert!(s.load_puzzle(3));
    assert_eq!(saved(&mut e), saved(&mut s));
}

#[test]
fn the_help_follows_the_language() {
    let _lang = crate::lang::hold();
    let mut e = Editor::new(packs(Lang::En), Document::new(), 800, 600);
    e.command(cmd::CONTENTS);
    let english = e.help.as_ref().expect("the help opens").topic().title.clone();
    e.command(cmd::SETTINGS);
    next_language(&mut e, b'L');
    let swedish = e.help.as_ref().expect("the help stays open").topic().title.clone();
    assert_ne!(english, swedish);
}
