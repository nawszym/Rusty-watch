//! Język tekstów rysowanych na tarczy zegarka.
//!
//! Tarcza sama odczytuje język systemu przez `DeviceRuntimeCore.HmUtils.getLanguage()`
//! (tak jak oficjalny bootstrap `app.js`), z awaryjnym `hmSetting.getLanguage()`.
//! Zgodnie z mapą wielojęzyczności Zepp `9` to `pl-PL`; każdy inny język daje angielski.
//! Dzięki temu ten sam plik ZIP działa w obu wersjach językowych bez konfiguracji.
//! Blok jest emitowany wewnątrz IIFE `watchface/index.js`, nigdy na poziomie modułu.

/// Wartość zwracana przez `hmSetting.getLanguage()` dla `pl-PL`.
pub const POLISH_LANGUAGE_ID: i32 = 9;
/// Wartość używana, gdy API języka jest niedostępne (angielski).
pub const ENGLISH_LANGUAGE_ID: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchfaceLocale {
    English,
    Polish,
}

impl WatchfaceLocale {
    pub fn from_language_id(id: i32) -> Self {
        if id == POLISH_LANGUAGE_ID {
            Self::Polish
        } else {
            Self::English
        }
    }

    /// Kod używany przez most FFI (`"pl"` / `"en"`).
    pub fn from_code(code: &str) -> Self {
        if code.trim().eq_ignore_ascii_case("pl") || code.trim().eq_ignore_ascii_case("pl-pl") {
            Self::Polish
        } else {
            Self::English
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Polish => "pl",
        }
    }

    /// Nazwa miesiąca dla indeksu 0..11 (zgodnego z `Date.getMonth()`).
    pub fn month_name(self, index: usize) -> Option<&'static str> {
        self.months().get(index).copied()
    }

    /// Skróty nazw miesięcy używane przez widget daty.
    pub fn months(self) -> &'static [&'static str] {
        match self {
            Self::English => &[
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                "Dec",
            ],
            Self::Polish => &[
                "Sty", "Lut", "Mar", "Kwi", "Maj", "Cze", "Lip", "Sie", "Wrz", "Paź", "Lis",
                "Gru",
            ],
        }
    }

    /// Pełne nazwy miesięcy używane przez token `MMMM` wzorca daty.
    pub fn months_full(self) -> &'static [&'static str] {
        match self {
            Self::English => &[
                "January", "February", "March", "April", "May", "June", "July", "August",
                "September", "October", "November", "December",
            ],
            Self::Polish => &[
                "Styczeń", "Luty", "Marzec", "Kwiecień", "Maj", "Czerwiec", "Lipiec",
                "Sierpień", "Wrzesień", "Październik", "Listopad", "Grudzień",
            ],
        }
    }

    /// Skróty dni tygodnia, indeks 0 = niedziela (zgodnie z `Date.getDay()`).
    pub fn days(self) -> &'static [&'static str] {
        match self {
            Self::English => &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
            Self::Polish => &["Nie", "Pon", "Wt", "Śr", "Czw", "Pt", "Sob"],
        }
    }

    /// Pełne nazwy dni tygodnia, indeks 0 = niedziela.
    pub fn days_full(self) -> &'static [&'static str] {
        match self {
            Self::English => &[
                "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
            ],
            Self::Polish => &[
                "Niedziela", "Poniedziałek", "Wtorek", "Środa", "Czwartek", "Piątek", "Sobota",
            ],
        }
    }

    /// Oznaczenie przed południem dla tokenu `A` / `a`.
    pub fn am(self, upper: bool) -> &'static str {
        match (self, upper) {
            (Self::English, false) => "am",
            (Self::English, true) => "AM",
            (Self::Polish, false) => "przedp.",
            (Self::Polish, true) => "PRZEDP.",
        }
    }

    /// Oznaczenie po południu dla tokenu `A` / `a`.
    pub fn pm(self, upper: bool) -> &'static str {
        match (self, upper) {
            (Self::English, false) => "pm",
            (Self::English, true) => "PM",
            (Self::Polish, false) => "pop.",
            (Self::Polish, true) => "POP.",
        }
    }

    /// Blok JS wstawiany na początek wygenerowanego `watchface/index.js`.
    /// Zmienne `L` i `MONTHS` są potem używane przez wyrażenia czasu i daty.
    /// `typeof` i `try` chronią przed brakiem `hmSetting` w starszych runtime'ach.
    pub fn js_detection() -> &'static str {
        "function _watchLanguage() {\n\
         \x20 try {\n\
         \x20   if (typeof DeviceRuntimeCore !== 'undefined' && DeviceRuntimeCore.HmUtils &&\n\
         \x20       typeof DeviceRuntimeCore.HmUtils.getLanguage === 'function') {\n\
         \x20     return DeviceRuntimeCore.HmUtils.getLanguage()\n\
         \x20   }\n\
         \x20 } catch (e) {}\n\
         \x20 try {\n\
         \x20   if (typeof hmSetting !== 'undefined' && typeof hmSetting.getLanguage === 'function') return hmSetting.getLanguage()\n\
         \x20 } catch (e) {}\n\
         \x20 return 2\n\
         }\n\
         // pl-PL = 9 w mapie języków ZeppOS; każdy inny język => angielski.\n\
         const IS_POLISH = _watchLanguage() === 9\n\
         const L = IS_POLISH\n\
         \x20 ? { months: ['Sty','Lut','Mar','Kwi','Maj','Cze','Lip','Sie','Wrz','Paź','Lis','Gru'], monthsFull: ['Styczeń','Luty','Marzec','Kwiecień','Maj','Czerwiec','Lipiec','Sierpień','Wrzesień','Październik','Listopad','Grudzień'], days: ['Nie','Pon','Wt','Śr','Czw','Pt','Sob'], daysFull: ['Niedziela','Poniedziałek','Wtorek','Środa','Czwartek','Piątek','Sobota'], am: 'przedp.', amUp: 'PRZEDP.', pm: 'pop.', pmUp: 'POP.' }\n\
         \x20 : { months: ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'], monthsFull: ['January','February','March','April','May','June','July','August','September','October','November','December'], days: ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'], daysFull: ['Sunday','Monday','Tuesday','Wednesday','Thursday','Friday','Saturday'], am: 'am', amUp: 'AM', pm: 'pm', pmUp: 'PM' }\n\
         const MONTHS = L.months\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_zepp_language_ids() {
        assert_eq!(WatchfaceLocale::from_language_id(9), WatchfaceLocale::Polish);
        assert_eq!(WatchfaceLocale::from_language_id(2), WatchfaceLocale::English);
        assert_eq!(WatchfaceLocale::from_language_id(0), WatchfaceLocale::English);
        assert_eq!(WatchfaceLocale::from_language_id(99), WatchfaceLocale::English);
        assert_eq!(WatchfaceLocale::from_code("pl-PL"), WatchfaceLocale::Polish);
        assert_eq!(WatchfaceLocale::from_code("de"), WatchfaceLocale::English);
    }

    #[test]
    fn provides_both_language_tables() {
        let en = WatchfaceLocale::English;
        let pl = WatchfaceLocale::Polish;
        assert_eq!(en.months()[0], "Jan");
        assert_eq!(pl.months()[0], "Sty");
        assert_eq!(pl.months()[9], "Paź");
        assert_eq!(pl.month_name(10), Some("Lis"));
        assert_eq!(pl.month_name(12), None);
        assert_eq!(en.am(true), "AM");
        assert_eq!(en.pm(false), "pm");
        assert_eq!(pl.am(true), "PRZEDP.");
        assert_eq!(pl.pm(false), "pop.");
        assert_eq!(pl.code(), "pl");
    }

    #[test]
    fn js_detection_uses_classic_api_and_polish_id() {
        let js = WatchfaceLocale::js_detection();
        assert!(js.contains("hmSetting.getLanguage()"));
        assert!(js.contains("=== 9"));
        assert!(js.contains("'Paź'"));
        assert!(js.contains("amUp: 'PRZEDP.'"));
        assert!(js.contains("const MONTHS = L.months"));
    }
}
