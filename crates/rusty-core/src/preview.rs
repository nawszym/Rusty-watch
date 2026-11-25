//! Renderer statycznego podglądu tarczy (`preview.png`) i animacji 7 klatek
//! (`preview.gif`). Cała logika jest w Rust — WinUI dostaje tylko gotowe pliki,
//! więc nie ma offscreen-renderowania kontrolek ani artefaktów pod edytorem.
//!
//! Dane klatek mają dwie warstwy:
//! * [`FRAME_PRESETS`] — zahardkodowany szkielet czasu i **prawdziwych dat**
//!   (7 kolejnych poniedziałków–niedziel, 7 różnych miesięcy), ściśle rosnące
//!   godziny/minuty/sekundy oraz wymuszone ekstrema wizualne;
//! * [`PreviewRandomSeries`] — jednorazowo wylosowane wartości (tętno, kalorie,
//!   dystans, kroki itd.). Kroki zawsze wynikają z dystansu, więc widgety nie
//!   mogą pokazać sprzecznych liczb.
//!
//! Podgląd nie czyta Custom Data z edytora — jest niezależny od ustawień UI.

use ab_glyph::{point, Font, FontRef, PxScale, PxScaleFont, ScaleFont};
use gif::{Encoder, Frame, Repeat};
use image::imageops::{self, FilterType};
use image::{ImageEncoder, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::layers::{describe, LayerView};
use crate::locale::WatchfaceLocale;
use crate::model::{MeasurementSystem, StatusType, SCREEN_SIZE};
use crate::{date_format, time_format};

/// Liczba klatek animacji — jedna na każdy dzień tygodnia.
pub const FRAME_COUNT: usize = 7;
/// Opóźnienie jednej klatki GIF-a w setnych sekundy (~1,7 s).
const GIF_FRAME_DELAY: u16 = 170;
/// Promień okręgu tarczy Active 2 (Round).
const SCREEN_RADIUS: f64 = 233.0;
/// Wbudowany Inter używany, gdy projekt nie ma jeszcze przypisanej czcionki.
const DEFAULT_FONT_BYTES: &[u8] =
    include_bytes!("../../../native/RustyWatch/Assets/Fonts/Inter-Regular.ttf");

const HOURS: [u8; FRAME_COUNT] = [6, 9, 12, 15, 18, 20, 21];
const MINUTES: [u8; FRAME_COUNT] = [5, 11, 17, 23, 29, 33, 37];
const SECONDS: [u8; FRAME_COUNT] = [4, 10, 16, 22, 28, 34, 40];

/// Widoczność ikon STATUS w kolejnych klatkach. Bity: ALARM=1, DND=2,
/// BLUETOOTH=4 (oznacza rozłączenie), LOCK=8.
///
/// Ikony w rzeczywistości pojawiają się rzadko — tylko 5 z 7 klatek ma jakąś
/// ikonę. Ostatnia klatka (7) ma wszystkie cztery i jest źródłem `preview.png`.
/// Liczby widocznych ikon są celowo nierośnące: 2,2,0,3,0,1,4.
const STATUS_MASKS: [u8; FRAME_COUNT] = [
    0b1010, // 1: noc, DND i blokada
    0b1010, // 2: DND i blokada
    0b0000, // 3: czysty poranek
    0b1011, // 4: alarm + DND + blokada
    0b0000, // 5: pusto
    0b0100, // 6: chwilowe rozłączenie Bluetooth
    0b1111, // 7: pełny zestaw (→ preview.png)
];

/// Która wartość ma być wymuszona na skrajną, żeby przetestować układ widgetu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewExtreme {
    MinHeartRate,
    MaxHeartRate,
}

/// Zahardkodowany szkielet klatki — bez wartości losowych.
#[derive(Debug, Clone, Copy)]
pub struct PreviewFramePreset {
    pub index: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Prawdziwa data klatki: poniedziałek–niedziela, 7 różnych miesięcy 2022.
    pub date: PreviewDate,
    /// Maska widocznych ikon STATUS (`ALARM=1, DND=2, BLUETOOTH=4, LOCK=8`).
    pub status_mask: u8,
    force_extreme: Option<PreviewExtreme>,
}

const fn make_preset(index: usize) -> PreviewFramePreset {
    PreviewFramePreset {
        index: index as u8,
        hour: HOURS[index],
        minute: MINUTES[index],
        second: SECONDS[index],
        date: PreviewDate {
            day: DATES[index].0,
            month: DATES[index].1,
            year: DATES[index].2,
        },
        status_mask: STATUS_MASKS[index],
        force_extreme: if index == 0 {
            Some(PreviewExtreme::MinHeartRate)
        } else if index == FRAME_COUNT - 1 {
            Some(PreviewExtreme::MaxHeartRate)
        } else {
            None
        },
    }
}

/// (dzień, miesiąc 1-based, rok) — realne daty 2022-01-03 … 2022-07-31,
/// kolejno poniedziałek … niedziela i kolejno styczeń … lipiec.
const DATES: [(u32, u32, i32); FRAME_COUNT] = [
    (3, 1, 2022),
    (8, 2, 2022),
    (9, 3, 2022),
    (14, 4, 2022),
    (20, 5, 2022),
    (25, 6, 2022),
    (31, 7, 2022),
];

/// 7 stanów tarczy: godziny rosną, minuty i sekundy nigdy nie cofają się,
/// każda klatka ma inny, prawdziwy dzień tygodnia, datę i miesiąc.
pub const FRAME_PRESETS: [PreviewFramePreset; FRAME_COUNT] = [
    make_preset(0),
    make_preset(1),
    make_preset(2),
    make_preset(3),
    make_preset(4),
    make_preset(5),
    make_preset(6),
];

/// Data klatki — z predefiniowanego kalendarza 7 prawdziwych dat.
/// Wszystkie widgety daty wewnątrz klatki korzystają z tego samego wpisu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDate {
    pub day: u32,
    /// 1-based (tak jak `Date.getDate()` w ZeppOS).
    pub month: u32,
    pub year: i32,
}

/// Jednorazowa seria wylosowanych danych zwracana przez FFI do edytora.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRandomSeries {
    pub dates: Vec<PreviewDate>,
    pub heart_rates: Vec<u16>,
    pub calories: Vec<u16>,
    pub distances_m: Vec<u32>,
    pub steps_per_km: Vec<u16>,
    pub stress: Vec<u8>,
    pub pai: Vec<u16>,
    pub skin_temp_c: Vec<f32>,
    pub sleep_minutes: Vec<u32>,
    pub spo2: Vec<u8>,
    /// Liczba klatek — C# czyta ją, żeby nie mieć własnej, rozjechanej stałej.
    #[serde(default = "default_frame_count")]
    pub frame_count: usize,
    /// Język nazw miesięcy i oznaczeń AM/PM (`"pl"` albo `"en"`).
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "en".to_string()
}

fn default_frame_count() -> usize {
    FRAME_COUNT
}

/// Finalny, rozwiązany stan pojedynczej klatki przekazywany do renderera.
#[derive(Debug, Clone)]
pub struct PreviewFrameData {
    pub index: usize,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    /// Maska widocznych ikon STATUS w tej klatce.
    pub status_mask: u8,
    pub date: PreviewDate,
    pub battery_pct: u8,
    pub heart_rate: u16,
    pub calories: u16,
    pub distance_m: u32,
    pub steps: u32,
    pub steps_per_km: u16,
    pub stress: u8,
    pub pai: u16,
    pub skin_temp_c: f32,
    pub sleep_minutes: u32,
    pub spo2: u8,
}

/// Losuje całą serię 7 klatek. `seed` pozwala uzyskać powtarzalny wynik w testach.
pub fn random_series(seed: u64, locale: &str) -> PreviewRandomSeries {
    let mut rng = fastrand::Rng::with_seed(seed);
    let mut dates = Vec::with_capacity(FRAME_COUNT);
    let mut heart_rates = Vec::with_capacity(FRAME_COUNT);
    let mut calories = Vec::with_capacity(FRAME_COUNT);
    let mut distances_m = Vec::with_capacity(FRAME_COUNT);
    let mut steps_per_km = Vec::with_capacity(FRAME_COUNT);
    let mut stress = Vec::with_capacity(FRAME_COUNT);
    let mut pai = Vec::with_capacity(FRAME_COUNT);
    let mut skin_temp_c = Vec::with_capacity(FRAME_COUNT);
    let mut sleep_minutes = Vec::with_capacity(FRAME_COUNT);
    let mut spo2 = Vec::with_capacity(FRAME_COUNT);

    let mut heart = 64u16;
    let mut kcal = 0u16;
    let mut metres = 0u32;
    let mut pai_value = 0u16;

    for (index, preset) in FRAME_PRESETS.iter().enumerate() {
        dates.push(preset.date);

        heart = match preset.force_extreme {
            Some(PreviewExtreme::MinHeartRate) => 56,
            Some(PreviewExtreme::MaxHeartRate) => 120,
            None => (heart as i32 + rng.i32(-7..=7)).clamp(56, 118) as u16,
        };
        heart_rates.push(heart);

        // Narastające, ale z losowym wahaniem wokół równomiernego przebiegu tygodnia.
        let kcal_target = (2450.0 * index as f64 / (FRAME_COUNT - 1) as f64).round() as i32;
        kcal = (kcal_target + rng.i32(-45..=45)).max(kcal as i32).min(2450) as u16;
        if index == 0 {
            kcal = 0;
        }
        if index == FRAME_COUNT - 1 {
            kcal = 2450;
        }
        calories.push(kcal);

        let metre_target = (12300.0 * index as f64 / (FRAME_COUNT - 1) as f64).round() as i32;
        metres = (metre_target + rng.i32(-220..=220))
            .max(metres as i32)
            .min(12300) as u32;
        if index == 0 {
            metres = 0;
        }
        if index == FRAME_COUNT - 1 {
            metres = 12300;
        }
        distances_m.push(metres);
        steps_per_km.push(rng.u16(1300..=1400));

        stress.push(rng.u8(3..=92));
        pai_value = (pai_value + rng.u16(0..=16)).min(268);
        pai.push(pai_value);
        skin_temp_c.push(36.2 + rng.f32() * 1.0);
        sleep_minutes.push(if (1..7).contains(&index) {
            rng.u32(300..=420)
        } else if index >= FRAME_COUNT - 2 {
            rng.u32(180..=390)
        } else {
            0
        });
        spo2.push(rng.u8(95..=100));
    }

    PreviewRandomSeries {
        dates,
        heart_rates,
        calories,
        distances_m,
        steps_per_km,
        stress,
        pai,
        skin_temp_c,
        sleep_minutes,
        spo2,
        frame_count: FRAME_COUNT,
        locale: locale.to_string(),
    }
}

/// Seed oparty na zegarze i PID — kolejne eksporty dają inne dane.
pub fn random_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ (std::process::id() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// Łączy zahardkodowany preset z wylosowaną serią i wylicza spójne kroki.
pub fn frame_data(index: usize, series: &PreviewRandomSeries) -> Result<PreviewFrameData, String> {
    if index >= FRAME_COUNT {
        return Err(format!("Nieprawidłowy indeks klatki: {index}"));
    }
    let expected = [
        series.dates.len(),
        series.heart_rates.len(),
        series.calories.len(),
        series.distances_m.len(),
        series.steps_per_km.len(),
        series.stress.len(),
        series.pai.len(),
        series.skin_temp_c.len(),
        series.sleep_minutes.len(),
        series.spo2.len(),
    ];
    if expected.iter().any(|len| *len != FRAME_COUNT) {
        return Err("Seria danych podglądu musi zawierać dokładnie 7 wpisów".to_string());
    }

    let preset = FRAME_PRESETS[index];
    let distance_m = series.distances_m[index];
    let steps_per_km = series.steps_per_km[index];
    let steps = (distance_m as f64 / 1000.0 * steps_per_km as f64).round() as u32;
    // Bateria maleje liniowo 100% -> 10% i nie jest losowana.
    let battery_pct = 100 - (index as u32 * 90 / (FRAME_COUNT as u32 - 1)) as u8;

    Ok(PreviewFrameData {
        index,
        hour: preset.hour,
        minute: preset.minute,
        second: preset.second,
        status_mask: preset.status_mask,
        date: series.dates[index],
        battery_pct,
        heart_rate: series.heart_rates[index],
        calories: series.calories[index],
        distance_m,
        steps,
        steps_per_km,
        stress: series.stress[index],
        pai: series.pai[index],
        skin_temp_c: series.skin_temp_c[index],
        sleep_minutes: series.sleep_minutes[index],
        spo2: series.spo2[index],
    })
}

/// Wartość tekstowa widgetu w danej klatce — te same wzorce co podgląd i JS.
pub fn text_for_layer(
    layer: &LayerView,
    frame: &PreviewFrameData,
    project: &crate::model::Project,
    locale: WatchfaceLocale,
) -> String {
    let unit = layer.show_unit;
    match layer.kind.as_str() {
        "time" | "analog-time" => {
            let pattern = layer
                .time_format
                .as_deref()
                .unwrap_or("HH:MM:SS");
            time_format::format_with_locale(
                pattern,
                frame.hour as u32,
                frame.minute as u32,
                frame.second as u32,
                locale,
            )
            .unwrap_or_else(|_| layer.text.clone().unwrap_or_default())
        }
        "date" => {
            let pattern = layer
                .time_format
                .as_deref()
                .unwrap_or(crate::date_format::DEFAULT_PATTERN);
            // `date_format` przyjmuje miesiąc 0-based (jak Date.getMonth()).
            date_format::format_with_locale(
                pattern,
                frame.date.day,
                frame.date.month.saturating_sub(1),
                frame.date.year,
                locale,
            )
            .unwrap_or_else(|_| layer.text.clone().unwrap_or_default())
        }
        "weekday-number" => date_format::weekday_number(
            frame.date.day,
            frame.date.month,
            frame.date.year,
            project.first_day_of_week,
        )
        .to_string(),
        "week-number" => date_format::iso_week(frame.date.day, frame.date.month, frame.date.year).to_string(),
        "day-of-year" => {
            date_format::day_of_year(frame.date.day, frame.date.month, frame.date.year).to_string()
        }
        // ZeppOS nie udostępnia w tarczy danych księżycowych w formacie do
        // wiarygodnego policzenia w PC — uczciwie pokazujemy placeholder.
        "lunar-date" | "festival" => "--".to_string(),
        "world-clock" => format!("{:02}:{:02}", (frame.hour as u32 + 8) % 24, frame.minute),
        "steps" => frame.steps.to_string(),
        "distance" => {
            let (system, suffix) = match layer.measurement_system {
                Some(ref value) => {
                    let parsed = MeasurementSystem::parse(value).unwrap_or(project.measurement_system);
                    (parsed, parsed.suffix())
                }
                None => (project.measurement_system, project.measurement_system.suffix()),
            };
            format!(
                "{:.2} {}",
                frame.distance_m as f64 / system.meters_per_unit(),
                suffix
            )
        }
        "heart" => {
            if unit {
                format!("{} bpm", frame.heart_rate)
            } else {
                frame.heart_rate.to_string()
            }
        }
        "stress" => frame.stress.to_string(),
        "pai" => frame.pai.to_string(),
        "skin-temp" => {
            if unit {
                format!("{:.1} °C", frame.skin_temp_c)
            } else {
                format!("{:.1}", frame.skin_temp_c)
            }
        }
        "sleep" => format!("{}:{:02}", frame.sleep_minutes / 60, frame.sleep_minutes % 60),
        "calories" => {
            if unit {
                format!("{} kcal", frame.calories)
            } else {
                frame.calories.to_string()
            }
        }
        "spo2" => {
            if unit {
                format!("{}%", frame.spo2)
            } else {
                frame.spo2.to_string()
            }
        }
        "battery" => format!("{}%", frame.battery_pct),
        _ => layer.text.clone().unwrap_or_default(),
    }
}

fn parse_argb(raw: Option<&str>) -> Rgba<u8> {
    let value = raw.unwrap_or("0xFFFFFFFF").trim();
    let hex = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("#"))
        .unwrap_or(value);
    if hex.len() != 8 {
        return Rgba([255, 255, 255, 255]);
    }
    let a = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let r = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
    Rgba([r, g, b, a])
}

fn blend(canvas: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>, alpha: f32) {
    if x < 0 || y < 0 || x >= SCREEN_SIZE as i32 || y >= SCREEN_SIZE as i32 || alpha <= 0.0 {
        return;
    }
    let index = (y as u32 * SCREEN_SIZE + x as u32) as usize;
    let dst = canvas.as_mut_ptr();
    let source_alpha = (color.0[3] as f32 / 255.0) * alpha;
    if source_alpha <= 0.0 {
        return;
    }
    // Tło jest zawsze nieprzezroczyste (czarne `FILL_RECT`), więc zwykłe
    // alpha-blending daje ten sam efekt co kompozycja ZeppOS.
    unsafe {
        for channel in 0..3 {
            let src = color.0[channel] as f32 / 255.0;
            let back = *dst.add(index * 4 + channel) as f32 / 255.0;
            let mixed = src * source_alpha + back * (1.0 - source_alpha);
            *dst.add(index * 4 + channel) = (mixed * 255.0).round() as u8;
        }
    }
}

fn sample_bilinear(image: &RgbaImage, x: f64, y: f64) -> Rgba<u8> {
    let (w, h) = image.dimensions();
    if x < 0.0 || y < 0.0 || x >= w as f64 - 1.0 || y >= h as f64 - 1.0 {
        return Rgba([0, 0, 0, 0]);
    }
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let fx = (x - x0 as f64) as f32;
    let fy = (y - y0 as f64) as f32;
    let p00 = image.get_pixel(x0, y0).0;
    let p10 = image.get_pixel(x0 + 1, y0).0;
    let p01 = image.get_pixel(x0, y0 + 1).0;
    let p11 = image.get_pixel(x0 + 1, y0 + 1).0;
    let mut out = [0u8; 4];
    for channel in 0..4 {
        let top = p00[channel] as f32 * (1.0 - fx) + p10[channel] as f32 * fx;
        let bottom = p01[channel] as f32 * (1.0 - fx) + p11[channel] as f32 * fx;
        out[channel] = (top * (1.0 - fy) + bottom * fy).round() as u8;
    }
    Rgba(out)
}

/// Obraca kwadratowy obraz wskazówki wokół środka. Dodatni kąt = zgodnie z ruchem zegara.
pub(crate) fn rotate_hand(src: &RgbaImage, degrees: f64) -> RgbaImage {
    let (w, h) = src.dimensions();
    let mut out = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let angle = degrees.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    for y in 0..h {
        for x in 0..w {
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - cy;
            let sx = cos * dx + sin * dy + cx - 0.5;
            let sy = -sin * dx + cos * dy + cy - 0.5;
            out.put_pixel(x, y, sample_bilinear(src, sx, sy));
        }
    }
    out
}

fn overlay_asset(
    canvas: &mut RgbaImage,
    project_dir: &Path,
    src: Option<&str>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    resize: bool,
) {
    let src = match src {
        Some(value) if !value.is_empty() => value,
        _ => return,
    };
    let path = project_dir.join("assets").join(src);
    let image = match image::open(path) {
        Ok(image) => image.to_rgba8(),
        Err(_) => return,
    };
    if resize && image.dimensions() != (w, h) && w > 0 && h > 0 {
        let fitted = imageops::resize(&image, w, h, FilterType::Lanczos3);
        imageops::overlay(canvas, &fitted, x as i64, y as i64);
    } else {
        imageops::overlay(canvas, &image, x as i64, y as i64);
    }
}

fn measure_width(font: &FontRef<'_>, scaled: &PxScaleFont<&FontRef<'_>>, text: &str) -> f32 {
    text.chars()
        .map(|c| scaled.h_advance(font.glyph_id(c)))
        .sum()
}

fn layout_lines(
    font: &FontRef<'_>,
    scaled: &PxScaleFont<&FontRef<'_>>,
    text: &str,
    max_width: f32,
    style: &str,
) -> Vec<String> {
    if style == "wrap" {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in text.split_whitespace() {
            let candidate = if current.is_empty() {
                word.to_string()
            } else {
                format!("{current} {word}")
            };
            if measure_width(font, scaled, &candidate) <= max_width || current.is_empty() {
                current = candidate;
            } else {
                lines.push(std::mem::take(&mut current));
                current = word.to_string();
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        return if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
    }

    let mut line = text.to_string();
    if style == "ellipsis" && measure_width(font, scaled, &line) > max_width {
        while line.len() > 1 && measure_width(font, scaled, &format!("{line}…")) > max_width {
            line.pop();
        }
        line.push('…');
    }
    vec![line]
}

fn draw_text(canvas: &mut RgbaImage, project_dir: &Path, layer: &LayerView, text: &str) {
    if text.is_empty() || layer.w <= 0 || layer.h <= 0 {
        return;
    }
    // Widget zawsze niesie nazwę pliku TTF — podgląd i eksport używają tego
    // samego fontu. Brak pliku (stare projekty) wraca do wbudowanego Inter.
    let project_font = layer
        .font
        .as_deref()
        .filter(|name| !name.is_empty())
        .map(|name| project_dir.join("assets").join("fonts").join(name));
    let font_bytes = project_font
        .as_ref()
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_else(|| DEFAULT_FONT_BYTES.to_vec());
    let font = match FontRef::try_from_slice(&font_bytes) {
        Ok(font) => font,
        Err(_) => return,
    };
    let scale = PxScale::from((layer.text_size.unwrap_or(64) as f32).max(1.0));
    let scaled = font.as_scaled(scale);
    let line_height = (scaled.ascent() - scaled.descent() + scaled.line_gap()).max(1.0);
    let lines = layout_lines(&font, &scaled, text, layer.w as f32, &layer.text_style);

    let total_height = line_height * lines.len() as f32;
    let top = match layer.v_align.as_str() {
        "top" => layer.y as f32,
        "bottom" => layer.y as f32 + layer.h as f32 - total_height,
        _ => layer.y as f32 + (layer.h as f32 - total_height) / 2.0,
    };

    let color = parse_argb(layer.color.as_deref());
    for (line_index, line) in lines.iter().enumerate() {
        let width = measure_width(&font, &scaled, line);
        let left = match layer.align.as_deref() {
            Some("left") => layer.x as f32,
            Some("right") => layer.x as f32 + layer.w as f32 - width,
            _ => layer.x as f32 + (layer.w as f32 - width) / 2.0,
        };
        let baseline = top + line_index as f32 * line_height + scaled.ascent();
        let mut pen_x = left;
        for c in line.chars() {
            let id = font.glyph_id(c);
            let advance = scaled.h_advance(id);
            let mut glyph = id.with_scale(scale);
            glyph.position = point(pen_x, baseline);
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                let origin_x = bounds.min.x as i32;
                let origin_y = bounds.min.y as i32;
                outlined.draw(|gx, gy, coverage| {
                    blend(
                        canvas,
                        origin_x + gx as i32,
                        origin_y + gy as i32,
                        color,
                        coverage,
                    );
                });
            }
            pen_x += advance;
        }
    }
}

fn apply_round_cutout(canvas: &mut RgbaImage) {
    let center = (SCREEN_SIZE as f64 - 1.0) / 2.0;
    for y in 0..SCREEN_SIZE {
        for x in 0..SCREEN_SIZE {
            let dx = x as f64 + 0.5 - center - 0.5;
            let dy = y as f64 + 0.5 - center - 0.5;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > SCREEN_RADIUS - 1.0 {
                let factor = ((SCREEN_RADIUS - distance).max(0.0)).min(1.0) as f32;
                let pixel = canvas.get_pixel(x, y);
                let black = Rgba([0, 0, 0, 255]);
                let mut mixed = [0u8; 4];
                for channel in 0..4 {
                    mixed[channel] = (pixel.0[channel] as f32 * factor
                        + black.0[channel] as f32 * (1.0 - factor))
                        .round() as u8;
                }
                canvas.put_pixel(x, y, Rgba(mixed));
            }
        }
    }
}

/// Rysuje jedną klatkę w kolejności warstw z `layers::describe()`.
pub fn render_frame(
    project_dir: &Path,
    project: &crate::model::Project,
    frame: &PreviewFrameData,
    series: &PreviewRandomSeries,
) -> RgbaImage {
    let mut canvas = RgbaImage::from_pixel(SCREEN_SIZE, SCREEN_SIZE, Rgba([0, 0, 0, 255]));
    let locale = WatchfaceLocale::from_code(&series.locale);

    for layer in describe(project) {
        if !layer.visible {
            continue;
        }
        match layer.kind.as_str() {
            "image" => overlay_asset(
                &mut canvas,
                project_dir,
                layer.src.as_deref(),
                layer.x,
                layer.y,
                layer.w as u32,
                layer.h as u32,
                true,
            ),
            "images" => overlay_asset(
                &mut canvas,
                project_dir,
                layer.src.as_deref(),
                layer.x,
                layer.y,
                layer.w as u32,
                layer.h as u32,
                true,
            ),
            "analog-time" => {
                let w = layer.w as u32;
                let h = layer.h as u32;
                let hands = [
                    (
                        layer.hour_src.as_deref(),
                        frame.hour as f64 % 12.0 * 30.0 + frame.minute as f64 * 0.5,
                    ),
                    (
                        layer.minute_src.as_deref(),
                        frame.minute as f64 * 6.0 + frame.second as f64 * 0.1,
                    ),
                    (layer.second_src.as_deref(), frame.second as f64 * 6.0),
                    (layer.cap_src.as_deref(), 0.0),
                ];
                for (src, angle) in hands {
                    let Some(src) = src else { continue };
                    let path = project_dir.join("assets").join(src);
                    let Ok(image) = image::open(path) else { continue };
                    let mut hand = image.to_rgba8();
                    if hand.dimensions() != (w, h) && w > 0 && h > 0 {
                        hand = imageops::resize(&hand, w, h, FilterType::Lanczos3);
                    }
                    let rotated = rotate_hand(&hand, angle);
                    imageops::overlay(&mut canvas, &rotated, layer.x as i64, layer.y as i64);
                }
            }
            "status-alarm" | "status-dnd" | "status-bluetooth" | "status-lock" => {
                let status = match layer.kind.as_str() {
                    "status-alarm" => StatusType::Alarm,
                    "status-dnd" => StatusType::Dnd,
                    "status-bluetooth" => StatusType::Bluetooth,
                    _ => StatusType::Lock,
                };
                // Ikona jest rysowana tylko wtedy, gdy dany stan jest aktywny
                // w tej klatce — dokładnie tak zachowuje się IMG_STATUS na zegarku.
                if frame.status_mask & status.bit() == 0 {
                    continue;
                }
                overlay_asset(
                    &mut canvas,
                    project_dir,
                    layer.src.as_deref(),
                    layer.x,
                    layer.y,
                    layer.w as u32,
                    layer.h as u32,
                    true,
                );
            }
            _ => {
                let value = text_for_layer(&layer, frame, project, locale);
                draw_text(&mut canvas, project_dir, &layer, &value);
            }
        }
    }

    apply_round_cutout(&mut canvas);
    canvas
}

/// Zapisuje klatkę jako PNG (pełny, nieprzezroczysty RGBA).
pub fn write_png(image: &RgbaImage, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|e| format!("Nie można zapisać PNG: {e}"))?;
    image::codecs::png::PngEncoder::new(file)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Nie można zakodować PNG: {e}"))
}

/// Koduje 7 klatek do animowanego GIF-a (~1,7 s na klatkę).
pub fn write_gif(frames: &[RgbaImage], path: &Path) -> Result<(), String> {
    if frames.is_empty() {
        return Err("Brak klatek do zapisu GIF".to_string());
    }
    let file = File::create(path).map_err(|e| format!("Nie można zapisać GIF: {e}"))?;
    let mut encoder = Encoder::new(
        file,
        frames[0].width() as u16,
        frames[0].height() as u16,
        &[],
    )
    .map_err(|e| format!("Nie można utworzyć kodera GIF: {e}"))?;
    // Animacja ma krążyć w nieskończoność — bez tego ZeppOS/Windows
    // odtwarzają 7 klatek tylko raz i zatrzymują się na ostatniej.
    encoder
        .set_repeat(Repeat::Infinite)
        .map_err(|e| format!("Nie można ustawić zapętlenia GIF: {e}"))?;
    for image in frames {
        let mut pixels = image.as_raw().to_vec();
        let mut frame = Frame::from_rgba_speed(
            image.width() as u16,
            image.height() as u16,
            &mut pixels,
            10,
        );
        // Jednostka to 1/100 s — 170 daje ~1,7 s na klatkę.
        frame.delay = GIF_FRAME_DELAY;
        encoder
            .write_frame(&frame)
            .map_err(|e| format!("Nie można zakodować klatki GIF: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FirstDayOfWeek;

    fn sample_series() -> PreviewRandomSeries {
        random_series(1234, "en")
    }

    #[test]
    fn presets_have_strictly_increasing_time_and_real_dates() {
        for i in 1..FRAME_COUNT {
            assert!(FRAME_PRESETS[i].hour > FRAME_PRESETS[i - 1].hour);
            assert!(FRAME_PRESETS[i].minute > FRAME_PRESETS[i - 1].minute);
            assert!(FRAME_PRESETS[i].second > FRAME_PRESETS[i - 1].second);
            assert!(FRAME_PRESETS[i].date.month > FRAME_PRESETS[i - 1].date.month);
            assert!(FRAME_PRESETS[i].date.day > FRAME_PRESETS[i - 1].date.day);
        }
        assert_eq!(
            FRAME_PRESETS[0].date,
            PreviewDate { day: 3, month: 1, year: 2022 }
        );
        assert_eq!(
            FRAME_PRESETS[6].date,
            PreviewDate { day: 31, month: 7, year: 2022 }
        );
        // Wszystkie daty są prawdziwe — 2022 nie jest rokiem przestępnym.
        assert!(FRAME_PRESETS
            .iter()
            .all(|p| p.date.year == 2022 && (1..=7).contains(&p.date.month)));
    }

    #[test]
    fn every_frame_has_a_distinct_real_weekday_and_month() {
        let months: std::collections::HashSet<u32> =
            FRAME_PRESETS.iter().map(|p| p.date.month).collect();
        assert_eq!(months.len(), FRAME_COUNT);
        let mut previous = 0;
        for (index, preset) in FRAME_PRESETS.iter().enumerate() {
            let weekday = crate::date_format::weekday_number(
                preset.date.day, preset.date.month, preset.date.year, FirstDayOfWeek::Monday,
            );
            assert_eq!(weekday, index as u32 + 1, "frame {}", index + 1);
            assert!(weekday > previous);
            previous = weekday;
        }
    }

    #[test]
    fn frame_7_is_the_static_21_37_preview() {
        let frame = FRAME_PRESETS[FRAME_COUNT - 1];
        assert_eq!((frame.hour, frame.minute, frame.second), (21, 37, 40));
        assert_eq!(frame.status_mask, 0b1111);
    }

    #[test]
    fn status_visibility_covers_all_five_states_and_is_not_monotonic() {
        let counts: Vec<u32> = FRAME_PRESETS
            .iter()
            .map(|frame| frame.status_mask.count_ones())
            .collect();
        assert_eq!(counts, [2, 2, 0, 3, 0, 1, 4]);
        let states: std::collections::HashSet<u32> = counts.iter().copied().collect();
        assert_eq!(states, [0, 1, 2, 3, 4].into_iter().collect());
        assert_eq!(counts.iter().filter(|count| **count == 0).count(), 2);
        assert!(counts.windows(2).any(|pair| pair[0] > pair[1]));
        assert!(counts.windows(2).any(|pair| pair[0] < pair[1]));
        // Klatka 4 ma alarm+DND+blokada, klatka 6 tylko Bluetooth.
        assert_eq!(FRAME_PRESETS[3].status_mask, 0b1011);
        assert_eq!(FRAME_PRESETS[5].status_mask, 0b0100);
    }

    #[test]
    fn series_covers_visual_extremes_and_uses_preset_dates() {
        let series = sample_series();
        assert_eq!(series.frame_count, FRAME_COUNT);
        assert_eq!(series.heart_rates[0], 56);
        assert_eq!(series.heart_rates[FRAME_COUNT - 1], 120);
        assert_eq!(series.calories[0], 0);
        assert_eq!(series.calories[FRAME_COUNT - 1], 2450);
        assert_eq!(series.distances_m[0], 0);
        assert_eq!(series.distances_m[FRAME_COUNT - 1], 12300);
        assert!(series
            .steps_per_km
            .iter()
            .all(|value| (1300..=1400).contains(value)));
        assert_eq!(series.dates.len(), FRAME_COUNT);
        for (date, preset) in series.dates.iter().zip(FRAME_PRESETS.iter()) {
            assert_eq!(*date, preset.date);
        }
    }

    #[test]
    fn steps_are_derived_from_distance_and_battery_decreases() {
        let series = sample_series();
        let mut previous_battery = 255u8;
        for index in 0..FRAME_COUNT {
            let frame = frame_data(index, &series).unwrap();
            let expected =
                (frame.distance_m as f64 / 1000.0 * frame.steps_per_km as f64).round() as u32;
            assert_eq!(frame.steps, expected);
            assert!(frame.battery_pct < previous_battery);
            previous_battery = frame.battery_pct;
        }
        assert_eq!(frame_data(0, &series).unwrap().battery_pct, 100);
        assert_eq!(
            frame_data(FRAME_COUNT - 1, &series).unwrap().battery_pct,
            10
        );
        let last = frame_data(FRAME_COUNT - 1, &series).unwrap();
        assert!((12_000..=17_500).contains(&last.steps), "steps={}", last.steps);
    }

    #[test]
    fn date_widgets_are_consistent_with_the_same_frame_date() {
        let series = sample_series();
        let frame = frame_data(3, &series).unwrap();
        // 14 kwietnia 2022 to czwartek: poniedziałek-first daje 4.
        let fixed = PreviewDate { day: 14, month: 4, year: 2022 };
        assert_eq!(frame.date, series.dates[3]);
        assert_eq!(frame.date, fixed);
        assert_eq!(
            date_format::weekday_number(fixed.day, fixed.month, fixed.year, FirstDayOfWeek::Monday),
            4
        );
        assert_eq!(
            date_format::weekday_number(fixed.day, fixed.month, fixed.year, FirstDayOfWeek::Sunday),
            5
        );
        assert_eq!(date_format::iso_week(fixed.day, fixed.month, fixed.year), 15);
        assert_eq!(date_format::day_of_year(fixed.day, fixed.month, fixed.year), 104);
        let text = date_format::format_with_locale(
            "DD.MM.YYYY",
            fixed.day,
            fixed.month - 1,
            fixed.year,
            WatchfaceLocale::Polish,
        )
        .unwrap();
        assert_eq!(text, "14.04.2022");
    }

    #[test]
    fn gif_frame_delay_is_about_1_7_seconds() {
        // Jednostka GIF-a to 1/100 s, więc 170 = 1,7 s na klatkę.
        assert_eq!(GIF_FRAME_DELAY, 170);
        let image = RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]));
        let path = std::env::temp_dir().join(format!("rw-delay-test-{}.gif", std::process::id()));
        write_gif(&[image.clone(), image], &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.windows(2).any(|w| w == [0xAA, 0x00]), "brak opóźnienia 170");
        assert!(bytes.windows(11).any(|w| w == b"NETSCAPE2.0"));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn cutout_paints_corners_black() {
        let mut image = RgbaImage::from_pixel(SCREEN_SIZE, SCREEN_SIZE, Rgba([200, 100, 50, 255]));
        apply_round_cutout(&mut image);
        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(image.get_pixel(465, 465).0, [0, 0, 0, 255]);
        assert_ne!(image.get_pixel(233, 233).0, [0, 0, 0, 255]);
    }
}
