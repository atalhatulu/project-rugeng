#![allow(unused_imports)]
use glam::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct DockLayout {
    pub screen_w: f32,
    pub screen_h: f32,
    pub menu_h: f32,
    pub left_w: f32,
    pub right_w: f32,
    pub bottom_h: f32,
    pub left_top_h: f32,
    pub left_bot_y: f32,
    pub left_bot_h: f32,
    pub vp_x: f32,
    pub vp_w: f32,
    pub scene_tab_h: f32,
    pub vp_tool_h: f32,
    pub vp_canvas_y: f32,
    pub vp_canvas_h: f32,
    pub insp_x: f32,
    pub insp_h: f32,
}

impl DockLayout {
    pub const DEFAULT_LEFT_W: f32 = 260.0;
    pub const DEFAULT_RIGHT_W: f32 = 300.0;
    pub const DEFAULT_BOTTOM_H: f32 = 160.0;
    pub const SPLITTER_HIT_THICKNESS: f32 = 3.0;

    #[allow(dead_code)]
    pub fn new(screen_w: f32, screen_h: f32) -> Self {
        Self::with_custom_sizes(
            screen_w,
            screen_h,
            Self::DEFAULT_LEFT_W,
            Self::DEFAULT_RIGHT_W,
            Self::DEFAULT_BOTTOM_H,
        )
    }

    pub fn with_custom_sizes(
        screen_w: f32,
        screen_h: f32,
        left_w: f32,
        right_w: f32,
        bottom_h: f32,
    ) -> Self {
        let menu_h = 30.0;
        let left_w = left_w.clamp(160.0, (screen_w * 0.45).max(160.0));
        let right_w = right_w.clamp(180.0, (screen_w * 0.45).max(180.0));
        let bottom_h = bottom_h.clamp(80.0, (screen_h * 0.5).max(80.0));

        let left_top_h = (screen_h - menu_h) * 0.52;
        let left_bot_y = menu_h + left_top_h;
        let left_bot_h = (screen_h - menu_h) - left_top_h;

        let vp_x = left_w;
        let vp_w = (screen_w - left_w - right_w).max(100.0);
        let scene_tab_h = 26.0;
        let vp_tool_h = 24.0;
        let vp_canvas_y = menu_h + scene_tab_h + vp_tool_h;
        let vp_canvas_h = (screen_h - bottom_h - vp_canvas_y).max(100.0);

        let insp_x = screen_w - right_w;
        let insp_h = screen_h - menu_h;

        Self {
            screen_w,
            screen_h,
            menu_h,
            left_w,
            right_w,
            bottom_h,
            left_top_h,
            left_bot_y,
            left_bot_h,
            vp_x,
            vp_w,
            scene_tab_h,
            vp_tool_h,
            vp_canvas_y,
            vp_canvas_h,
            insp_x,
            insp_h,
        }
    }

    /// Sol splitter hattı (Left dock ile Viewport arası) üzerinde mi?
    #[inline]
    pub fn is_over_left_splitter(&self, pos: Vec2) -> bool {
        (pos.x - self.left_w).abs() <= Self::SPLITTER_HIT_THICKNESS
            && pos.y >= self.menu_h
            && pos.y <= self.screen_h - self.bottom_h
    }

    /// Sağ splitter hattı (Viewport ile Inspector arası) üzerinde mi?
    #[inline]
    pub fn is_over_right_splitter(&self, pos: Vec2) -> bool {
        (pos.x - self.insp_x).abs() <= Self::SPLITTER_HIT_THICKNESS
            && pos.y >= self.menu_h
            && pos.y <= self.screen_h
    }

    /// Alt splitter hattı (Viewport ile Bottom shelf arası) üzerinde mi?
    #[inline]
    pub fn is_over_bottom_splitter(&self, pos: Vec2) -> bool {
        let bottom_y = self.screen_h - self.bottom_h;
        (pos.y - bottom_y).abs() <= Self::SPLITTER_HIT_THICKNESS
            && pos.x >= self.left_w
            && pos.x <= self.insp_x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_layout_custom_sizes_clamping() {
        // Çok küçük değerler girildiğinde min sınırlarına clamp etmeli
        let layout_min = DockLayout::with_custom_sizes(1280.0, 720.0, 10.0, 20.0, 15.0);
        assert_eq!(layout_min.left_w, 160.0);
        assert_eq!(layout_min.right_w, 180.0);
        assert_eq!(layout_min.bottom_h, 80.0);

        // Ekran boyutunu aşan aşırı büyük değerlerde maksimum sınırlarına clamp etmeli
        let layout_max = DockLayout::with_custom_sizes(1000.0, 600.0, 900.0, 900.0, 800.0);
        assert_eq!(layout_max.left_w, 1000.0 * 0.45);
        assert_eq!(layout_max.right_w, 1000.0 * 0.45);
        assert_eq!(layout_max.bottom_h, 600.0 * 0.5);

        // Viewport genişliği pozitif kalmalı
        assert!(layout_max.vp_w >= 100.0);
    }
}
