//! 5x7 Profesyonel Vektörel Bitmap Font Motoru.
//!
//! Her glif 5 sütun x 7 satır çözünürlükte tasarlanmıştır.
//! Büyük harfler (A-Z), küçük harfler (a-z), sayılar (0-9), Türkçe karakterler (ç, ğ, ı, ö, ş, ü)
//! ve UI sembolleri (oklar, cetvel, noktalama) için net, yüksek kontrastlı ve okunaklı glifler sunar.

use crate::pipeline_2d::QuadInstance;

/// Verilen karakter için 5x7 bit haritasını (7 satır, her satırın alt 5 biti) döndürür.
pub fn glyph_rows(c: char) -> [u8; 7] {
    match c {
        // --- UI Simgeleri ve Oklar ---
        '▼' | '▾' => [
            0b00000, 0b00000, 0b11111, 0b01110, 0b00100, 0b00000, 0b00000,
        ],
        '▲' => [
            0b00000, 0b00100, 0b01110, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '▶' => [
            0b10000, 0b11000, 0b11100, 0b11110, 0b11100, 0b11000, 0b10000,
        ],
        '◀' => [
            0b00001, 0b00011, 0b00111, 0b01111, 0b00111, 0b00011, 0b00001,
        ],
        '⏸' => [
            0b11011, 0b11011, 0b11011, 0b11011, 0b11011, 0b11011, 0b11011,
        ],
        '⏹' => [
            0b00000, 0b11111, 0b11111, 0b11111, 0b11111, 0b11111, 0b00000,
        ],
        '⏭' => [
            0b10001, 0b11001, 0b11101, 0b11111, 0b11101, 0b11001, 0b10001,
        ],
        '⏮' => [
            0b10001, 0b10011, 0b10111, 0b11111, 0b10111, 0b10011, 0b10001,
        ],
        '✓' => [
            0b00000, 0b00001, 0b00011, 0b10110, 0b11100, 0b01000, 0b00000,
        ],
        '✕' | '✖' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        '^' => [
            0b00100, 0b01110, 0b11111, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '*' => [
            0b00000, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0b00000,
        ],
        '■' => [
            0b00000, 0b11111, 0b11111, 0b11111, 0b11111, 0b11111, 0b00000,
        ],
        '●' => [
            0b00000, 0b01110, 0b11111, 0b11111, 0b11111, 0b01110, 0b00000,
        ],
        '•' => [
            0b00000, 0b00000, 0b01110, 0b01110, 0b01110, 0b00000, 0b00000,
        ],
        '❚' | '▎' | '▌' => [
            0b01100, 0b01100, 0b01100, 0b01100, 0b01100, 0b01100, 0b01100,
        ],

        // --- Sayılar ---
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],

        // --- Büyük Harfler ---
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00001, 0b00001, 0b00001, 0b00001, 0b10001, 0b10001, 0b01110,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],

        // --- Küçük Harfler ---
        'a' => [
            0b00000, 0b00000, 0b01110, 0b00001, 0b01111, 0b10001, 0b01111,
        ],
        'b' => [
            0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'c' => [
            0b00000, 0b00000, 0b01110, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'd' => [
            0b00001, 0b00001, 0b01111, 0b10001, 0b10001, 0b10001, 0b01111,
        ],
        'e' => [
            0b00000, 0b00000, 0b01110, 0b10001, 0b11111, 0b10000, 0b01110,
        ],
        'f' => [
            0b00110, 0b01001, 0b01000, 0b11100, 0b01000, 0b01000, 0b01000,
        ],
        'g' => [
            0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b01110,
        ],
        'h' => [
            0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
        'i' => [
            0b00100, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'j' => [
            0b00010, 0b00000, 0b00110, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'k' => [
            0b10000, 0b10000, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010,
        ],
        'l' => [
            0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'm' => [
            0b00000, 0b00000, 0b11011, 0b10101, 0b10101, 0b10101, 0b10101,
        ],
        'n' => [
            0b00000, 0b00000, 0b11110, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
        'o' => [
            0b00000, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'p' => [
            0b00000, 0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000,
        ],
        'q' => [
            0b00000, 0b01111, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001,
        ],
        'r' => [
            0b00000, 0b00000, 0b10110, 0b11001, 0b10000, 0b10000, 0b10000,
        ],
        's' => [
            0b00000, 0b00000, 0b01111, 0b10000, 0b01110, 0b00001, 0b11110,
        ],
        't' => [
            0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00100, 0b00011,
        ],
        'u' => [
            0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'v' => [
            0b00000, 0b00000, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'w' => [
            0b00000, 0b00000, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'x' => [
            0b00000, 0b00000, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001,
        ],
        'y' => [
            0b00000, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        'z' => [
            0b00000, 0b00000, 0b11111, 0b00010, 0b00100, 0b01000, 0b11111,
        ],

        // --- Türkçe Karakterler ---
        'ç' => [
            0b00000, 0b00000, 0b01110, 0b10000, 0b10001, 0b01110, 0b00100,
        ],
        'Ç' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10001, 0b01110, 0b00100,
        ],
        'ğ' => [
            0b01110, 0b00000, 0b01111, 0b10001, 0b01111, 0b00001, 0b01110,
        ],
        'Ğ' => [
            0b01110, 0b00000, 0b01110, 0b10000, 0b10111, 0b10001, 0b01110,
        ],
        'ı' => [
            0b00000, 0b00000, 0b01100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'İ' => [
            0b00100, 0b00000, 0b01110, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'ö' => [
            0b01010, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'Ö' => [
            0b01010, 0b00000, 0b01110, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'ş' => [
            0b00000, 0b01110, 0b10000, 0b01110, 0b00001, 0b11110, 0b00100,
        ],
        'Ş' => [
            0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b01110, 0b00100,
        ],
        'ü' => [
            0b01010, 0b00000, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'Ü' => [
            0b01010, 0b00000, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],

        // --- Noktalama ve Semboller ---
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ',' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100, 0b01000,
        ],
        '%' => [
            0b11001, 0b11010, 0b00100, 0b01000, 0b10110, 0b00110, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
        ],
        '\\' => [
            0b10000, 0b01000, 0b00100, 0b00010, 0b00001, 0b00000, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '+' => [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
        '[' => [
            0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110,
        ],
        ']' => [
            0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110,
        ],
        '(' => [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
        ')' => [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
        '<' => [
            0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010,
        ],
        '>' => [
            0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000,
        ],
        '=' => [
            0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '#' => [
            0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
        '|' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        '_' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
        ],
        '?' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100,
        ],
        '!' => [
            0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
        ';' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b00100, 0b01000,
        ],
        '\'' => [
            0b00100, 0b00100, 0b00010, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '"' => [
            0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '{' => [
            0b00011, 0b00010, 0b00010, 0b00100, 0b00010, 0b00010, 0b00011,
        ],
        '}' => [
            0b11000, 0b01000, 0b01000, 0b00100, 0b01000, 0b01000, 0b11000,
        ],
        '$' => [
            0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100,
        ],
        '@' => [
            0b01110, 0b10001, 0b01101, 0b10101, 0b10110, 0b10000, 0b01111,
        ],
        '&' => [
            0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101,
        ],
        '~' => [
            0b00000, 0b01101, 0b10010, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
        '`' => [
            0b01000, 0b00100, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],

        // Eşleşmeyen diğer karakterler için güvenli büyük/küçük harf geri dönüşü
        _ => {
            if c.is_ascii_lowercase() {
                let up = c.to_ascii_uppercase();
                if up != c {
                    return glyph_rows(up);
                }
            } else if c.is_ascii_uppercase() {
                let low = c.to_ascii_lowercase();
                if low != c {
                    return glyph_rows(low);
                }
            }
            [
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
            ]
        }
    }
}

/// Metnin verilen ölçekte piksel genişliğini hesaplar (standart 2.0 piksel karakter aralığı).
pub fn text_width(text: &str, scale: f32) -> f32 {
    text_width_spaced(text, scale, 2.0)
}

/// Metnin özel karakter boşluğu (spacing) ile piksel genişliğini hesaplar.
/// Çok satırlı metinlerde (newline içeren) en geniş satırın piksel genişliğini döndürür.
pub fn text_width_spaced(text: &str, scale: f32, spacing: f32) -> f32 {
    let mut max_width = 0.0f32;
    let mut current_line_chars = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            let w = if current_line_chars == 0 {
                0.0
            } else {
                (current_line_chars as f32 * (5.0 + spacing) - spacing) * scale
            };
            if w > max_width {
                max_width = w;
            }
            current_line_chars = 0;
        } else if ch != '\r' {
            current_line_chars += 1;
        }
    }
    let w = if current_line_chars == 0 {
        0.0
    } else {
        (current_line_chars as f32 * (5.0 + spacing) - spacing) * scale
    };
    max_width.max(w)
}

/// Verilen metni özel karakter boşluğu (spacing) ile 5x7 vektörel bitmap font kullanarak instances listesine ekler.
/// '\n' karakterlerinde bir alt satıra geçiş yapar.
pub fn draw_text_spaced(
    instances: &mut Vec<QuadInstance>,
    text: &str,
    start_x: f32,
    start_y: f32,
    scale: f32,
    spacing: f32,
    color: [u8; 4],
) {
    let char_width = 5.0 * scale;
    let char_spacing = spacing * scale;
    let line_height = (7.0 + 2.0) * scale;
    let mut cursor_x = start_x;
    let mut cursor_y = start_y;

    for ch in text.chars() {
        if ch == '\n' {
            cursor_x = start_x;
            cursor_y += line_height;
            continue;
        }
        if ch == '\r' {
            continue;
        }
        if ch == ' ' {
            cursor_x += char_width + char_spacing;
            continue;
        }
        if ch == '\t' {
            cursor_x += (char_width + char_spacing) * 4.0;
            continue;
        }

        let rows = glyph_rows(ch);
        for (r_idx, &row) in rows.iter().enumerate() {
            let py = cursor_y + (r_idx as f32) * scale;
            for c_idx in 0..5 {
                if (row & (1 << (4 - c_idx))) != 0 {
                    let px = cursor_x + (c_idx as f32) * scale;
                    instances.push(QuadInstance::new([px, py], [scale, scale], color, 0));
                }
            }
        }
        cursor_x += char_width + char_spacing;
    }
}

/// Verilen metni 5x7 vektörel bitmap font ile instances listesine ekler.
pub fn draw_text(
    instances: &mut Vec<QuadInstance>,
    text: &str,
    start_x: f32,
    start_y: f32,
    scale: f32,
    color: [u8; 4],
) {
    draw_text_spaced(instances, text, start_x, start_y, scale, 2.0, color);
}

/// fontdue ile ttf/otf dosyalarından doğrudan rasterize edilmiş dinamik TrueType font yapısı.
pub struct TrueTypeFont {
    font: fontdue::Font,
}

impl TrueTypeFont {
    /// Ham TTF bayt dizisinden font nesnesi yükler.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())?;
        Ok(Self { font })
    }

    /// Disk üzerindeki TTF/OTF dosya yolundan font nesnesi yükler.
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Font dosyasi okunamadi: {e}"))?;
        Self::from_bytes(&bytes).map_err(|e| format!("Font cozumlenemedi: {e}"))
    }

    /// Verilen metnin genişliğini (piksel cinsinden) hesaplar.
    pub fn measure_text(&self, text: &str, px_size: f32) -> f32 {
        let mut total_w = 0.0f32;
        for ch in text.chars() {
            if ch == ' ' {
                total_w += px_size * 0.32;
            } else {
                let metrics = self.font.metrics(ch, px_size);
                total_w += metrics.advance_width;
            }
        }
        total_w
    }

    /// Verilen metni dinamik olarak rasterize ederek quad instances olarak ekler.
    /// Alpha eşikleme ve parlaklık ölçeklemesi ile net, keskin pikseller çizer.
    pub fn draw_text(
        &self,
        instances: &mut Vec<QuadInstance>,
        text: &str,
        start_x: f32,
        start_y: f32,
        px_size: f32,
        color: [u8; 4],
    ) {
        let mut cursor_x = start_x;
        let line_height = px_size * 1.25;
        let mut cursor_y = start_y;

        for ch in text.chars() {
            if ch == '\n' {
                cursor_x = start_x;
                cursor_y += line_height;
                continue;
            }
            if ch == '\r' {
                continue;
            }
            if ch == ' ' {
                cursor_x += (px_size * 0.32).round().max(4.0);
                continue;
            }

            let (metrics, bitmap) = self.font.rasterize(ch, px_size);
            if metrics.width > 0 && metrics.height > 0 {
                let glyph_x = cursor_x + metrics.xmin as f32;
                // Fontdue ymin yukarı doğru koordinattır, baseline hizalaması:
                let glyph_y =
                    cursor_y + (px_size * 0.8) - (metrics.height as f32 + metrics.ymin as f32);

                for r in 0..metrics.height {
                    for c in 0..metrics.width {
                        let coverage = bitmap[r * metrics.width + c];
                        if coverage >= 64 {
                            let mut px_col = color;
                            let alpha_factor = coverage as f32 / 255.0;
                            px_col[3] = ((color[3] as f32) * alpha_factor).round() as u8;
                            instances.push(QuadInstance::new(
                                [glyph_x + c as f32, glyph_y + r as f32],
                                [1.0, 1.0],
                                px_col,
                                0,
                            ));
                        }
                    }
                }
            }

            cursor_x += metrics
                .advance_width
                .round()
                .max(metrics.width as f32 + 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_rows_lowercase_distinct() {
        let v_upper = glyph_rows('V');
        let v_lower = glyph_rows('v');
        assert_ne!(v_upper, [0; 7]);
        assert_ne!(v_lower, [0; 7]);
        // Lowercase 'v' must not be equal to dropdown arrow ▼
        let arrow = glyph_rows('▼');
        assert_ne!(v_lower, arrow, "'v' harfi asagi ok ile karismamali");
    }

    #[test]
    fn test_all_ascii_lowercase_and_digits_defined() {
        for c in 'a'..='z' {
            let rows = glyph_rows(c);
            assert_ne!(rows, [0; 7], "Kucuk harf '{}' icin glif bulunamadi", c);
        }
        for c in '0'..='9' {
            let rows = glyph_rows(c);
            assert_ne!(rows, [0; 7], "Sayi '{}' icin glif bulunamadi", c);
        }
    }

    #[test]
    fn test_turkish_glyphs() {
        for ch in ['ç', 'Ç', 'ğ', 'Ğ', 'ı', 'İ', 'ö', 'Ö', 'ş', 'Ş', 'ü', 'Ü'] {
            let rows = glyph_rows(ch);
            assert_ne!(
                rows, [0; 7],
                "Turkce karakter '{}' icin glif bulunamadi",
                ch
            );
        }
    }

    #[test]
    fn test_text_width_calculation() {
        assert_eq!(text_width("", 1.0), 0.0);
        assert_eq!(text_width("A", 1.0), 5.0);
        assert_eq!(text_width("AB", 1.0), 12.0); // 5 + 2 + 5
        assert_eq!(text_width("AB", 2.0), 24.0);
        assert_eq!(text_width_spaced("AB", 1.0, 3.0), 13.0); // 5 + 3 + 5
    }

    #[test]
    fn test_draw_text_generates_instances() {
        let mut instances = Vec::new();
        draw_text(&mut instances, "A", 0.0, 0.0, 1.0, [255, 255, 255, 255]);
        assert!(!instances.is_empty());
        for inst in &instances {
            assert_eq!(inst.color, u32::from_le_bytes([255, 255, 255, 255]));
        }

        let mut instances_spaced = Vec::new();
        draw_text_spaced(
            &mut instances_spaced,
            "AB",
            0.0,
            0.0,
            1.0,
            4.0,
            [255, 255, 255, 255],
        );
        assert!(instances_spaced.len() > instances.len());
    }

    #[test]
    fn test_ui_and_playback_glyphs() {
        for icon in ['▶', '⏸', '⏹', '⏭', '⏮', '✓', '✕', '▼', '▲', '❚'] {
            let rows = glyph_rows(icon);
            assert_ne!(rows, [0; 7], "UI ikon '{}' icin glif tanimlanmamis", icon);
        }
    }

    #[test]
    fn test_multiline_text_spacing_and_drawing() {
        let multi = "ABC\nDE";
        let width = text_width(multi, 1.0);
        assert_eq!(width, text_width("ABC", 1.0));

        let mut instances = Vec::new();
        draw_text(&mut instances, multi, 0.0, 0.0, 1.0, [255, 255, 255, 255]);
        assert!(!instances.is_empty());
        // Second line should have y offset of (7 + 2) * scale = 9.0
        let has_second_line = instances.iter().any(|inst| inst.pos[1] >= 9.0);
        assert!(
            has_second_line,
            "Cok satirli metinde ikinci satir cizilmeli"
        );
    }

    #[test]
    fn test_turkish_s_cedilla_structure() {
        let s_rows = glyph_rows('s');
        let s_cedilla_rows = glyph_rows('ş');
        assert_ne!(s_cedilla_rows, [0; 7]);
        assert_ne!(s_rows, s_cedilla_rows, "s ve ş farkli glif olmali");
        // Row 6 of ş must have the cedilla tail
        assert_eq!(
            s_cedilla_rows[6], 0b00100,
            "ş glifinin 6. satiri kuyruk olmali"
        );
        // Row 5 of ş must preserve the bottom curve of s
        assert_eq!(
            s_cedilla_rows[5], 0b11110,
            "ş glifinin 5. satiri s harfinin alt kavisini korumali"
        );
    }

    #[test]
    fn test_truetype_font_loading_and_drawing() {
        let font_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/fonts/Roboto-Regular.ttf"
        );
        let font = TrueTypeFont::from_file(font_path).expect("Roboto-Regular.ttf yuklenebilmeli");
        let w = font.measure_text("Project Rugeng", 16.0);
        assert!(w > 50.0, "Metin genisligi hesaplanmali");

        let mut instances = Vec::new();
        font.draw_text(
            &mut instances,
            "Hello 2D",
            0.0,
            0.0,
            16.0,
            [255, 255, 255, 255],
        );
        assert!(
            !instances.is_empty(),
            "TrueType font quad instance uretmeli"
        );
    }
}
