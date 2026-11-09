//! Editor Color Theme & Procedural Quad Icons
use rugeng_render::pipeline_2d::QuadInstance;

/// Modern Antrasit Koyu Ton Renk Teması (Godot 4 / VS Code / Blender paleti)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorTheme {
    pub bg_canvas: [u8; 4],
    pub bg_panel: [u8; 4],
    pub bg_panel_alt: [u8; 4],
    pub tab_active: [u8; 4],
    pub tab_inactive: [u8; 4],
    pub tab_hover: [u8; 4],
    pub border: [u8; 4],
    pub border_subtle: [u8; 4],
    pub accent: [u8; 4],
    pub accent_subtle: [u8; 4],
    pub accent_active: [u8; 4],
    pub text_primary: [u8; 4],
    pub text_secondary: [u8; 4],
    pub text_muted: [u8; 4],
    pub success: [u8; 4],
    pub warning: [u8; 4],
    pub danger: [u8; 4],
    pub purple: [u8; 4],
    pub grid_line: [u8; 4],
    pub card_bg: [u8; 4],
    pub card_header: [u8; 4],
    pub btn_normal: [u8; 4],
    pub btn_hover: [u8; 4],
    pub input_bg: [u8; 4],
}

impl EditorTheme {
    pub const fn dark() -> Self {
        Self {
            bg_canvas: [35, 35, 35, 255],         // #232323 Tuval arka planı
            bg_panel: [40, 40, 40, 255],          // #282828 Paneller ve dock'lar
            bg_panel_alt: [32, 32, 32, 255],      // #202020 Alternatif panel/liste
            tab_active: [48, 48, 48, 255],        // #303030 Aktif sekme
            tab_inactive: [32, 32, 32, 255],      // #202020 Pasif sekme
            tab_hover: [55, 55, 55, 255],         // #373737 Sekme hover
            border: [26, 26, 26, 255],            // #1a1a1a Derin kenarlık
            border_subtle: [45, 45, 45, 255],     // #2d2d2d İnce yumuşak kenarlık
            accent: [74, 158, 255, 255],          // Temiz modern mavi
            accent_subtle: [42, 65, 92, 255],     // Vurgulu yumuşak seçim
            accent_active: [52, 85, 122, 255],    // Aktif mavi seçim
            text_primary: [238, 238, 238, 255],   // #eeeeee Net beyazımsı metin
            text_secondary: [168, 168, 168, 255], // #a8a8a8 İkincil yumuşak gri
            text_muted: [115, 115, 115, 255],     // #737373 Soluk etiketler
            success: [85, 195, 125, 255],         // Doğal başarı yeşili
            warning: [235, 185, 65, 255],         // Doğal uyarı sarısı
            danger: [235, 85, 85, 255],           // Doğal hata kırmızısı
            purple: [175, 115, 245, 255],         // Yumuşak mor
            grid_line: [46, 46, 46, 255],         // #2e2e2e Izgara çizgisi
            card_bg: [34, 34, 34, 255],           // #222222 Kart içi
            card_header: [42, 42, 42, 255],       // #2a2a2a Kart başlığı
            btn_normal: [46, 46, 46, 255],        // #2e2e2e Buton normal
            btn_hover: [58, 58, 58, 255],         // #3a3a3a Buton hover
            input_bg: [28, 28, 28, 255],          // #1c1c1c Girdi kutusu
        }
    }
}

// Re-export play/pause/stop/step from rugeng_render to avoid duplication
pub use rugeng_render::icons::{draw_icon_pause, draw_icon_play, draw_icon_step, draw_icon_stop};

// Viewport tool icons wrapper delegating to rugeng_render with default tool size (12.0)
#[inline]
pub fn draw_icon_select(instances: &mut Vec<QuadInstance>, x: f32, y: f32, color: [u8; 4]) {
    rugeng_render::icons::draw_icon_select(instances, x, y, 12.0, color);
}

#[inline]
pub fn draw_icon_move(instances: &mut Vec<QuadInstance>, x: f32, y: f32, color: [u8; 4]) {
    rugeng_render::icons::draw_icon_move(instances, x, y, 12.0, color);
}

#[inline]
pub fn draw_icon_rotate(instances: &mut Vec<QuadInstance>, x: f32, y: f32, color: [u8; 4]) {
    rugeng_render::icons::draw_icon_rotate(instances, x, y, 12.0, color);
}

#[inline]
pub fn draw_icon_scale(instances: &mut Vec<QuadInstance>, x: f32, y: f32, color: [u8; 4]) {
    rugeng_render::icons::draw_icon_scale(instances, x, y, 12.0, color);
}

#[inline]
pub fn draw_icon_brush(instances: &mut Vec<QuadInstance>, x: f32, y: f32, color: [u8; 4]) {
    rugeng_render::icons::draw_icon_brush(instances, x, y, 12.0, color);
}

#[inline]
pub fn draw_icon_eraser(instances: &mut Vec<QuadInstance>, x: f32, y: f32, color: [u8; 4]) {
    rugeng_render::icons::draw_icon_eraser(instances, x, y, 12.0, color);
}
