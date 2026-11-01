//! Doku Varlığı, Ham Piksel Temsili ve Hata Dayanıklı Placeholder.
//!
//! Bozuk veya eksik dokularda mor/siyah dama tahtası (checkerboard) fallback
//! sağlayarak uygulamanın çökmesini engeller.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextureAsset {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub is_placeholder: bool,
}

impl TextureAsset {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
            is_placeholder: false,
        }
    }

    /// 16x16 ebadında hata bildiren mor-siyah dama tahtası (checkerboard placeholder) üretir.
    pub fn placeholder() -> Self {
        let width = 16;
        let height = 16;
        let mut rgba = vec![0u8; width * height * 4];

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let is_magenta = ((x / 4) + (y / 4)) % 2 == 0;
                if is_magenta {
                    rgba[idx] = 255; // R
                    rgba[idx + 1] = 0; // G
                    rgba[idx + 2] = 255; // B
                    rgba[idx + 3] = 255; // A
                } else {
                    rgba[idx] = 0; // R
                    rgba[idx + 1] = 0; // G
                    rgba[idx + 2] = 0; // B
                    rgba[idx + 3] = 255; // A
                }
            }
        }

        Self {
            width: width as u32,
            height: height as u32,
            rgba,
            is_placeholder: true,
        }
    }

    /// PNG baytlarından doku çözer. Hata durumunda Err döndürür.
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, String> {
        let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
            .map_err(|e| format!("PNG cozulemedi: {e}"))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        if w == 0 || h == 0 {
            return Err("Gecersiz boyut: 0x0".to_string());
        }
        Ok(Self {
            width: w,
            height: h,
            rgba: img.into_raw(),
            is_placeholder: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_placeholder() {
        let tex = TextureAsset::placeholder();
        assert_eq!(tex.width, 16);
        assert_eq!(tex.height, 16);
        assert_eq!(tex.rgba.len(), 16 * 16 * 4);
        assert!(tex.is_placeholder);

        // (0,0) pikseli mor olmalı [255, 0, 255, 255]
        assert_eq!(&tex.rgba[0..4], &[255, 0, 255, 255]);
        // (4,0) pikseli siyah olmalı [0, 0, 0, 255]
        assert_eq!(&tex.rgba[16..20], &[0, 0, 0, 255]);
    }
}
