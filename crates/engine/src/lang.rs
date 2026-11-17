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
        "Import from self-running widget" => "Importera från fristående uppfinning",
        "Save Into Stand Alone Widget" => "Spara i fristående uppfinning",
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
            "Verktygslådan visar en hel mapp åt gången; fönster dras hela."
        }
        "&Full screen" => "&Helskärm",
        "&Change CD..." => "&Byt cd...",
        "&Language:" => "&Språk:",
        "Language: the CD's ({})" => "Språk: skivans ({})",
        "Close" => "Stäng",
        // The port's File menu, Options menu.
        "Save to Do&wnload" => "Spara för ne&dladdning",
        "&Download a Widget..." => "&Ladda ner en uppfinning...",
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

/// The French of the port's own words: the French edition's (Eurêka, le
/// Laboratoire du savant fou) where its code has them, Windows' French for
/// what Windows said, the port's own for the rest. The game says "tu".
fn fr(en: &str) -> Option<&'static str> {
    Some(match en {
        // Message boxes (Windows' buttons).
        "&Yes" => "&Oui",
        "&No" => "&Non",
        "&Retry" => "&Réessayer",
        "Cancel" => "Annuler",
        // The common file dialogs.
        "Load widget" => "Charger Eurêka!",
        "Save Widget as" => "Enregistrer l'Eurêka sous",
        "Download widget" => "Télécharger un Eurêka",
        "Import from self-running widget" => "Importer un Eurêka indépendant",
        "Save Into Stand Alone Widget" => "Sauvegarder en logiciel indépendant",
        "Import from Wave format file" => "Importer du fichier Wave",
        "Saved widgets (*.wgt)" => "Eurêkas sauvegardés (*.wgt)",
        "Self running widgets (*.exe)" => "Eurêkas indépendants (*.exe)",
        "Wave files (*.wav)" => "Fichiers Wave (*.wav)",
        "Save &in:" => "Enregistrer &dans :",
        "Look &in:" => "Rechercher &dans :",
        "File &name:" => "&Nom de fichier :",
        "Save as &type:" => "&Type :",
        "Files of &type:" => "&Type de fichiers :",
        "&Save" => "&Enregistrer",
        "&Open" => "&Ouvrir",
        "Widget Workshop CD" => "CD de Widget Workshop",
        "untitled.wgt" => "sansnom.wgt",
        // WinHelp.
        "Can't find help file\nIt should be in the same folder as Widget Workshop and be named WIDGET.HLP" => {
            "Impossible de trouver le fichier aide\nWIDGET.HLP"
        }
        "&Contents" => "&Sommaire",
        "&Index" => "&Index",
        "&Back" => "&Précédent",
        "&Print" => "&Imprimer",
        "Windows Help" => "Aide de Windows",
        "Help Topics: {}" => "Rubriques d'aide : {}",
        "&1  Type the first few letters of the word you're looking for." => {
            "&1  Tapez les premières lettres du mot recherché."
        }
        "&2  Click the index entry you want, and then click Display." => {
            "&2  Cliquez sur l'entrée voulue, puis sur Afficher."
        }
        "&Display" => "&Afficher",
        // The port's Settings window.
        "Settings" => "Paramètres",
        "UI scale" => "Échelle",
        "&Automatic (the display's)" => "&Automatique (celle de l'écran)",
        "&Enhancements" => "A&méliorations",
        "The parts tray shows a whole folder at once; windows drag whole." => {
            "Un dossier de pièces entier à la fois ; fenêtres déplacées entières."
        }
        "&Full screen" => "&Plein écran",
        "&Change CD..." => "&Changer de CD...",
        "&Language:" => "&Langue :",
        "Language: the CD's ({})" => "Langue : celle du CD ({})",
        "Close" => "Fermer",
        // The port's File menu, Options menu.
        "Save to Do&wnload" => "Enregistrer pour télé&chargement",
        "&Download a Widget..." => "&Télécharger un Eurêka...",
        "Open Widget &Link\tCtrl+V" => "Ouvrir un &lien d'Eurêka\tCtrl+V",
        "Copy Lin&k to Widget" => "Copier le lien &vers l'Eurêka",
        "&Settings..." => "&Paramètres...",
        // Links and downloads.
        "A link to {} is on the clipboard ({} characters)." => {
            "Un lien vers {} est dans le presse-papiers ({} caractères)."
        }
        "the widget" => "l'Eurêka",
        "The link could not be copied." => "Le lien n'a pas pu être copié.",
        "The clipboard holds no link to a widget." => "Le presse-papiers ne contient aucun lien vers un Eurêka.",
        "That link holds no widget." => "Ce lien ne contient aucun Eurêka.",
        "The widget could not be downloaded." => "L'Eurêka n'a pas pu être téléchargé.",
        // The parts' first words, the original's code's.
        "Label" => "Étiquette",
        "Message One" => "Message Un",
        "Message Two" => "Message Deux",
        "Message Three" => "Message Trois",
        "Message Four" => "Message Quatre",
        // The game's own messages the code has (not its resources).
        "Save changes to {}?" => "Enregistrer les modifications apportées à {}?",
        "{} already exists.\nDo you want to replace it?" => "{} existe déjà.\nVeux-tu le remplacer?",
        "{}\nPath not found.\nPlease verify the correct path was given." => {
            "{}\nChemin introuvable.\nVérifie que le chemin indiqué est correct."
        }
        "{}\nFile not found.\nPlease verify the correct file name was given." => {
            "{}\nFichier introuvable.\nVérifie que le nom de fichier indiqué est correct."
        }
        "Nothing to fit" => "Rien ne correspond",
        "Your widget is already fit to any Windows screen" => {
            "Ton Eurêka a déjà la taille idéale pour un écran Windows."
        }
        "Do you really want to mark all puzzles as unsolved?\nThis is a permanent change\n" => {
            "Veux-tu que tous les problèmes soient non résolus?\nCeci est une modification permanente.\n"
        }
        "Can't find file: WIDGET.SLF\nIt must be in the same directory as Widget Workshop\n" => {
            "Impossible de trouver fichier: WIDGET.SLF\nIl doit être dans le même répertoire qu'Eurêka\n"
        }
        "There was an error while recording sound" => "Une erreur s'est produite pendant l'enregistrement du son",
        "Can`t open file" => "Impossible d'ouvrir le fichier",
        "Bad file format" => "Mauvais format de fichier",
        "Can't import non-8 bit sound" => "Impossible d'importer un son qui ne soit pas 8 bits",
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
