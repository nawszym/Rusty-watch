//! Wzorzec wyświetlania czasu współdzielony przez podgląd (UI) i generator JS.
//!
//! Tokeny: `HH`/`H` (24h), `hh`/`h` (12h), `MM`/`M`, `SS`/`S`, `A`/`a` (AM/PM).
//! Wszystko inne traktowane jest jako literal. `\` przed znakiem wymusza literal.

use anyhow::{bail, Result};

use crate::locale::WatchfaceLocale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Hour24 { pad: bool },
    Hour12 { pad: bool },
    Minute { pad: bool },
    Second { pad: bool },
    AmPm { upper: bool },
}

pub const MAX_PATTERN_LEN: usize = 32;

pub fn parse(pattern: &str) -> Result<Vec<Segment>> {
    if pattern.is_empty() {
        bail!("Format czasu nie może być pusty");
    }
    if pattern.chars().count() > MAX_PATTERN_LEN {
        bail!("Format czasu może mieć najwyżej {MAX_PATTERN_LEN} znaków");
    }
    if pattern.chars().any(|c| c.is_control()) {
        bail!("Format czasu nie może zawierać znaków kontrolnych");
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
        let next = chars.get(index + 1).copied();
        let token = match c {
            'H' => Some(Segment::Hour24 {
                pad: next == Some('H'),
            }),
            'h' => Some(Segment::Hour12 {
                pad: next == Some('h'),
            }),
            'M' | 'm' => Some(Segment::Minute {
                pad: next == Some(c),
            }),
            'S' | 's' => Some(Segment::Second {
                pad: next == Some(c),
            }),
            'A' => Some(Segment::AmPm { upper: true }),
            'a' => Some(Segment::AmPm { upper: false }),
            _ => None,
        };

        if token.is_some() {
            flush(&mut segments, &mut literal);
            let segment = token.unwrap();
            let consumes_two = matches!(
                segment,
                Segment::Hour24 { pad: true }
                    | Segment::Hour12 { pad: true }
                    | Segment::Minute { pad: true }
                    | Segment::Second { pad: true }
            );
            index += if consumes_two { 2 } else { 1 };
            segments.push(segment);
            has_token = true;
            continue;
        }

        if c == '\\' {
            if let Some(escaped) = next {
                literal.push(escaped);
                index += 2;
                continue;
            }
            bail!("Format czasu kończy się niedokończonym znakiem ucieczki");
        }

        literal.push(c);
        index += 1;
    }

    flush(&mut segments, &mut literal);
    if !has_token {
        bail!("Format czasu musi zawierać przynajmniej jeden token (H, M, S lub A)");
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

/// Formatuje czas zgodnie ze wzorcem po angielsku (domyślny podgląd i testy).
pub fn format(pattern: &str, hour: u32, minute: u32, second: u32) -> Result<String> {
    format_with_locale(pattern, hour, minute, second, WatchfaceLocale::English)
}

/// Formatuje czas z wybranym językiem oznaczeń AM/PM. Podgląd edytora używa tego
/// samego wzorca co generator JS, więc wybór języka podglądu daje 1:1 z tarczą.
pub fn format_with_locale(
    pattern: &str,
    hour: u32,
    minute: u32,
    second: u32,
    locale: WatchfaceLocale,
) -> Result<String> {
    let mut out = String::new();
    for segment in parse(pattern)? {
        match segment {
            Segment::Literal(text) => out.push_str(&text),
            Segment::Hour24 { pad } => out.push_str(&padded(hour, pad)),
            Segment::Hour12 { pad } => {
                let hour12 = if hour % 12 == 0 { 12 } else { hour % 12 };
                out.push_str(&padded(hour12, pad));
            }
            Segment::Minute { pad } => out.push_str(&padded(minute, pad)),
            Segment::Second { pad } => out.push_str(&padded(second, pad)),
            Segment::AmPm { upper } => {
                out.push_str(if hour < 12 {
                    locale.am(upper)
                } else {
                    locale.pm(upper)
                });
            }
        }
    }
    Ok(out)
}

pub fn has_seconds(pattern: &str) -> bool {
    parse(pattern)
        .map(|segments| segments.iter().any(|s| matches!(s, Segment::Second { .. })))
        .unwrap_or(false)
}

fn js_literal(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("'{escaped}'")
}

fn js_expression_with_am_pm<F>(
    pattern: &str,
    hour_expr: &str,
    minute_expr: &str,
    second_expr: &str,
    labels: F,
) -> Result<String>
where
    F: Fn(bool) -> (String, String),
{
    let mut parts = Vec::new();
    for segment in parse(pattern)? {
        let part = match segment {
            Segment::Literal(text) => js_literal(&text),
            Segment::Hour24 { pad } => {
                let value = format!("({hour_expr})");
                if pad {
                    format!("p2({value})")
                } else {
                    value
                }
            }
            Segment::Hour12 { pad } => {
                let value = format!("(({hour_expr} % 12) || 12)");
                if pad {
                    format!("p2({value})")
                } else {
                    value
                }
            }
            Segment::Minute { pad } => {
                let value = format!("({minute_expr})");
                if pad {
                    format!("p2({value})")
                } else {
                    value
                }
            }
            Segment::Second { pad } => {
                let value = format!("({second_expr})");
                if pad {
                    format!("p2({value})")
                } else {
                    value
                }
            }
            Segment::AmPm { upper } => {
                let (am, pm) = labels(upper);
                format!("(({hour_expr} < 12) ? {am} : {pm})")
            }
        };
        parts.push(part);
    }
    Ok(parts.join(" + "))
}

/// Buduje wyrażenie JS zwracające sformatowany czas (angielskie AM/PM).
/// `hour_expr`, `minute_expr`, `second_expr` to wyrażenia zwracające liczby.
pub fn js_expression(pattern: &str, hour_expr: &str, minute_expr: &str, second_expr: &str) -> Result<String> {
    js_expression_with_am_pm(pattern, hour_expr, minute_expr, second_expr, |upper| {
        if upper {
            (js_literal("AM"), js_literal("PM"))
        } else {
            (js_literal("am"), js_literal("pm"))
        }
    })
}

/// Wersja dla generowanej tarczy: etykiety AM/PM pochodzą z obiektu `L`,
/// który wybiera język w runtime na podstawie `hmSetting.getLanguage()`.
pub fn js_expression_localized(
    pattern: &str,
    hour_expr: &str,
    minute_expr: &str,
    second_expr: &str,
) -> Result<String> {
    js_expression_with_am_pm(pattern, hour_expr, minute_expr, second_expr, |upper| {
        if upper {
            ("L.amUp".to_string(), "L.pmUp".to_string())
        } else {
            ("L.am".to_string(), "L.pm".to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_common_patterns() {
        assert_eq!(format("HH:MM:SS", 9, 5, 3).unwrap(), "09:05:03");
        assert_eq!(format("H:MM", 9, 5, 3).unwrap(), "9:05");
        assert_eq!(format("hh:mm:ss A", 15, 5, 3).unwrap(), "03:05:03 PM");
        assert_eq!(format("h a", 0, 0, 0).unwrap(), "12 am");
        assert_eq!(format("MM\\HSS", 9, 5, 3).unwrap(), "05H03");
    }

    #[test]
    fn detects_seconds_and_rejects_bad_patterns() {
        assert!(has_seconds("HH:MM:SS"));
        assert!(!has_seconds("HH:MM"));
        assert!(parse("").is_err());
        assert!(parse("xyz").is_err());
        assert!(parse("HH\\").is_err());
    }

    #[test]
    fn builds_js_expression() {
        let js = js_expression("HH:MM", "h", "m", "s").unwrap();
        assert_eq!(js, "p2((h)) + ':' + p2((m))");
        let js = js_expression("h A", "h", "m", "s").unwrap();
        assert_eq!(js, "((h % 12) || 12) + ' ' + ((h < 12) ? 'AM' : 'PM')");
    }

    #[test]
    fn formats_am_pm_in_polish() {
        assert_eq!(
            format_with_locale("hh A", 15, 5, 3, WatchfaceLocale::Polish).unwrap(),
            "03 POP."
        );
        assert_eq!(
            format_with_locale("h a", 9, 0, 0, WatchfaceLocale::Polish).unwrap(),
            "9 przedp."
        );
        assert_eq!(
            format_with_locale("h a", 9, 0, 0, WatchfaceLocale::English).unwrap(),
            "9 am"
        );
    }

    #[test]
    fn builds_localized_js_expression() {
        let js = js_expression_localized("h a", "h", "m", "s").unwrap();
        assert_eq!(
            js,
            "((h % 12) || 12) + ' ' + ((h < 12) ? L.am : L.pm)"
        );
        let js = js_expression_localized("hh A", "h", "m", "s").unwrap();
        assert!(js.contains("(h < 12) ? L.amUp : L.pmUp"));
    }
}
