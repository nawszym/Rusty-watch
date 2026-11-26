//! Generowanie kodu QR dla instalacji tarczy przez tryb deweloperski Zeppa.
//!
//! Całość jest w czystym Rustcie — bez usług webowych i bez zależności C.
//! Poziom korekcji `L` jest zgodny z `qrcode-terminal`, którego używa `zeus preview`.

use std::io::Cursor;

use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat, Luma};
use qrcode::{EcLevel, QrCode};

/// Zwraca PNG z kodem QR zakodowanym z `text` (np. `zpkd1://ip:port/plik.zip`).
pub fn qr_png_bytes(text: &str, size: u32) -> Result<Vec<u8>> {
    let code = QrCode::with_error_correction_level(text.as_bytes(), EcLevel::L)
        .context("nie udało się zakodować tekstu w kodzie QR")?;
    let image = code
        .render::<Luma<u8>>()
        .min_dimensions(size, size)
        .quiet_zone(true)
        .build();
    let mut png = Vec::new();
    DynamicImage::ImageLuma8(image)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .context("nie udało się zapisać PNG kodu QR")?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_png_is_a_real_png() {
        let png = qr_png_bytes("zpkd1://192.168.1.10:8123/Test.zip", 256).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let image = image::load_from_memory(&png).unwrap().to_luma8();
        assert!(image.width() >= 256 && image.height() >= 256);
        // Kod QR ma białe tło i czarne moduły — nie może być jednolity.
        let dark = image.pixels().filter(|p| p.0[0] < 128).count();
        let light = image.pixels().filter(|p| p.0[0] >= 128).count();
        assert!(dark > 0 && light > 0);
    }
}
