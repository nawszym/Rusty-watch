//! Wzorzec wyświetlania daty współdzielony przez podgląd (UI) i generator JS.
//!
//! Wielkość litery w tokenie steruje wielkością wyświetlanej nazwy:
//! - `D` / `d` — dzień miesiąca, `DD` / `dd` — z wiodącym zerem,
//! - `M` / `m` — numer miesiąca, `MM` / `mm` — z wiodącym zerem,
//! - `MMM` — skrót miesiąca („Jan"), `mmm` — małymi literami („jan"),
//!   `MMMM` / `mmmm` — pełna nazwa („January" / „january"),
//! - `DDD` — skrót dnia tygodnia („Mon"), `ddd` — małymi literami („mon"),
//!   `DDDD` / `dddd` — pełna nazwa („Monday" / „monday"),
//! - 5 lub 6 powtórzeń litery daje WIELKIE LITERY: `MMMMM`/`DDDDD` — skrót,
//!   `MMMMMM`/`DDDDDD` — pełna nazwa,
//! - `YY` / `yy` — dwucyfrowy rok, `YYYY` / `yyyy` — czterocyfrowy,
//! - `\` przed znakiem wymusza literal.
//!
//! Miesiąc jest 0-based (zgodnie z `Date.getMonth()`), dzień tygodnia 0-based
//! od niedzieli (zgodnie z `Date.getDay()`) — tak samo jak w wygenerowanym JS.
//! Nazwy pochodzą z tabel `L` wybieranych w runtime przez `hmSetting.getLanguage()`.
//!
//! Uwaga: `M` oznacza tu miesiąc — w `time_format.rs` ten sam token oznacza
//! minuty. To osobne wzorce, parser wybierany jest na podstawie `WidgetKind`.

use anyhow::{bail, Context, Result};

use crate::locale::WatchfaceLocale;
use crate::model::FirstDayOfWeek;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameCase {
    /// Tak jak w tabeli języka: pierwsza litera wielka.
    Capitalized,
    Lower,
    Upper,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Day { pad: bool },
    MonthNumber { pad: bool },
    MonthName { full: bool, case: NameCase },
    WeekdayName { full: bool, case: NameCase },
    Year2,
    Year4,
}

pub const MAX_PATTERN_LEN: usize = 32;
/// Domyślny wzorzec nowego widgetu daty — odpowiada dawnemu zachowaniu `Lis 23`.
pub const DEFAULT_PATTERN: &str = "MMM D";

/// 0 = niedziela, zgodnie z `Date.getDay()`. Algorytm Sakamoto dla dat
/// gregoriańskich; JS wyznacza dzień tygodnia z `Date`, więc oba widoki się zgadzają.
fn weekday_index(year: i32, month: u32, day: u32) -> usize {
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let month = (month.min(11)) as usize;
    let mut y = year;
    if month < 2 {
        y -= 1;
    }
    let index = y + y / 4 - y / 100 + y / 400 + OFFSETS[month] + day as i32;
    index.rem_euclid(7) as usize
}

fn name_case(upper: bool) -> NameCase {
    if upper {
        NameCase::Capitalized
    } else {
        NameCase::Lower
    }
}

pub fn parse(pattern: &str) -> Result<Vec<Segment>> {
    if pattern.is_empty() {
        bail!("Format daty nie może być pusty");
    }
    if pattern.chars().count() > MAX_PATTERN_LEN {
        bail!("Format daty może mieć najwyżej {MAX_PATTERN_LEN} znaków");
    }
    if pattern.chars().any(|c| c.is_control()) {
        bail!("Format daty nie może zawierać znaków kontrolnych");
    }

    let chars: Vec<char> = pattern.chars().collect();
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut has_token = false;
    let mut index = 0;

    let flush = |segments: &mut Vec<Segment>, literal: &mut String| {
        if !literal.is_empty() {
            segments.push(Segment::Literal(std::mem::take(literal)));
        }
    };

    while index < chars.len() {
        let c = chars[index];
        if matches!(c, 'M' | 'm' | 'D' | 'd' | 'Y' | 'y') {
            let mut len = 1;
            while index + len < chars.len() && chars[index + len] == c {
                len += 1;
            }
            let case = name_case(c.is_ascii_uppercase());
            let segment = match (c.to_ascii_lowercase(), len) {
                ('m', 1) => Some(Segment::MonthNumber { pad: false }),
                ('m', 2) => Some(Segment::MonthNumber { pad: true }),
                ('m', 3) => Some(Segment::MonthName { full: false, case }),
                ('m', 4) => Some(Segment::MonthName { full: true, case }),
                ('m', 5) => Some(Segment::MonthName {
                    full: false,
                    case: NameCase::Upper,
                }),
                ('m', 6) => Some(Segment::MonthName {
                    full: true,
                    case: NameCase::Upper,
                }),
                ('d', 1) => Some(Segment::Day { pad: false }),
                ('d', 2) => Some(Segment::Day { pad: true }),
                ('d', 3) => Some(Segment::WeekdayName { full: false, case }),
                ('d', 4) => Some(Segment::WeekdayName { full: true, case }),
                ('d', 5) => Some(Segment::WeekdayName {
                    full: false,
                    case: NameCase::Upper,
                }),
                ('d', 6) => Some(Segment::WeekdayName {
                    full: true,
                    case: NameCase::Upper,
                }),
                ('y', 2) => Some(Segment::Year2),
                ('y', 4) => Some(Segment::Year4),
                _ => None,
            };
            if let Some(segment) = segment {
                flush(&mut segments, &mut literal);
                segments.push(segment);
                has_token = true;
                index += len;
                continue;
            }
            // Nieprawidłowa długość ciągu (np. pojedyncze `Y`) — dalej potraktujemy
            // pierwszy znak jako literal.
        }

        if c == '\\' {
            let escaped = chars
                .get(index + 1)
                .copied()
                .context("Format daty kończy się niedokończonym znakiem ucieczki")?;
            literal.push(escaped);
            index += 2;
            continue;
        }

        literal.push(c);
        index += 1;
    }

    flush(&mut segments, &mut literal);
    if !has_token {
        bail!("Format daty musi zawierać przynajmniej jeden token (D, M lub Y)");
    }
    Ok(segments)
}

fn padded(value: u32, pad: bool) -> String {
    if pad {
        format!("{value:02}")
    } else {
        value.to_string()
    }
}

fn apply_case(text: &str, case: NameCase) -> String {
    match case {
        NameCase::Capitalized => text.to_string(),
        NameCase::Lower => text.to_lowercase(),
        NameCase::Upper => text.to_uppercase(),
    }
}

/// Formatuje datę zgodnie ze wzorcem (angielskie nazwy).
pub fn format(pattern: &str, day: u32, month: u32, year: i32) -> Result<String> {
    format_with_locale(pattern, day, month, year, WatchfaceLocale::English)
}

/// Formatuje datę z nazwami miesięcy i dni tygodnia w języku podglądu tarczy.
pub fn format_with_locale(
    pattern: &str,
    day: u32,
    month: u32,
    year: i32,
    locale: WatchfaceLocale,
) -> Result<String> {
    let month_index = (month.min(11)) as usize;
    let weekday_index = weekday_index(year, month, day);
    let mut out = String::new();
    for segment in parse(pattern)? {
        match segment {
            Segment::Literal(text) => out.push_str(&text),
            Segment::Day { pad } => out.push_str(&padded(day, pad)),
            Segment::MonthNumber { pad } => out.push_str(&padded(month + 1, pad)),
            Segment::MonthName { full, case } => {
                let names = if full {
                    locale.months_full()
                } else {
                    locale.months()
                };
                let name = names
                    .get(month_index)
                    .copied()
                    .context("indeks miesiąca poza zakresem 0..11")?;
                out.push_str(&apply_case(name, case));
            }
            Segment::WeekdayName { full, case } => {
                let names = if full {
                    locale.days_full()
                } else {
                    locale.days()
                };
                let name = names
                    .get(weekday_index)
                    .copied()
                    .context("indeks dnia tygodnia poza zakresem 0..6")?;
                out.push_str(&apply_case(name, case));
            }
            Segment::Year2 => out.push_str(&padded(year.rem_euclid(100) as u32, true)),
            Segment::Year4 => out.push_str(&format!("{:04}", year.rem_euclid(10_000))),
        }
    }
    Ok(out)
}

fn js_literal(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("'{escaped}'")
}

fn js_number(value: &str, pad: bool) -> String {
    if pad {
        format!("p2({value})")
    } else {
        value.to_string()
    }
}

fn js_case(expression: String, case: NameCase) -> String {
    match case {
        NameCase::Capitalized => expression,
        NameCase::Lower => format!("{expression}.toLowerCase()"),
        NameCase::Upper => format!("{expression}.toUpperCase()"),
    }
}

/// Buduje wyrażenie JS zwracające sformatowaną datę. `date_expr` to wyrażenie
/// zwracające obiekt `Date` (np. `d1`). Nazwy miesięcy i dni tygodnia pochodzą
/// z obiektu `L`, więc wielkość liter w wzorcu daje identyczny efekt jak w podglądzie.
pub fn js_expression_localized(pattern: &str, date_expr: &str) -> Result<String> {
    let mut parts = Vec::new();
    for segment in parse(pattern)? {
        let part = match segment {
            Segment::Literal(text) => js_literal(&text),
            Segment::Day { pad } => js_number(&format!("({date_expr}.getDate())"), pad),
            Segment::MonthNumber { pad } => {
                js_number(&format!("({date_expr}.getMonth() + 1)"), pad)
            }
            Segment::MonthName { full, case } => {
                let table = if full { "L.monthsFull" } else { "L.months" };
                js_case(format!("{table}[{date_expr}.getMonth()]"), case)
            }
            Segment::WeekdayName { full, case } => {
                let table = if full { "L.daysFull" } else { "L.days" };
                js_case(format!("{table}[{date_expr}.getDay()]"), case)
            }
            Segment::Year2 => format!("p2(({date_expr}.getFullYear() % 100))"),
            Segment::Year4 => format!("({date_expr}.getFullYear())"),
        };
        parts.push(part);
    }
    Ok(parts.join(" + "))
}

/// Pomocnicze funkcje JS dla pól daty liczonych z `Date` — ten sam algorytm
/// co `iso_week()` / `day_of_year()` w Rust, żeby podgląd i tarcza się zgadzały.
pub const JS_ISO_WEEK: &str = "function _isoWeek() {\n\
  const _d = new Date()\n\
  const _t = new Date(Date.UTC(_d.getFullYear(), _d.getMonth(), _d.getDate()))\n\
  const _wd = _t.getUTCDay() || 7\n\
  _t.setUTCDate(_t.getUTCDate() + 4 - _wd)\n\
  const _ys = new Date(Date.UTC(_t.getUTCFullYear(), 0, 1))\n\
  return Math.ceil((((_t - _ys) / 86400000) + 1) / 7)\n\
}\n";

pub const JS_DAY_OF_YEAR: &str = "function _dayOfYear() {\n\
  const _d = new Date()\n\
  return Math.floor((_d - new Date(_d.getFullYear(), 0, 0)) / 86400000)\n\
}\n";

/// Numer dnia tygodnia dla pola „Dzień tygodnia (liczba)".
/// `month` jest 1-based (od FFI). Domyślnie 1 = poniedziałek … 7 = niedziela;
/// przy `Sunday` obowiązuje 1 = niedziela … 7 = sobota.
pub fn weekday_number(day: u32, month: u32, year: i32, first_day: FirstDayOfWeek) -> u32 {
    // `weekday_index` jest 0-based od niedzieli, zgodnie z `Date.getDay()`.
    let sunday_based = weekday_index(year, month.saturating_sub(1), day) as u32;
    match first_day {
        FirstDayOfWeek::Monday => ((sunday_based + 6) % 7) + 1,
        FirstDayOfWeek::Sunday => sunday_based + 1,
    }
}

/// Indeks dnia tygodnia liczony w JS tym samym algorytmem Sakamoto co `weekday_index`.
/// Zamiast polegać na `hmSensor.id.TIME.week` (dokumentacja ZeppOS podaje 1 = poniedziałek
/// dla API 1.0, a w nowszych wersjach tylko zakres 1–7), tarcza liczy dzień z pól
/// `year`/`month`/`day` czujnika TIME — dzięki temu podgląd i eksport są zawsze 1:1.
pub const JS_WEEKDAY_INDEX: &str = "function _weekdayIndex(y, m, d) {\n\
  const _offsets = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4]\n\
  const _mm = Math.max(1, Math.min(m, 12)) - 1\n\
  const _year = _mm < 2 ? y - 1 : y\n\
  const _index = _year + Math.floor(_year / 4) - Math.floor(_year / 100) + Math.floor(_year / 400) + _offsets[_mm] + d\n\
  return ((_index % 7) + 7) % 7\n\
}\n";

/// Buduje funkcję JS zwracającą numer dnia tygodnia z obiektu czujnika TIME.
pub fn js_weekday_number(first_day: FirstDayOfWeek) -> String {
    let body = match first_day {
        FirstDayOfWeek::Monday => "return ((_w + 6) % 7) + 1",
        FirstDayOfWeek::Sunday => "return _w + 1",
    };
    format!(
        "function _weekdayNumber(_t) {{\n  if (!_t) return '--'\n  const _w = _weekdayIndex(_t.year, _t.month, _t.day)\n  {body}\n}}\n"
    )
}

/// Dzień roku (1..366). Używany przez podgląd widgetu „Dzień roku".
pub fn day_of_year(day: u32, month: u32, year: i32) -> u32 {
    const DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let mut total = day.min(31);
    for index in 0..(month.saturating_sub(1).min(12) as usize) {
        total += DAYS[index];
        if index == 1 && leap {
            total += 1;
        }
    }
    total
}

/// Numer tygodnia ISO (1..53), zgodny z algorytmem użytym w generowanym JS.
pub fn iso_week(day: u32, month: u32, year: i32) -> u32 {
    let doy = day_of_year(day, month, year) as i32;
    // Dzień tygodnia: 1 = poniedziałek … 7 = niedziela.
    let weekday = (weekday_index(year, month.saturating_sub(1), day) as i32 + 6) % 7 + 1;
    let mut week = (doy - weekday + 10) / 7;
    if week < 1 {
        // Dni przed pierwszym poniedziałkiem należą do ostatniego tygodnia roku poprzedniego.
        let prev_year = year - 1;
        let prev_dec31_weekday = (weekday_index(prev_year, 11, 31) as i32 + 6) % 7 + 1;
        week = if prev_dec31_weekday == 4
            || (prev_dec31_weekday == 3 && is_leap(prev_year))
        {
            53
        } else {
            52
        };
    } else {
        let dec31_weekday = (weekday_index(year, 11, 31) as i32 + 6) % 7 + 1;
        if week == 53 && !(dec31_weekday == 4 || (dec31_weekday == 3 && is_leap(year))) {
            week = 1;
        }
    }
    week as u32
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-11-03 to wtorek (Tuesday).
    #[test]
    fn formats_common_patterns() {
        assert_eq!(format("MMM D", 3, 10, 2026).unwrap(), "Nov 3");
        assert_eq!(format("DD.MM.YYYY", 3, 10, 2026).unwrap(), "03.11.2026");
        assert_eq!(format("D/M/yy", 3, 10, 2026).unwrap(), "3/11/26");
        assert_eq!(format("MMMM D, YYYY", 3, 0, 2026).unwrap(), "January 3, 2026");
        assert_eq!(format("DD\\日", 3, 0, 2026).unwrap(), "03日");
        assert_eq!(
            format("DDD, D MMM YYYY", 3, 10, 2026).unwrap(),
            "Tue, 3 Nov 2026"
        );
    }

    #[test]
    fn token_case_controls_name_case() {
        assert_eq!(
            format("MMM mmm MMMM mmmm MMMMM MMMMMM", 3, 10, 2026).unwrap(),
            "Nov nov November november NOV NOVEMBER"
        );
        assert_eq!(
            format("DDD ddd DDDD dddd DDDDD DDDDDD", 3, 10, 2026).unwrap(),
            "Tue tue Tuesday tuesday TUE TUESDAY"
        );
        // Małe litery działają też dla tokenów liczbowych.
        assert_eq!(format("dd.mm.yyyy", 3, 10, 2026).unwrap(), "03.11.2026");
    }

    #[test]
    fn formats_polish_names_and_weekday() {
        // 2026-01-01 to czwartek.
        assert_eq!(
            format_with_locale("dddd, D mmmm YYYY", 1, 0, 2026, WatchfaceLocale::Polish).unwrap(),
            "czwartek, 1 styczeń 2026"
        );
        assert_eq!(
            format_with_locale("DDD, D MMM", 1, 0, 2026, WatchfaceLocale::Polish).unwrap(),
            "Czw, 1 Sty"
        );
        assert_eq!(
            format_with_locale("DDDDDD", 1, 0, 2026, WatchfaceLocale::Polish).unwrap(),
            "CZWARTEK"
        );
    }

    #[test]
    fn rejects_invalid_patterns() {
        assert!(parse("").is_err());
        assert!(parse("abc").is_err());
        assert!(parse("DD\\").is_err());
        assert!(parse(&"&".repeat(MAX_PATTERN_LEN + 1)).is_err());
    }

    #[test]
    fn builds_localized_js_expression() {
        assert_eq!(
            js_expression_localized("MMM D", "d1").unwrap(),
            "L.months[d1.getMonth()] + ' ' + (d1.getDate())"
        );
        assert_eq!(
            js_expression_localized("DD.MM.YYYY", "d").unwrap(),
            "p2((d.getDate())) + '.' + p2((d.getMonth() + 1)) + '.' + (d.getFullYear())"
        );
        assert_eq!(
            js_expression_localized("mmm dddd", "d").unwrap(),
            "L.months[d.getMonth()].toLowerCase() + ' ' + L.daysFull[d.getDay()].toLowerCase()"
        );
        assert_eq!(
            js_expression_localized("DDD MMMMMM", "d").unwrap(),
            "L.days[d.getDay()] + ' ' + L.monthsFull[d.getMonth()].toUpperCase()"
        );
    }
}
