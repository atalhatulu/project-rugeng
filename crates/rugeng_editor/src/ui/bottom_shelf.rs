#![allow(unused_imports)]
#![allow(unused_variables, dead_code)]
use crate::theme::*;
use crate::types::*;
use crate::ui::dock_layout::DockLayout;
use crate::App;
use glam::Vec2;
use rugeng_core::{
    interpolate_particle_color, Animation2DComponent, Collider2D, Entity, Mesh3DComponent,
    NodeName, Parent, ParticleEmitterComponent, ScriptComponent, SpriteColor, SpriteTexture,
    Text2DComponent, TileType, Tilemap2DComponent, Transform2D, Transform3D, Velocity2D, ZIndex2D,
};
use rugeng_render::font;
use rugeng_render::pipeline_2d::QuadInstance;

impl App {
    pub(crate) fn render_bottom_shelf(
        &self,
        instances: &mut Vec<QuadInstance>,
        layout: &DockLayout,
    ) {
        let selected_e = self.editor.selected_entity;
        let screen_w = layout.screen_w;
        let screen_h = layout.screen_h;
        let menu_h = layout.menu_h;
        let left_w = layout.left_w;
        let right_w = layout.right_w;
        let bottom_h = layout.bottom_h;
        let left_top_h = layout.left_top_h;
        let left_bot_y = layout.left_bot_y;
        let left_bot_h = layout.left_bot_h;
        let vp_x = layout.vp_x;
        let vp_w = layout.vp_w;
        let scene_tab_h = layout.scene_tab_h;
        let vp_tool_h = layout.vp_tool_h;
        let vp_canvas_y = layout.vp_canvas_y;
        let vp_canvas_h = layout.vp_canvas_h;

        let theme = self.editor.theme;
        let bg_dock = theme.bg_panel;
        let bg_header_inact = theme.tab_inactive;
        let bg_header_act = theme.tab_active;
        let border_col = theme.border;
        let text_norm = theme.text_primary;
        let text_dim = theme.text_secondary;
        let godot_blue = theme.accent;
        let godot_red = theme.danger;
        let godot_green = theme.success;
        let godot_yellow = theme.warning;

        // =========================================================================
        // 4. ALT PANEL: KONSOL & HATA AYIKLAMA (SADECE TUVALIN ALTINDA!)
        // =========================================================================
        let bot_y = screen_h - bottom_h;
        instances.push(QuadInstance::new(
            [vp_x, bot_y],
            [vp_w, bottom_h],
            theme.bg_panel_alt,
            0,
        ));
        instances.push(QuadInstance::new([vp_x, bot_y], [vp_w, 1.0], border_col, 0));

        // Sekmeye göre içerik göster: Output (Konsol Logları) veya Debugger (Sistem Metrikleri)
        if self.editor.bottom_shelf_tab == BottomShelfTab::Debugger {
            let mut dbg_y = bot_y + 8.0;
            let entity_count = self.registry.len();
            let fps = if self.frame_time_ms > 0.001 {
                1000.0 / self.frame_time_ms
            } else {
                0.0
            };
            let arena_used = self.arena.allocated_bytes();

            let metric_1 = format!(
                "Varlik (Entity): {} | Kare Suresi: {:.2}ms ({:.0} FPS)",
                entity_count, self.frame_time_ms, fps
            );
            let metric_2 = format!(
                "Kare Ring/Arena: {} KB / {} KB | Cizilen Quad: {}",
                arena_used / 1024,
                6400,
                instances.len()
            );
            let metric_3 = format!(
                "Simulasyon: {} | Adim Araligi: 1/60s (16.6ms)",
                if self.editor.is_playing {
                    "CALISIYOR"
                } else {
                    "DURDURULDU"
                }
            );

            font::draw_text(instances, &metric_1, vp_x + 10.0, dbg_y, 1.3, theme.accent);
            dbg_y += 18.0;
            font::draw_text(instances, &metric_2, vp_x + 10.0, dbg_y, 1.3, godot_green);
            dbg_y += 18.0;
            font::draw_text(instances, &metric_3, vp_x + 10.0, dbg_y, 1.3, godot_yellow);
        } else {
            // Konsol Metin Alanı (Loglar)
            let mut log_y = bot_y + 8.0;
            let recent_logs = self.editor.logs.iter().rev().take(5).collect::<Vec<_>>();
            for msg in recent_logs.into_iter().rev() {
                let lvl_color = match msg.level {
                    "WARN" => godot_yellow,
                    "ERROR" => godot_red,
                    _ => godot_blue,
                };
                font::draw_text(instances, &msg.text, vp_x + 10.0, log_y, 1.3, lvl_color);
                log_y += 18.0;
            }
        }

        // Alt Sekmeler Çubuğu (Output & Debugger)
        let btm_tab_y = screen_h - 24.0;
        instances.push(QuadInstance::new(
            [vp_x, btm_tab_y],
            [vp_w, 24.0],
            bg_dock,
            0,
        ));
        instances.push(QuadInstance::new(
            [vp_x, btm_tab_y],
            [vp_w, 1.0],
            border_col,
            0,
        ));

        let btm_tabs = [
            ("Output", BottomShelfTab::Output),
            ("Debugger", BottomShelfTab::Debugger),
        ];

        let mut b_x = vp_x + 8.0;
        for (lbl, tab) in btm_tabs {
            let is_cur = self.editor.bottom_shelf_tab == tab;
            let tab_bg = if is_cur { bg_header_act } else { bg_dock };
            let tab_txt = if is_cur { text_norm } else { text_dim };
            instances.push(QuadInstance::new(
                [b_x, btm_tab_y + 1.0],
                [78.0, 23.0],
                tab_bg,
                0,
            ));
            if is_cur {
                instances.push(QuadInstance::new(
                    [b_x, btm_tab_y + 1.0],
                    [78.0, 1.5],
                    godot_blue,
                    0,
                ));
            }
            font::draw_text(instances, lbl, b_x + 8.0, btm_tab_y + 6.0, 1.3, tab_txt);
            b_x += 82.0;
        }

        // Sağdaki Gerçek Uyarı, Hata ve Versiyon Rozetleri
        let warn_count = self
            .editor
            .logs
            .iter()
            .filter(|l| l.level == "WARN")
            .count();
        let err_count = self
            .editor
            .logs
            .iter()
            .filter(|l| l.level == "ERROR")
            .count();
        let status_x = vp_x + vp_w - 240.0;

        // Uyarı rozeti
        instances.push(QuadInstance::new(
            [status_x, btm_tab_y + 3.0],
            [36.0, 18.0],
            [50, 40, 22, 255],
            0,
        ));
        instances.push(QuadInstance::new(
            [status_x, btm_tab_y + 3.0],
            [36.0, 1.0],
            godot_yellow,
            0,
        ));
        font::draw_text(
            instances,
            &format!("! {}", warn_count),
            status_x + 6.0,
            btm_tab_y + 6.0,
            1.3,
            godot_yellow,
        );

        // Hata rozeti
        instances.push(QuadInstance::new(
            [status_x + 40.0, btm_tab_y + 3.0],
            [36.0, 18.0],
            [50, 24, 26, 255],
            0,
        ));
        instances.push(QuadInstance::new(
            [status_x + 40.0, btm_tab_y + 3.0],
            [36.0, 1.0],
            godot_red,
            0,
        ));
        font::draw_text(
            instances,
            &format!("X {}", err_count),
            status_x + 46.0,
            btm_tab_y + 6.0,
            1.3,
            godot_red,
        );

        // Motor Güncel Versiyon Rozeti (Beta)
        font::draw_text(
            instances,
            "RugEng v0.1.0-beta",
            status_x + 86.0,
            btm_tab_y + 6.0,
            1.25,
            theme.accent,
        );
    }
}

impl crate::ui::Panel for crate::ui::BottomShelfPanel {
    fn handle_click(
        &mut self,
        app: &mut App,
        mouse: Vec2,
        layout: &DockLayout,
        _double_click: bool,
    ) -> bool {
        let screen_h = layout.screen_h;
        let bottom_h = layout.bottom_h;
        let vp_x = layout.vp_x;
        let vp_w = layout.vp_w;

        if mouse.x >= vp_x && mouse.x <= vp_x + vp_w && mouse.y >= screen_h - bottom_h {
            let btm_tab_y = screen_h - 24.0;
            if mouse.y >= btm_tab_y {
                let mut b_x = vp_x + 8.0;
                let btm_tabs = [BottomShelfTab::Output, BottomShelfTab::Debugger];
                for tab in btm_tabs {
                    if mouse.x >= b_x && mouse.x <= b_x + 78.0 {
                        app.editor.bottom_shelf_tab = tab;
                        return true;
                    }
                    b_x += 82.0;
                }
            }
            return true;
        }
        false
    }
}
