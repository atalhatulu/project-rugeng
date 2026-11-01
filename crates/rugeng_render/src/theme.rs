//! Rugeng Renk Paleti ve Tema Sistemi (Godot 4 / VS Code Koyu Tonları).

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    /// 2D Tuval arka plan rengi (#181a20).
    pub bg_canvas: [u8; 4],
    /// Yan paneller, dock'lar ve çubuklar (#1e2026).
    pub bg_dock: [u8; 4],
    /// Aktif sekme ve başlık rengi (#262a34).
    pub bg_header_act: [u8; 4],
    /// Pasif sekme ve başlık rengi (#1a1c22).
    pub bg_header_inact: [u8; 4],
    /// Liste elemanı arka planı.
    pub bg_item: [u8; 4],
    /// Seçili liste elemanı veya düğüm arka planı.
    pub bg_item_selected: [u8; 4],
    /// Standart buton arka planı.
    pub bg_button: [u8; 4],
    /// Vurgulu / aktif buton arka planı.
    pub bg_button_active: [u8; 4],
    /// Buton üzerine gelme (hover) rengi.
    pub bg_button_hover: [u8; 4],
    /// İnce koyu ayırıcı ve kenarlık rengi (#14161b).
    pub border: [u8; 4],
    /// Açık kenarlık ve kılavuz çizgi rengi (#262a32).
    pub border_light: [u8; 4],
    /// Net açık metin rengi (#e6ebf5).
    pub text_norm: [u8; 4],
    /// İkincil soluk metin rengi (#8c94a2).
    pub text_dim: [u8; 4],
    /// Vurgu rengi: Canlı Neon Godot Mavisi (#4493f8).
    pub accent: [u8; 4],
    /// Vurgu rengi üzerine gelme (hover).
    pub accent_hover: [u8; 4],
    /// Durum yeşili (oynatma / başarı) (#48c78e).
    pub success: [u8; 4],
    /// Durum kırmızısı (durdurma / hata) (#ef5a5a).
    pub danger: [u8; 4],
    /// Durum sarısı (duraklatma / uyarı) (#eab33a).
    pub warning: [u8; 4],
    /// İkincil mor vurgu (script / animasyon) (#a855f7).
    pub purple: [u8; 4],
}

impl Theme {
    pub const DARK: Self = Self {
        bg_canvas: [35, 35, 35, 255],         // #232323 Tuval arka planı
        bg_dock: [40, 40, 40, 255],           // #282828 Paneller ve dock'lar
        bg_header_act: [48, 48, 48, 255],     // #303030 Aktif sekme ve başlık
        bg_header_inact: [32, 32, 32, 255],   // #202020 Pasif sekme ve başlık
        bg_item: [34, 34, 34, 255],           // #222222 Liste elemanı
        bg_item_selected: [52, 85, 122, 255], // Vurgulu mavi seçim
        bg_button: [46, 46, 46, 255],         // #2e2e2e Buton normal
        bg_button_active: [52, 85, 122, 255], // Aktif buton
        bg_button_hover: [58, 58, 58, 255],   // #3a3a3a Buton hover
        border: [26, 26, 26, 255],            // #1a1a1a İnce koyu ayırıcı
        border_light: [45, 45, 45, 255],      // #2d2d2d Açık kenarlık ve kılavuz çizgisi
        text_norm: [238, 238, 238, 255],      // #eeeeee Net açık metin
        text_dim: [168, 168, 168, 255],       // #a8a8a8 İkincil yumuşak gri
        accent: [74, 158, 255, 255],          // Temiz modern mavi
        accent_hover: [96, 175, 255, 255],    // Vurgulu mavi hover
        success: [85, 195, 125, 255],         // Başarı yeşili
        danger: [235, 85, 85, 255],           // Tehlike kırmızısı
        warning: [235, 185, 65, 255],         // Uyarı sarısı
        purple: [175, 115, 245, 255],         // Mor
    };

    /// Vulkan RenderPass / dynamic rendering temizleme rengi [0.0..1.0].
    pub fn clear_color_f32(&self) -> [f32; 4] {
        [
            self.bg_canvas[0] as f32 / 255.0,
            self.bg_canvas[1] as f32 / 255.0,
            self.bg_canvas[2] as f32 / 255.0,
            self.bg_canvas[3] as f32 / 255.0,
        ]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::DARK
    }
}
