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
}

impl Lang {
    /// Every language, in the order a list offers them.
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Sv];

    /// Its ISO 639-1 code: `en`, `sv`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Sv => "sv",
        }
    }

    /// Its name in itself, for a list to choose from.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Sv => "Svenska",
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
    /// (both CDs' are language-neutral), else MFC's word for Cancel in its
    /// string table (string `0xf221`: "Cancel", "Avbryt").
    #[must_use]
    pub fn of_resources(r: &formats::pe::Resources) -> Option<Lang> {
        r.language
            .map(Lang::from_langid)
            .or_else(|| match r.string(0xf221)?.trim().to_lowercase().as_str() {
                "cancel" => Some(Lang::En),
                "avbryt" => Some(Lang::Sv),
                _ => None,
            })
    }

    /// The language of a Windows `LANGID` (its primary language, the low
    /// 10 bits: `0x1d` Swedish); English for any other.
    #[must_use]
    pub fn from_langid(id: u16) -> Lang {
        match id & 0x3ff {
            0x1d => Lang::Sv,
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
        },
        Ordering::Relaxed,
    );
}

/// The current language.
#[must_use]
pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Sv,
        _ => Lang::En,
    }
}

/// `en` in the current language.
#[must_use]
pub fn tr(en: &'static str) -> &'static str {
    match current() {
        Lang::En => en,
        Lang::Sv => sv(en).unwrap_or(en),
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
        "Download widget" => "Ladda ned uppfinning",
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
        "&Index" => "In&dex",
        "&Back" => "&Bakåt",
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
        "&Automatic (the display's)" => "&Automatisk (skärmens)",
        "&Enhancements" => "&Förbättringar",
        "The parts tray shows a whole folder at once; windows drag whole." => {
            "Verktygslådan visar en hel mapp åt gången; fönster dras hela."
        }
        "&Full screen" => "&Helskärm",
        "&Change CD..." => "&Byt skiva...",
        "Close" => "Stäng",
        // The port's File menu, Options menu.
        "Save to Do&wnload" => "Spara som ne&dladdning",
        "&Download a Widget..." => "&Ladda ned en uppfinning...",
        "Open Widget &Link\tCtrl+V" => "Öppna län&k till uppfinning\tCtrl+V",
        "Copy Lin&k to Widget" => "Ko&piera länk till uppfinningen",
        "&Settings..." => "&Inställningar...",
        // Links and downloads.
        "A link to {} is on the clipboard ({} characters)." => "En länk till {} ligger i Urklipp ({} tecken).",
        "the widget" => "uppfinningen",
        "The link could not be copied." => "Länken kunde inte kopieras.",
        "The clipboard holds no link to a widget." => "Urklipp har ingen länk till en uppfinning.",
        "That link holds no widget." => "Länken har ingen uppfinning i sig.",
        "The widget could not be downloaded." => "Uppfinningen kunde inte laddas ned.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_and_words() {
        assert_eq!(Lang::from_code("sv-SE"), Some(Lang::Sv));
        assert_eq!(Lang::from_code("sv_SE.UTF-8"), Some(Lang::Sv));
        assert_eq!(Lang::from_code("EN"), Some(Lang::En));
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::from_langid(0x041d), Lang::Sv);
        assert_eq!(Lang::from_langid(0x0409), Lang::En);
        assert_eq!(sv("Cancel"), Some("Avbryt"));
        assert_eq!(sv("no such words"), None);
    }
}
