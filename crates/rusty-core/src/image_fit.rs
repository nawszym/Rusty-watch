//! Wspólne skalowanie obrazków dla tarczy.
//!
//! ZeppOS nie potrafi przeskalować zasobu `IMG` w runtime — rozmiar obrazu to
//! rozmiar pliku. Dlatego każdy obrazek (tło dzisiaj, pogoda/alert w przyszłości)
//! musi być przygotowany do dokładnych wymiarów jeszcze przed eksportem.
//! `FitMode::Cover` przycina nadmiar zamiast dodawać puste pola — dzięki temu
//! nie pojawiają się białe paski ani przezroczyste krawędzie.

use image::imageops::{self, FilterType};
use image::{DynamicImage, GenericImageView, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FitMode {
    /// Wypełnia cały cel, przycinając nadmiar (domyślne dla tła — zero pustych pikseli).
    Cover,
    /// Zachowuje cały obraz, dodając przezroczyste pola (nie białe).
    /// Domyślny tryb widgetu IMAGES — nie ucina grafiki użytkownika.
    #[default]
    Contain,
    /// Rozciąga obraz do wymiarów celu, ignorując proporcje.
    Stretch,
}

impl FitMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cover" => Some(Self::Cover),
            "contain" => Some(Self::Contain),
            "stretch" => Some(Self::Stretch),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cover => "cover",
            Self::Contain => "contain",
            Self::Stretch => "stretch",
        }
    }
}

/// Prostokąt źródła, który trafia do celu. Zawsze w granicach obrazu wejściowego.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Oblicza przycięcie dla `Cover`. Dla pozostałych trybów zwraca cały obraz.
pub fn cover_crop(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> CropRect {
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return CropRect {
            x: 0,
            y: 0,
            w: src_w,
            h: src_h,
        };
    }
    let src_ratio = src_w as f64 / src_h as f64;
    let dst_ratio = dst_w as f64 / dst_h as f64;
    if src_ratio > dst_ratio {
        // Za szeroki — przycinamy boki.
        let w = ((src_h as f64 * dst_ratio).round() as u32).clamp(1, src_w);
        CropRect {
            x: (src_w - w) / 2,
            y: 0,
            w,
            h: src_h,
        }
    } else {
        // Za wysoki (albo dokładnie pasuje) — przycinamy górę i dół.
        let h = ((src_w as f64 / dst_ratio).round() as u32).clamp(1, src_h);
        CropRect {
            x: 0,
            y: (src_h - h) / 2,
            w: src_w,
            h,
        }
    }
}

fn filter_for_scale(scale: f64) -> FilterType {
    if scale < 1.0 {
        // Zmniejszanie: Lanczos zachowuje ostrość detali tarczy.
        FilterType::Lanczos3
    } else {
        // Powiększanie: łagodniejszy filtr nie daje „ząbkowania".
        FilterType::CatmullRom
    }
}

/// Przygotowuje obrazek do dokładnie `dst_w × dst_h` w wybranym trybie.
/// Wynik jest zawsze RGBA8 — ZeppOS zaleca 24/32-bit PNG.
pub fn fit_image(src: &DynamicImage, dst_w: u32, dst_h: u32, mode: FitMode) -> RgbaImage {
    let (src_w, src_h) = src.dimensions();
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return RgbaImage::new(dst_w.max(1), dst_h.max(1));
    }

    match mode {
        FitMode::Stretch => src.resize_exact(dst_w, dst_h, FilterType::CatmullRom).to_rgba8(),
        FitMode::Cover => {
            let rect = cover_crop(src_w, src_h, dst_w, dst_h);
            let cropped = imageops::crop_imm(src, rect.x, rect.y, rect.w, rect.h).to_image();
            let scale = rect.w as f64 / dst_w as f64;
            DynamicImage::ImageRgba8(cropped)
                .resize_exact(dst_w, dst_h, filter_for_scale(scale))
                .to_rgba8()
        }
        FitMode::Contain => {
            let scale = (dst_w as f64 / src_w as f64).min(dst_h as f64 / src_h as f64);
            let w = ((src_w as f64 * scale).round() as u32).clamp(1, dst_w);
            let h = ((src_h as f64 * scale).round() as u32).clamp(1, dst_h);
            let resized = src.resize_exact(w, h, filter_for_scale(scale)).to_rgba8();
            let mut output = RgbaImage::new(dst_w, dst_h);
            imageops::overlay(
                &mut output,
                &resized,
                ((dst_w - w) / 2) as i64,
                ((dst_h - h) / 2) as i64,
            );
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn cover_crops_wide_and_tall_images_centered() {
        assert_eq!(
            cover_crop(1000, 500, 466, 466),
            CropRect {
                x: 250,
                y: 0,
                w: 500,
                h: 500
            }
        );
        assert_eq!(
            cover_crop(500, 1000, 466, 466),
            CropRect {
                x: 0,
                y: 250,
                w: 500,
                h: 500
            }
        );
    }

    #[test]
    fn cover_keeps_exact_and_small_images_in_bounds() {
        assert_eq!(
            cover_crop(466, 466, 466, 466),
            CropRect {
                x: 0,
                y: 0,
                w: 466,
                h: 466
            }
        );
        let small = cover_crop(10, 10, 466, 466);
        assert_eq!(small.w, 10);
        assert_eq!(small.h, 10);
        let huge = cover_crop(4000, 3000, 466, 466);
        assert!(huge.x + huge.w <= 4000);
        assert!(huge.y + huge.h <= 3000);
    }

    #[test]
    fn cover_output_has_no_empty_edges() {
        let source = RgbaImage::from_pixel(800, 300, Rgba([20, 40, 60, 255]));
        let output = fit_image(&DynamicImage::ImageRgba8(source), 466, 466, FitMode::Cover);
        assert_eq!(output.dimensions(), (466, 466));
        for (x, y) in [(0, 0), (465, 0), (0, 465), (465, 465), (233, 233)] {
            assert_eq!(output.get_pixel(x, y).0[3], 255, "transparent pixel at {x},{y}");
        }
    }

    #[test]
    fn contain_pads_with_transparent_pixels_not_white() {
        let source = RgbaImage::from_pixel(100, 200, Rgba([20, 40, 60, 255]));
        let output = fit_image(&DynamicImage::ImageRgba8(source), 466, 466, FitMode::Contain);
        assert_eq!(output.dimensions(), (466, 466));
        assert_eq!(output.get_pixel(0, 0).0[3], 0);
        assert_eq!(output.get_pixel(233, 233).0[3], 255);
    }

    #[test]
    fn stretch_fills_exact_dimensions() {
        let source = RgbaImage::from_pixel(10, 200, Rgba([10, 20, 30, 255]));
        let output = fit_image(&DynamicImage::ImageRgba8(source), 466, 466, FitMode::Stretch);
        assert_eq!(output.dimensions(), (466, 466));
        assert_eq!(output.get_pixel(0, 0).0[3], 255);
    }
}
