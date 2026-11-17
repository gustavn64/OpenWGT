//! The language of the port's own words: what the original took from
//! Windows (message boxes' buttons, the common file dialogs, `WinHelp`) or
//! what the port adds (its Settings window, links, downloads). The
//! game's own texts come from its resources, the CD's or OpenWDATA's, in
//! their language; these follow it ([`set`], from the CD's resources'
//! language or the pack chosen).
//!
//! [`tr`] takes the English text, as written in the code, and gives it in
//! the current language; a text it has no translation for stays English.

use std::sync::atomic::{AtomicU8, Ordering};

/// A language the port speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Sv,
    Fr,
}

impl Lang {
    /// Every language, in the order a list offers them.
    pub const ALL: [Lang; 3] = [Lang::En, Lang::Sv, Lang::Fr];

    /// Its ISO 639-1 code: `en`, `sv`, `fr`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Sv => "sv",
            Lang::Fr => "fr",
        }
    }

    /// Its name in itself, for a list to choose from.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Sv => "svenska",
            Lang::Fr => "français",
        }
    }

    /// The language of a code or locale (`sv`, `sv-SE`, `sv_SE.UTF-8`,
    /// any case); `None` for one the port doesn't speak.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Lang> {
        let primary = code.split(['-', '_', '.', '@']).next()?.to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == primary)
    }

    /// The language of a game's resources: their `LANGID` if they have one
    /// (the CDs' are language-neutral), else MFC's word for Cancel in its
    /// string table (string `0xf221`: "Cancel", "Avbryt", "Annuler").
    #[must_use]
    pub fn of_resources(r: &formats::pe::Resources) -> Option<Lang> {
        r.language
            .map(Lang::from_langid)
            .or_else(|| match r.string(0xf221)?.trim().to_lowercase().as_str() {
                "cancel" => Some(Lang::En),
                "avbryt" => Some(Lang::Sv),
                "annuler" => Some(Lang::Fr),
                _ => None,
            })
    }

    /// The language of a Windows `LANGID` (its primary language, the low
    /// 10 bits: `0x1d` Swedish, `0x0c` French); English for any other.
    #[must_use]
    pub fn from_langid(id: u16) -> Lang {
        match id & 0x3ff {
            0x1d => Lang::Sv,
            0x0c => Lang::Fr,
            _ => Lang::En,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Makes `lang` the language of the port's own words from now on.
pub fn set(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::En => 0,
            Lang::Sv => 1,
            Lang::Fr => 2,
        },
        Ordering::Relaxed,
    );
}

/// The current language.
#[must_use]
pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Sv,
        2 => Lang::Fr,
        _ => Lang::En,
    }
}

/// `en` in the current language.
#[must_use]
pub fn tr(en: &'static str) -> &'static str {
    match current() {
        Lang::En => en,
        Lang::Sv => sv(en).unwrap_or(en),
        Lang::Fr => fr(en).unwrap_or(en),
    }
}

/// `en` in the current language with each `{}` replaced by the next of
/// `args`.
#[must_use]
pub fn trf(en: &'static str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let mut rest = tr(en);
    while let Some(i) = rest.find("{}") {
        out.push_str(&rest[..i]);
        out.push_str(args.next().copied().unwrap_or_default());
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

/// The Swedish of the port's own words.
#[allow(clippy::too_many_lines)]
fn sv(en: &str) -> Option<&'static str> {
    Some(match en {
        // Message boxes (Windows' buttons).
        "&Yes" => "&Ja",
        "&No" => "&Nej",
        "&Retry" => "&Försök igen",
        "Cancel" => "Avbryt",
        // The common file dialogs.
        "Load widget" => "Öppna uppfinning",
        "Save Widget as" => "Spara uppfinningen som",
        "Download widget" => "Ladda ner uppfinning",
        "Import from self-running widget" => "Importera fristående uppfinning",
        "Save Into Stand Alone Widget" => "Spara som fristående uppfinning",
        "Import from Wave format file" => "Importera från ljudfil (Wave)",
        "Saved widgets (*.wgt)" => "Sparade uppfinningar (*.wgt)",
        "Self running widgets (*.exe)" => "Fristående uppfinningar (*.exe)",
        "Wave files (*.wav)" => "Ljudfiler (*.wav)",
        "Save &in:" => "Spara &i:",
        "Look &in:" => "Leta &i:",
        "File &name:" => "Fil&namn:",
        "Save as &type:" => "Spara som fil&typ:",
        "Files of &type:" => "Fil&typ:",
        "&Save" => "&Spara",
        "&Open" => "&Öppna",
        "Widget Workshop CD" => "Widget Workshop-skivan",
        "untitled.wgt" => "namnlös.wgt",
        // WinHelp.
        "Can't find help file\nIt should be in the same folder as Widget Workshop and be named WIDGET.HLP" => {
            "Hittar inte hjälpfilen\nDen ska ligga i samma mapp som Widget Workshop och heta WIDGET.HLP"
        }
        "&Contents" => "&Innehåll",
        "&Index" => "&Register",
        "&Back" => "&Tillbaka",
        "&Print" => "Skriv &ut",
        "Windows Help" => "Windows Hjälp",
        "Help Topics: {}" => "Hjälpavsnitt: {}",
        "&1  Type the first few letters of the word you're looking for." => {
            "&1  Skriv de första bokstäverna i ordet du letar efter."
        }
        "&2  Click the index entry you want, and then click Display." => {
            "&2  Klicka på det ord du vill se och sedan på Visa."
        }
        "&Display" => "&Visa",
        // The port's Settings window.
        "Settings" => "Inställningar",
        "UI scale" => "Skala",
        "&Automatic (the display's)" => "&Automatiskt (skärmens)",
        "&Enhancements" => "&Förbättringar",
        "The parts tray shows a whole folder at once; windows drag whole." => {
            "Verktygslådan visar en hel flik åt gången; fönster dras hela."
        }
        "&Full screen" => "&Helskärm",
        "&Change CD..." => "&Byt skiva...",
        "Play &without the CD" => "Spela &utan skivan",
        "&Language:" => "&Språk:",
        "Language: the CD's ({})" => "Språk: skivans ({})",
        "Close" => "Stäng",
        // The port's File menu, Options menu.
        "Save to Do&wnload" => "Spara för ne&dladdning",
        "&Download a Widget..." => "Ladda ner en &uppfinning...",
        "Open Widget &Link\tCtrl+V" => "Öppna uppfinnings&länk\tCtrl+V",
        "Copy Lin&k to Widget" => "Ko&piera länk till uppfinningen",
        "&Settings..." => "&Inställningar...",
        // Links and downloads.
        "A link to {} is on the clipboard ({} characters)." => "En länk till {} ligger i Urklipp ({} tecken).",
        "the widget" => "uppfinningen",
        "The link could not be copied." => "Länken kunde inte kopieras.",
        "The clipboard holds no link to a widget." => "Urklipp har ingen länk till en uppfinning.",
        "That link holds no widget." => "Länken har ingen uppfinning i sig.",
        "The widget could not be downloaded." => "Uppfinningen kunde inte laddas ner.",
        // Choosing the CD natively (the widget crate's `pick`), and
        // starting again.
        "Choose the Widget Workshop CD image" => "Välj en skivavbildning av Widget Workshop",
        "CD image" => "Skivavbildning",
        "All files" => "Alla filer",
        "That isn't Widget Workshop's CD image ({}). Choose another." => {
            "Det är ingen skivavbildning av Widget Workshop ({}). Välj en annan."
        }
        "{} isn't Widget Workshop's CD image ({}). Drop another." => {
            "{} är ingen skivavbildning av Widget Workshop ({}). Släpp en annan."
        }
        "no WWGADGET.DLL on it" => "WWGADGET.DLL saknas på den",
        "Drop the game's CD image (.iso) on this window, or a folder with its files." => {
            "Släpp spelets skivavbildning (.iso) på det här fönstret, eller en mapp med skivans filer."
        }
        "There's no file dialog here to choose it with (no xdg-desktop-portal, no zenity)." => {
            "Här finns ingen fildialog att välja den med (varken xdg-desktop-portal eller zenity)."
        }
        "Esc goes back to the game." => "Esc går tillbaka till spelet.",
        "It can be given on the command line too. Closing the window quits." => {
            "Den kan också anges på kommandoraden. Stäng fönstret för att avsluta."
        }
        "OpenWGT didn't start again: {}" => "OpenWGT kunde inte starta om: {}",
        "OpenWGT stopped: {}" => "OpenWGT stannade: {}",
        // The browser's start (the widget crate's `web`).
        "Reading the CD image…" => "Läser skivavbildningen…",
        "The CD image didn't read: {}." => "Skivavbildningen gick inte att läsa: {}.",
        "That CD image isn't Widget Workshop's: it has no WWGADGET.DLL." => {
            "Den skivavbildningen är inte Widget Workshops: WWGADGET.DLL saknas."
        }
        "That file doesn't open as a CD image: {}." => "Filen går inte att öppna som skivavbildning: {}.",
        // The parts' first words, the original's code's.
        "Label" => "Etikett",
        "Message One" => "Meddelande ett",
        "Message Two" => "Meddelande två",
        "Message Three" => "Meddelande tre",
        "Message Four" => "Meddelande fyra",
        // The game's own messages the code has (not its resources).
        "Save changes to {}?" => "Spara ändringarna i {}?",
        "{} already exists.\nDo you want to replace it?" => "{} finns redan.\nVill du ersätta den?",
        "{}\nPath not found.\nPlease verify the correct path was given." => {
            "{}\nSökvägen hittades inte.\nKontrollera att sökvägen är rätt."
        }
        "{}\nFile not found.\nPlease verify the correct file name was given." => {
            "{}\nFilen hittades inte.\nKontrollera att filnamnet är rätt."
        }
        "Nothing to fit" => "Inget att anpassa",
        "Your widget is already fit to any Windows screen" => "Din uppfinning får redan plats på alla skärmar",
        "Do you really want to mark all puzzles as unsolved?\nThis is a permanent change\n" => {
            "Vill du verkligen markera alla pussel som olösta?\nDet går inte att ångra\n"
        }
        "Can't find file: WIDGET.SLF\nIt must be in the same directory as Widget Workshop\n" => {
            "Hittar inte filen WIDGET.SLF\nDen måste ligga i samma mapp som Widget Workshop\n"
        }
        "There was an error while recording sound" => "Ett fel uppstod när ljudet spelades in",
        "Can`t open file" => "Kan inte öppna filen",
        "Bad file format" => "Felaktigt filformat",
        "Can't import non-8 bit sound" => "Kan bara importera 8-bitars ljud",
        _ => return None,
    })
}

/// The French of the port's own words, in its own French as the Swedish
/// is its own Swedish (the French edition's sentences aren't copied):
/// Windows' French for the plain words Windows said, the French
/// OpenWDATA's terms (an Eurêka, the Presse-papiers, enregistrer). The
/// game says "tu"; a no-break space (`\u{a0}`) goes before `:`, `;`,
/// `!` and `?`, so that a line never starts with one.
#[allow(clippy::too_many_lines)]
fn fr(en: &str) -> Option<&'static str> {
    Some(match en {
        // Message boxes (Windows' buttons).
        "&Yes" => "&Oui",
        "&No" => "&Non",
        "&Retry" => "&Réessayer",
        "Cancel" => "Annuler",
        // The common file dialogs.
        "Load widget" => "Ouvrir un Eurêka",
        "Save Widget as" => "Enregistrer ton Eurêka sous",
        "Download widget" => "Télécharger un Eurêka",
        "Import from self-running widget" => "Importer un Eurêka indépendant",
        "Save Into Stand Alone Widget" => "Enregistrer en Eurêka indépendant",
        "Import from Wave format file" => "Importer un fichier son (Wave)",
        "Saved widgets (*.wgt)" => "Eurêkas enregistrés (*.wgt)",
        "Self running widgets (*.exe)" => "Eurêkas indépendants (*.exe)",
        "Wave files (*.wav)" => "Fichiers son (*.wav)",
        "Save &in:" => "Enregistrer &dans\u{a0}:",
        "Look &in:" => "Rechercher &dans\u{a0}:",
        "File &name:" => "&Nom de fichier\u{a0}:",
        "Save as &type:" => "&Type\u{a0}:",
        "Files of &type:" => "&Type de fichiers\u{a0}:",
        "&Save" => "&Enregistrer",
        "&Open" => "&Ouvrir",
        "Widget Workshop CD" => "CD de Widget Workshop",
        "untitled.wgt" => "sansnom.wgt",
        // WinHelp.
        "Can't find help file\nIt should be in the same folder as Widget Workshop and be named WIDGET.HLP" => {
            "Le fichier d'aide est introuvable.\nIl doit se trouver dans le même dossier que Widget Workshop et s'appeler WIDGET.HLP"
        }
        "&Contents" => "&Sommaire",
        "&Index" => "&Index",
        "&Back" => "&Précédent",
        "&Print" => "&Imprimer",
        "Windows Help" => "Aide de Windows",
        "Help Topics: {}" => "Rubriques d'aide\u{a0}: {}",
        "&1  Type the first few letters of the word you're looking for." => {
            "&1  Tape les premières lettres du mot que tu cherches."
        }
        "&2  Click the index entry you want, and then click Display." => {
            "&2  Clique sur le mot voulu, puis sur Afficher."
        }
        "&Display" => "&Afficher",
        // The port's Settings window.
        "Settings" => "Paramètres",
        "UI scale" => "Échelle",
        "&Automatic (the display's)" => "&Automatique (celle de l'écran)",
        "&Enhancements" => "A&méliorations",
        "The parts tray shows a whole folder at once; windows drag whole." => {
            "La fenêtre à outils montre tout un dossier\u{a0}; les fenêtres glissent en entier."
        }
        "&Full screen" => "&Plein écran",
        "&Change CD..." => "&Changer de CD...",
        "Play &without the CD" => "Jouer &sans le CD",
        "&Language:" => "&Langue\u{a0}:",
        "Language: the CD's ({})" => "Langue\u{a0}: celle du CD ({})",
        "Close" => "Fermer",
        // The port's File menu, Options menu.
        "Save to Do&wnload" => "Enregistrer pour télé&chargement",
        "&Download a Widget..." => "&Télécharger un Eurêka...",
        "Open Widget &Link\tCtrl+V" => "Ouvrir un &lien d'Eurêka\tCtrl+V",
        "Copy Lin&k to Widget" => "Copier le lien &vers l'Eurêka",
        "&Settings..." => "&Paramètres...",
        // Links and downloads.
        "A link to {} is on the clipboard ({} characters)." => {
            "Un lien vers {} est dans le Presse-papiers ({} caractères)."
        }
        "the widget" => "l'Eurêka",
        "The link could not be copied." => "Le lien n'a pas pu être copié.",
        "The clipboard holds no link to a widget." => "Le Presse-papiers ne contient aucun lien vers un Eurêka.",
        "That link holds no widget." => "Ce lien ne contient aucun Eurêka.",
        "The widget could not be downloaded." => "L'Eurêka n'a pas pu être téléchargé.",
        // Choosing the CD natively (the widget crate's `pick`), and
        // starting again.
        "Choose the Widget Workshop CD image" => "Choisis une image du CD de Widget Workshop",
        "CD image" => "Image de CD",
        "All files" => "Tous les fichiers",
        "That isn't Widget Workshop's CD image ({}). Choose another." => {
            "Ce n'est pas une image du CD de Widget Workshop ({}). Choisis-en une autre."
        }
        "{} isn't Widget Workshop's CD image ({}). Drop another." => {
            "{} n'est pas une image du CD de Widget Workshop ({}). Dépose-en une autre."
        }
        "no WWGADGET.DLL on it" => "WWGADGET.DLL n'y est pas",
        "Drop the game's CD image (.iso) on this window, or a folder with its files." => {
            "Dépose l'image du CD du jeu (.iso) sur cette fenêtre, ou un dossier avec ses fichiers."
        }
        "There's no file dialog here to choose it with (no xdg-desktop-portal, no zenity)." => {
            "Il n'y a pas ici de fenêtre pour la choisir (ni xdg-desktop-portal, ni zenity)."
        }
        "Esc goes back to the game." => "Échap ramène au jeu.",
        "It can be given on the command line too. Closing the window quits." => {
            "On peut aussi la donner sur la ligne de commande. Fermer la fenêtre quitte le jeu."
        }
        "OpenWGT didn't start again: {}" => "OpenWGT n'a pas pu redémarrer\u{a0}: {}",
        "OpenWGT stopped: {}" => "OpenWGT s'est arrêté\u{a0}: {}",
        // The browser's start (the widget crate's `web`).
        "Reading the CD image…" => "Lecture de l'image du CD…",
        "The CD image didn't read: {}." => "L'image du CD n'a pas pu être lue\u{a0}: {}.",
        "That CD image isn't Widget Workshop's: it has no WWGADGET.DLL." => {
            "Cette image n'est pas celle du CD de Widget Workshop\u{a0}: WWGADGET.DLL n'y est pas."
        }
        "That file doesn't open as a CD image: {}." => "Ce fichier ne s'ouvre pas comme une image de CD\u{a0}: {}.",
        // The parts' first words, the original's code's.
        "Label" => "Étiquette",
        "Message One" => "Message un",
        "Message Two" => "Message deux",
        "Message Three" => "Message trois",
        "Message Four" => "Message quatre",
        // The game's own messages the code has (not its resources).
        "Save changes to {}?" => "Enregistrer les modifications de {}\u{a0}?",
        "{} already exists.\nDo you want to replace it?" => "{} existe déjà.\nVeux-tu le remplacer\u{a0}?",
        "{}\nPath not found.\nPlease verify the correct path was given." => {
            "{}\nCe dossier n'existe pas.\nVérifie le chemin indiqué."
        }
        "{}\nFile not found.\nPlease verify the correct file name was given." => {
            "{}\nCe fichier n'existe pas.\nVérifie le nom de fichier indiqué."
        }
        "Nothing to fit" => "Rien à ajuster",
        "Your widget is already fit to any Windows screen" => "Ton Eurêka tient déjà sur tous les écrans",
        "Do you really want to mark all puzzles as unsolved?\nThis is a permanent change\n" => {
            "Veux-tu vraiment marquer tous les problèmes comme non résolus\u{a0}?\nTu ne pourras pas revenir en arrière\n"
        }
        "Can't find file: WIDGET.SLF\nIt must be in the same directory as Widget Workshop\n" => {
            "Le fichier WIDGET.SLF est introuvable\nIl doit se trouver dans le même dossier que Widget Workshop\n"
        }
        "There was an error while recording sound" => "Quelque chose s'est mal passé pendant l'enregistrement du son",
        "Can`t open file" => "Ce fichier ne peut pas être ouvert",
        "Bad file format" => "Ce fichier n'a pas le bon format",
        "Can't import non-8 bit sound" => "Seuls les sons 8 bits peuvent être importés",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_and_words() {
        assert_eq!(Lang::from_code("sv-SE"), Some(Lang::Sv));
        assert_eq!(Lang::from_code("sv_SE.UTF-8"), Some(Lang::Sv));
        assert_eq!(Lang::from_code("EN"), Some(Lang::En));
        assert_eq!(Lang::from_code("fr-CA"), Some(Lang::Fr));
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::from_langid(0x040c), Lang::Fr);
        assert_eq!(Lang::from_langid(0x041d), Lang::Sv);
        assert_eq!(Lang::from_langid(0x0409), Lang::En);
        assert_eq!(sv("Cancel"), Some("Avbryt"));
        assert_eq!(sv("no such words"), None);
        assert_eq!(fr("Cancel"), Some("Annuler"));
    }
}

#[cfg(test)]
mod every_word {
    use super::*;

    /// Every Swedish word has a French one and the other way round, with
    /// the same `{}` and line breaks.
    #[test]
    fn swedish_and_french_cover_the_same_words() {
        let src = include_str!("lang.rs");
        let keys = |f: &str| -> Vec<String> {
            let body = &src[src.find(&format!("fn {f}(en: &str)")).unwrap()..];
            let body = &body[..body.find("_ => return None").unwrap()];
            body.lines()
                .filter_map(|l| {
                    l.trim()
                        .strip_prefix('"')
                        .and_then(|l| l.split_once("\" =>"))
                        .map(|(k, _)| k.to_owned())
                })
                .collect()
        };
        let (sv_keys, fr_keys) = (keys("sv"), keys("fr"));
        assert_eq!(sv_keys, fr_keys);
        for k in &sv_keys {
            let en: &'static str = Box::leak(k.replace("\\n", "\n").replace("\\t", "\t").into_boxed_str());
            for t in [sv(en), fr(en)].into_iter().flatten() {
                assert_eq!(t.matches("{}").count(), en.matches("{}").count(), "{en:?}");
            }
            assert!(sv(en).is_some() && fr(en).is_some(), "{en:?}");
        }
    }
}
