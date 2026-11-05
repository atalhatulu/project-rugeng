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
    pub(crate) fn render_viewport(&self, instances: &mut Vec<QuadInstance>, layout: &DockLayout) {
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
        // 1. ORTA ALAN: VIEWPORT (SAHNE SEKMLERİ, ARAÇ ÇUBUĞU, CETVEL, 2D TUVAL)
        // =========================================================================
        // A. Sahne Sekmeleri Çubuğu (Scene Tabs)
        instances.push(QuadInstance::new(
            [vp_x, menu_h],
            [vp_w, scene_tab_h],
            bg_header_inact,
            0,
        ));
        instances.push(QuadInstance::new(
            [vp_x, menu_h + scene_tab_h - 1.0],
            [vp_w, 1.0],
            border_col,
            0,
        ));

        // Tab 1: Aktif Sahne (Dinamik İsim ve * Kirli Bayrağı)
        let scene_title = format!(
            "{}{}",
            std::path::Path::new(&self.editor.current_scene_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            if self.editor.is_scene_dirty {
                " (*)"
            } else {
                ""
            }
        );
        let tab1_w = ((scene_title.len() as f32) * 7.5 + 36.0).max(120.0);
        instances.push(QuadInstance::new(
            [vp_x + 8.0, menu_h + 3.0],
            [tab1_w, scene_tab_h - 3.0],
            bg_header_act,
            0,
        ));
        instances.push(QuadInstance::new(
            [vp_x + 14.0, menu_h + 9.0],
            [8.0, 8.0],
            godot_blue,
            0,
        )); // 2D İkon
        font::draw_text(
            instances,
            &scene_title,
            vp_x + 28.0,
            menu_h + 8.0,
            1.3,
            text_norm,
        );

        // Tab 2: player.scene
        let tab2_x = vp_x + 8.0 + tab1_w + 4.0;
        instances.push(QuadInstance::new(
            [tab2_x, menu_h + 3.0],
            [90.0, scene_tab_h - 3.0],
            bg_header_inact,
            0,
        ));
        instances.push(QuadInstance::new(
            [tab2_x + 6.0, menu_h + 9.0],
            [8.0, 8.0],
            godot_yellow,
            0,
        ));
        font::draw_text(
            instances,
            "player.scene",
            tab2_x + 18.0,
            menu_h + 8.0,
            1.2,
            text_dim,
        );

        // [+] Sekme Ekle (Yeni Sahne Butonu)
        let plus_x = tab2_x + 94.0;
        instances.push(QuadInstance::new(
            [plus_x, menu_h + 3.0],
            [28.0, scene_tab_h - 3.0],
            theme.btn_normal,
            0,
        ));
        font::draw_text(
            instances,
            "+",
            plus_x + 10.0,
            menu_h + 8.0,
            1.5,
            godot_green,
        );

        // B. Viewport Alt Araç Çubuğu (Sub-Toolbar)
        instances.push(QuadInstance::new(
            [vp_x, menu_h + scene_tab_h],
            [vp_w, vp_tool_h],
            bg_dock,
            0,
        ));
        instances.push(QuadInstance::new(
            [vp_x, menu_h + scene_tab_h + vp_tool_h - 1.0],
            [vp_w, 1.0],
            border_col,
            0,
        ));

        let mut t_x = vp_x + 6.0;
        for &(tool, label, btn_w) in &VIEWPORT_TOOLS {
            let is_cur = self.editor.active_tool == tool;
            let btn_bg = if is_cur {
                theme.accent_subtle
            } else {
                theme.btn_normal
            };
            let icon_col = if is_cur { godot_blue } else { text_dim };
            let txt_col = if is_cur { text_norm } else { text_dim };

            instances.push(QuadInstance::new(
                [t_x, menu_h + scene_tab_h + 2.0],
                [btn_w, 20.0],
                btn_bg,
                0,
            ));
            instances.push(QuadInstance::new(
                [t_x, menu_h + scene_tab_h + 2.0],
                [btn_w, 1.0],
                if is_cur { godot_blue } else { border_col },
                0,
            ));
            if is_cur {
                instances.push(QuadInstance::new(
                    [t_x, menu_h + scene_tab_h + 20.0],
                    [btn_w, 1.5],
                    godot_blue,
                    0,
                ));
            }

            let icon_y = menu_h + scene_tab_h + 7.0;
            match tool {
                ViewportTool::Select => draw_icon_select(instances, t_x + 4.0, icon_y, icon_col),
                ViewportTool::Move => draw_icon_move(instances, t_x + 4.0, icon_y, icon_col),
                ViewportTool::Rotate => draw_icon_rotate(instances, t_x + 4.0, icon_y, icon_col),
                ViewportTool::Scale => draw_icon_scale(instances, t_x + 4.0, icon_y, icon_col),
                ViewportTool::TileBrush => draw_icon_brush(instances, t_x + 4.0, icon_y, icon_col),
                ViewportTool::TileEraser => {
                    draw_icon_eraser(instances, t_x + 4.0, icon_y, icon_col)
                }
            }

            let txt_scale = if label.len() > 6 { 0.9 } else { 0.95 };
            font::draw_text(
                instances,
                label,
                t_x + 16.0,
                menu_h + scene_tab_h + 5.0,
                txt_scale,
                txt_col,
            );
            t_x += btn_w + 4.0;
        }

        // Eğer Fırça veya Silgi aktifse karo paletini göster
        if self.editor.active_tool == ViewportTool::TileBrush
            || self.editor.active_tool == ViewportTool::TileEraser
        {
            // Ayırıcı dikey çizgi
            instances.push(QuadInstance::new(
                [t_x + 2.0, menu_h + scene_tab_h + 4.0],
                [1.0, 16.0],
                theme.border_subtle,
                0,
            ));
            t_x += 8.0;

            for &(lbl, tt, btn_w) in &TILE_PALETTE_ITEMS {
                let is_sel = self.editor.active_tile_type == tt;
                let c = tt.default_color();
                let bg_col = if is_sel {
                    [c[0], c[1], c[2], 255]
                } else {
                    theme.card_header
                };
                instances.push(QuadInstance::new(
                    [t_x, menu_h + scene_tab_h + 2.0],
                    [btn_w, 20.0],
                    bg_col,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [t_x, menu_h + scene_tab_h + 2.0],
                    [btn_w, 1.0],
                    if is_sel {
                        [255, 255, 255, 255]
                    } else {
                        border_col
                    },
                    0,
                ));
                let txt_scale = if lbl.len() > 3 { 0.95 } else { 1.0 };
                let txt_offset = if lbl.len() <= 2 {
                    11.0
                } else if lbl.len() == 3 {
                    8.0
                } else {
                    6.0
                };
                font::draw_text(
                    instances,
                    lbl,
                    t_x + txt_offset,
                    menu_h + scene_tab_h + 5.0,
                    txt_scale,
                    if is_sel { [20, 24, 30, 255] } else { text_norm },
                );
                t_x += btn_w + 4.0;
            }
        }

        // Sağa hizalı Grid Snap Butonu (Viewport sınırları içinde güvenli konumlandırma)
        let snap_x = vp_x + vp_w - 74.0;
        let is_snap = self.editor.snap_to_grid;
        let snap_bg = if is_snap {
            theme.accent_subtle
        } else {
            theme.btn_normal
        };
        instances.push(QuadInstance::new(
            [snap_x, menu_h + scene_tab_h + 2.0],
            [66.0, 20.0],
            snap_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [snap_x, menu_h + scene_tab_h + 2.0],
            [66.0, 1.0],
            if is_snap { godot_blue } else { border_col },
            0,
        ));
        if is_snap {
            instances.push(QuadInstance::new(
                [snap_x, menu_h + scene_tab_h + 20.0],
                [66.0, 1.5],
                godot_blue,
                0,
            ));
        }
        let snap_txt = if is_snap { "[#] SNAP" } else { "[#] Snap" };
        font::draw_text(
            instances,
            snap_txt,
            snap_x + 6.0,
            menu_h + scene_tab_h + 5.0,
            1.0,
            if is_snap { godot_blue } else { text_dim },
        );

        // C. 2D Tuval (Canvas Arka Planı & Izgara)
        if self.editor.workspace == WorkspaceMode::TwoD {
            instances.push(QuadInstance::new(
                [vp_x, vp_canvas_y],
                [vp_w, vp_canvas_h],
                theme.bg_canvas,
                0,
            ));

            let cam = &self.editor.camera;
            let eff_cam_pos = cam.get_effective_pos();
            let vp_center = Vec2::new(vp_x + vp_w * 0.5, vp_canvas_y + vp_canvas_h * 0.5);

            // İnce Izgara Çizgileri
            let grid_step = 64.0 * cam.zoom;
            let start_grid_x = vp_center.x + ((-eff_cam_pos.x * cam.zoom) % grid_step);
            let start_grid_y = vp_center.y + ((-eff_cam_pos.y * cam.zoom) % grid_step);

            let mut gx = start_grid_x;
            while gx < vp_x + vp_w {
                if gx >= vp_x {
                    instances.push(QuadInstance::new(
                        [gx, vp_canvas_y],
                        [1.0, vp_canvas_h],
                        theme.grid_line,
                        0,
                    ));
                }
                gx += grid_step;
            }
            let mut gx = start_grid_x - grid_step;
            while gx >= vp_x {
                instances.push(QuadInstance::new(
                    [gx, vp_canvas_y],
                    [1.0, vp_canvas_h],
                    theme.grid_line,
                    0,
                ));
                gx -= grid_step;
            }

            let mut gy = start_grid_y;
            while gy < vp_canvas_y + vp_canvas_h {
                if gy >= vp_canvas_y {
                    instances.push(QuadInstance::new(
                        [vp_x, gy],
                        [vp_w, 1.0],
                        theme.grid_line,
                        0,
                    ));
                }
                gy += grid_step;
            }
            let mut gy = start_grid_y - grid_step;
            while gy >= vp_canvas_y {
                instances.push(QuadInstance::new(
                    [vp_x, gy],
                    [vp_w, 1.0],
                    theme.grid_line,
                    0,
                ));
                gy -= grid_step;
            }

            // Koordinat Merkez Çizgileri (Godot: Yatay Kırmızı, Dikey Yeşil)
            let origin_x = vp_center.x - eff_cam_pos.x * cam.zoom;
            let origin_y = vp_center.y - eff_cam_pos.y * cam.zoom;

            if origin_x >= vp_x && origin_x <= vp_x + vp_w {
                instances.push(QuadInstance::new(
                    [origin_x, vp_canvas_y],
                    [1.5, vp_canvas_h],
                    theme.success,
                    0,
                ));
            }
            if origin_y >= vp_canvas_y && origin_y <= vp_canvas_y + vp_canvas_h {
                instances.push(QuadInstance::new(
                    [vp_x, origin_y],
                    [vp_w, 1.5],
                    theme.danger,
                    0,
                ));
            }

            // Sahnedeki Tilemap (Kareli Harita) Karolarını Çiz
            self.registry.view1_ref::<Tilemap2DComponent, _>(|_e, map| {
                let t_sz = map.tile_size * cam.zoom;
                for (&(gx, gy), data) in &map.tiles {
                    let world_pos = map.grid_to_world(gx, gy);
                    let screen_pos = vp_center + (world_pos - eff_cam_pos) * cam.zoom;
                    let draw_x = screen_pos.x - t_sz * 0.5;
                    let draw_y = screen_pos.y - t_sz * 0.5;
                    if draw_x + t_sz >= vp_x
                        && draw_x <= vp_x + vp_w
                        && draw_y + t_sz >= vp_canvas_y
                        && draw_y <= vp_canvas_h + vp_canvas_y
                    {
                        instances.push(QuadInstance::new(
                            [draw_x, draw_y],
                            [t_sz, t_sz],
                            data.color,
                            0,
                        ));
                        // Izgara kenarlık çizgisi
                        instances.push(QuadInstance::new(
                            [draw_x, draw_y],
                            [t_sz, 1.0],
                            [20, 24, 28, 90],
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [draw_x, draw_y],
                            [1.0, t_sz],
                            [20, 24, 28, 90],
                            0,
                        ));
                    }
                }
            });

            // Fırça / Silgi Önizleme Kutusu (Hover Tile Indicator)
            if self.editor.active_tool == ViewportTool::TileBrush
                || self.editor.active_tool == ViewportTool::TileEraser
            {
                let m_world = (self.editor.mouse_pos - vp_center) / cam.zoom + eff_cam_pos;
                let mut tile_sz = 32.0;
                if let Some(sel) = self.editor.selected_entity {
                    if let Some(tm) = self.registry.get::<Tilemap2DComponent>(sel) {
                        tile_sz = tm.tile_size;
                    }
                } else {
                    self.registry.view1_ref::<Tilemap2DComponent, _>(|_e, tm| {
                        tile_sz = tm.tile_size;
                    });
                }
                let gx = (m_world.x / tile_sz).floor() as i32;
                let gy = (m_world.y / tile_sz).floor() as i32;
                let center_world =
                    Vec2::new((gx as f32 + 0.5) * tile_sz, (gy as f32 + 0.5) * tile_sz);
                let p = vp_center + (center_world - eff_cam_pos) * cam.zoom;
                let sz = tile_sz * cam.zoom;
                let px = p.x - sz * 0.5;
                let py = p.y - sz * 0.5;
                if px + sz >= vp_x
                    && px <= vp_x + vp_w
                    && py + sz >= vp_canvas_y
                    && py <= vp_canvas_y + vp_canvas_h
                {
                    let hover_col = if self.editor.active_tool == ViewportTool::TileBrush {
                        let mut c = self.editor.active_tile_type.default_color();
                        c[3] = 160;
                        c
                    } else {
                        [228, 91, 91, 160]
                    };
                    instances.push(QuadInstance::new([px, py], [sz, sz], hover_col, 0));
                    instances.push(QuadInstance::new(
                        [px, py],
                        [sz, 1.5],
                        [255, 255, 255, 220],
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [px, py + sz - 1.5],
                        [sz, 1.5],
                        [255, 255, 255, 220],
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [px, py],
                        [1.5, sz],
                        [255, 255, 255, 220],
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [px + sz - 1.5, py],
                        [1.5, sz],
                        [255, 255, 255, 220],
                        0,
                    ));
                }
            }

            // Sahnedeki Varlıkları Çiz (Z-Index sıralamasına göre: düşükler arkada, yüksekler önde)
            let selected_e = self.editor.selected_entity;
            let atlas_ref = self.atlas.as_ref();

            let mut sorted_sprites: Vec<(Entity, Transform2D, SpriteColor, i32)> = Vec::new();
            self.registry
                .view2_ref_ref::<Transform2D, SpriteColor, _>(|e, tr, spr| {
                    let z = self.registry.get::<ZIndex2D>(e).map(|zi| zi.z).unwrap_or(0);
                    sorted_sprites.push((e, *tr, *spr, z));
                });
            sorted_sprites.sort_by_key(|item| item.3);

            for (e, tr, spr, _z) in sorted_sprites {
                let screen_pos = vp_center + (tr.pos - eff_cam_pos) * cam.zoom;
                let screen_size = tr.size * cam.zoom;
                let draw_x = screen_pos.x - screen_size.x * 0.5;
                let draw_y = screen_pos.y - screen_size.y * 0.5;

                if draw_x + screen_size.x >= vp_x
                    && draw_x <= vp_x + vp_w
                    && draw_y + screen_size.y >= vp_canvas_y
                    && draw_y <= vp_canvas_y + vp_canvas_h
                {
                    let sub_reg = self
                        .registry
                        .get::<SpriteTexture>(e)
                        .and_then(|st| atlas_ref.and_then(|a| a.get_region(&st.name)));

                    if let Some(sub) = sub_reg {
                        instances.push(QuadInstance::with_uv_rotated(
                            [draw_x, draw_y],
                            [screen_size.x, screen_size.y],
                            [sub.uv_min_packed, sub.uv_size_packed],
                            spr.color,
                            1,
                            tr.rotation,
                        ));
                    } else {
                        instances.push(QuadInstance::new_rotated(
                            [draw_x, draw_y],
                            [screen_size.x, screen_size.y],
                            spr.color,
                            0,
                            tr.rotation,
                        ));
                    }
                }
            }

            // Sahnedeki Metin Varlıklarını (Text2D) Çiz
            self.registry
                .view2_ref_ref::<Transform2D, Text2DComponent, _>(|_e, tr, txt| {
                    let screen_pos = vp_center + (tr.pos - eff_cam_pos) * cam.zoom;
                    if screen_pos.x >= vp_x
                        && screen_pos.x <= vp_x + vp_w
                        && screen_pos.y >= vp_canvas_y
                        && screen_pos.y <= vp_canvas_y + vp_canvas_h
                    {
                        font::draw_text(
                            instances,
                            &txt.text,
                            screen_pos.x,
                            screen_pos.y,
                            txt.scale * cam.zoom.max(0.6),
                            txt.color,
                        );
                    }
                });

            // Sahnedeki Parçacık Yayıcıları (ParticleEmitterComponent) Çiz
            self.registry
                .view1_ref::<ParticleEmitterComponent, _>(|_e, emitter| {
                    for p in &emitter.particles {
                        let t = 1.0 - (p.life / p.max_life).clamp(0.0, 1.0);
                        let col = interpolate_particle_color(p.color_start, p.color_end, t);
                        let p_screen = vp_center + (p.pos - eff_cam_pos) * cam.zoom;
                        let p_sz = p.size * cam.zoom;
                        let px = p_screen.x - p_sz * 0.5;
                        let py = p_screen.y - p_sz * 0.5;
                        if px + p_sz >= vp_x
                            && px <= vp_x + vp_w
                            && py + p_sz >= vp_canvas_y
                            && py <= vp_canvas_y + vp_canvas_h
                        {
                            instances.push(QuadInstance::new([px, py], [p_sz, p_sz], col, 0));
                        }
                    }
                });

            // Seçili Varlık Gizmo ve Seçim Çerçevesi (Tüm Varlık Tipleri İçin Görünür)
            if let Some(se) = selected_e {
                if let Some(tr) = self.registry.get::<Transform2D>(se) {
                    let screen_pos = vp_center + (tr.pos - eff_cam_pos) * cam.zoom;
                    let screen_size = tr.size * cam.zoom;
                    let draw_x = screen_pos.x - screen_size.x * 0.5;
                    let draw_y = screen_pos.y - screen_size.y * 0.5;

                    if draw_x + screen_size.x >= vp_x
                        && draw_x <= vp_x + vp_w
                        && draw_y + screen_size.y >= vp_canvas_y
                        && draw_y <= vp_canvas_y + vp_canvas_h
                    {
                        let border_col = [255, 230, 40, 255];
                        // Seçim Çerçevesi
                        instances.push(QuadInstance::new(
                            [draw_x - 2.0, draw_y - 2.0],
                            [screen_size.x + 4.0, 1.5],
                            border_col,
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [draw_x - 2.0, draw_y + screen_size.y + 0.5],
                            [screen_size.x + 4.0, 1.5],
                            border_col,
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [draw_x - 2.0, draw_y - 2.0],
                            [1.5, screen_size.y + 4.0],
                            border_col,
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [draw_x + screen_size.x + 0.5, draw_y - 2.0],
                            [1.5, screen_size.y + 4.0],
                            border_col,
                            0,
                        ));

                        // 4 Köşe Büyütme/Küçültme Tutamacı (Scale Handles)
                        let hsz = 7.0;
                        instances.push(QuadInstance::new(
                            [draw_x - hsz * 0.5, draw_y - hsz * 0.5],
                            [hsz, hsz],
                            [71, 140, 191, 255],
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [draw_x + screen_size.x - hsz * 0.5, draw_y - hsz * 0.5],
                            [hsz, hsz],
                            [71, 140, 191, 255],
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [draw_x - hsz * 0.5, draw_y + screen_size.y - hsz * 0.5],
                            [hsz, hsz],
                            [71, 140, 191, 255],
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [
                                draw_x + screen_size.x - hsz * 0.5,
                                draw_y + screen_size.y - hsz * 0.5,
                            ],
                            [hsz, hsz],
                            [71, 140, 191, 255],
                            0,
                        ));

                        // Merkez Eksen Okları (Taşıma / Translate Gizmo)
                        instances.push(QuadInstance::new(
                            [screen_pos.x, screen_pos.y - 1.0],
                            [28.0, 2.0],
                            godot_red,
                            0,
                        ));
                        instances.push(QuadInstance::new(
                            [screen_pos.x - 1.0, screen_pos.y],
                            [2.0, 28.0],
                            godot_green,
                            0,
                        ));

                        // Döndürme Tutamacı (Rotation Gizmo Heading Tip ve Yön Çizgisi)
                        let rot_len = 38.0;
                        let tip_x = screen_pos.x + tr.rotation.cos() * rot_len;
                        let tip_y = screen_pos.y + tr.rotation.sin() * rot_len;
                        for s in 1..=8 {
                            let t = s as f32 / 8.0;
                            let lx = screen_pos.x + (tip_x - screen_pos.x) * t;
                            let ly = screen_pos.y + (tip_y - screen_pos.y) * t;
                            instances.push(QuadInstance::new(
                                [lx - 1.0, ly - 1.0],
                                [2.0, 2.0],
                                [224, 169, 66, 180],
                                0,
                            ));
                        }
                        instances.push(QuadInstance::new(
                            [tip_x - 4.0, tip_y - 4.0],
                            [8.0, 8.0],
                            [224, 169, 66, 255],
                            0,
                        ));
                    }
                }
            }

            // Kutu Seçimi (Box Selection / Marquee Box)
            if let Some(box_sel) = self.editor.box_selection {
                let p1 = vp_center + (box_sel.start_world - cam.pos) * cam.zoom;
                let p2 = vp_center + (box_sel.current_world - cam.pos) * cam.zoom;
                let min_x = p1.x.min(p2.x).max(vp_x);
                let max_x = p1.x.max(p2.x).min(vp_x + vp_w);
                let min_y = p1.y.min(p2.y).max(vp_canvas_y);
                let max_y = p1.y.max(p2.y).min(vp_canvas_y + vp_canvas_h);
                let bw = max_x - min_x;
                let bh = max_y - min_y;

                if bw > 1.0 && bh > 1.0 {
                    // Yarı saydam iç alan
                    instances.push(QuadInstance::new(
                        [min_x, min_y],
                        [bw, bh],
                        [theme.accent[0], theme.accent[1], theme.accent[2], 50],
                        0,
                    ));
                    // Kenarlıklar
                    instances.push(QuadInstance::new(
                        [min_x, min_y],
                        [bw, 1.5],
                        [theme.accent[0], theme.accent[1], theme.accent[2], 220],
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [min_x, min_y + bh - 1.5],
                        [bw, 1.5],
                        [theme.accent[0], theme.accent[1], theme.accent[2], 220],
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [min_x, min_y],
                        [1.5, bh],
                        [theme.accent[0], theme.accent[1], theme.accent[2], 220],
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [min_x + bw - 1.5, min_y],
                        [1.5, bh],
                        [theme.accent[0], theme.accent[1], theme.accent[2], 220],
                        0,
                    ));
                }
            }

            // D. Üst ve Sol Piksel Cetvelleri (Pixel Rulers)
            // Üst Cetvel
            instances.push(QuadInstance::new(
                [vp_x, vp_canvas_y],
                [vp_w, 14.0],
                theme.bg_panel_alt,
                0,
            ));
            instances.push(QuadInstance::new(
                [vp_x, vp_canvas_y + 13.0],
                [vp_w, 1.0],
                border_col,
                0,
            ));

            let ruler_nums = [
                "-256", "-128", "0", "128", "256", "384", "512", "640", "768", "896", "1024",
                "1152",
            ];
            let mut r_screen_x = origin_x - 256.0 * cam.zoom;
            for num in ruler_nums {
                if r_screen_x >= vp_x + 10.0 && r_screen_x <= vp_x + vp_w - 30.0 {
                    instances.push(QuadInstance::new(
                        [r_screen_x, vp_canvas_y + 8.0],
                        [1.0, 5.0],
                        theme.border_subtle,
                        0,
                    ));
                    font::draw_text(
                        instances,
                        num,
                        r_screen_x - 10.0,
                        vp_canvas_y + 2.0,
                        1.1,
                        text_dim,
                    );
                }
                r_screen_x += 128.0 * cam.zoom;
            }

            // Sol Cetvel
            instances.push(QuadInstance::new(
                [vp_x, vp_canvas_y + 14.0],
                [16.0, vp_canvas_h - 14.0],
                theme.bg_panel_alt,
                0,
            ));
            instances.push(QuadInstance::new(
                [vp_x + 15.0, vp_canvas_y + 14.0],
                [1.0, vp_canvas_h - 14.0],
                border_col,
                0,
            ));

            let mut r_screen_y = origin_y;
            let y_nums = ["0", "128", "256", "384", "512"];
            for num in y_nums {
                if r_screen_y >= vp_canvas_y + 20.0
                    && r_screen_y <= vp_canvas_y + vp_canvas_h - 10.0
                {
                    instances.push(QuadInstance::new(
                        [vp_x + 10.0, r_screen_y],
                        [5.0, 1.0],
                        theme.border_subtle,
                        0,
                    ));
                    font::draw_text(instances, num, vp_x + 2.0, r_screen_y - 6.0, 1.1, text_dim);
                }
                r_screen_y += 128.0 * cam.zoom;
            }

            // E. Yüzen Zoom Rozeti (Godot Ekranındaki "[-] 82.6 % [+]")
            let zoom_box_x = vp_x + 28.0;
            let zoom_box_y = vp_canvas_y + 20.0;
            instances.push(QuadInstance::new(
                [zoom_box_x, zoom_box_y],
                [116.0, 20.0],
                [
                    theme.bg_canvas[0],
                    theme.bg_canvas[1],
                    theme.bg_canvas[2],
                    230,
                ],
                0,
            ));
            instances.push(QuadInstance::new(
                [zoom_box_x, zoom_box_y],
                [116.0, 1.0],
                border_col,
                0,
            ));

            font::draw_text(
                instances,
                "[-]",
                zoom_box_x + 6.0,
                zoom_box_y + 5.0,
                1.3,
                text_dim,
            );
            font::draw_text(
                instances,
                &format!("{:.1} %", cam.zoom * 100.0),
                zoom_box_x + 32.0,
                zoom_box_y + 5.0,
                1.3,
                text_norm,
            );
            font::draw_text(
                instances,
                "[+]",
                zoom_box_x + 92.0,
                zoom_box_y + 5.0,
                1.3,
                text_dim,
            );
        } else {
            // 3D Modundayken 3D Viewport Alanı Üzerine Bilgi Göstergeleri
            let compass_x = vp_x + 28.0;
            let compass_y = vp_canvas_y + 20.0;
            instances.push(QuadInstance::new(
                [compass_x, compass_y],
                [200.0, 22.0],
                [
                    theme.bg_panel_alt[0],
                    theme.bg_panel_alt[1],
                    theme.bg_panel_alt[2],
                    220,
                ],
                0,
            ));
            font::draw_text(
                instances,
                "3D Viewport | PBR Active",
                compass_x + 8.0,
                compass_y + 5.0,
                1.2,
                godot_green,
            );

            // 3D Viewport Sağ Üst Oryantasyon Widget'ı (View Orientation Gizmo - Görsel 2)
            {
                let widget_cx = vp_x + vp_w - 55.0;
                let widget_cy = vp_canvas_y + 55.0;
                let widget_r = 38.0;

                // Yarı saydam dairesel/kare arka plan kartı
                instances.push(QuadInstance::new(
                    [widget_cx - widget_r - 4.0, widget_cy - widget_r - 4.0],
                    [(widget_r + 4.0) * 2.0, (widget_r + 4.0) * 2.0],
                    [theme.bg_panel[0], theme.bg_panel[1], theme.bg_panel[2], 210],
                    0,
                ));
                instances.push(QuadInstance::new(
                    [widget_cx - widget_r - 4.0, widget_cy - widget_r - 4.0],
                    [(widget_r + 4.0) * 2.0, 1.0],
                    theme.border_subtle,
                    0,
                ));

                let aspect = vp_w / vp_canvas_h.max(1.0);
                let render_cam = self.editor.camera_3d.to_render_camera(aspect);
                let view_mat = render_cam.view_matrix();

                // 6 eksen yönü: Pozitif (+X, +Y, +Z) ve Negatif (-X, -Y, -Z)
                // View matrisi ile kamera bakış uzayına dönüştür: x sağa, y yukarı, z kameraya doğru
                let axes: [(&str, glam::Vec3, [u8; 4], bool); 6] = [
                    ("-X", -glam::Vec3::X, [200, 70, 70, 160], false),
                    ("-Y", -glam::Vec3::Y, [90, 160, 40, 160], false),
                    ("-Z", -glam::Vec3::Z, [50, 100, 170, 160], false),
                    ("X", glam::Vec3::X, [235, 65, 85, 255], true),
                    ("Y", glam::Vec3::Y, [140, 205, 30, 255], true),
                    ("Z", glam::Vec3::Z, [45, 140, 245, 255], true),
                ];

                let mut projected_axes: Vec<(&str, Vec2, f32, [u8; 4], bool)> = axes
                    .iter()
                    .map(|&(label, dir, color, is_pos)| {
                        let cam_space = view_mat.transform_vector3(dir);
                        let screen_pos = Vec2::new(
                            widget_cx + cam_space.x * widget_r,
                            widget_cy - cam_space.y * widget_r, // Y yukarı olduğu için ekran Y ters
                        );
                        (label, screen_pos, cam_space.z, color, is_pos)
                    })
                    .collect();

                // Derinliğe göre sırala (Arkadan öne doğru çizim)
                projected_axes
                    .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

                // Merkez bağlantı çizgileri ve noktaları çiz
                for &(label, sp, _depth, color, is_pos) in &projected_axes {
                    let diff = sp - Vec2::new(widget_cx, widget_cy);
                    let len = diff.length();
                    if len > 2.0 {
                        let steps = 8;
                        for s in 1..=steps {
                            let t = s as f32 / steps as f32;
                            let p = Vec2::new(widget_cx, widget_cy) + diff * t;
                            let line_w = if is_pos { 2.0 } else { 1.5 };
                            instances.push(QuadInstance::new(
                                [p.x - line_w * 0.5, p.y - line_w * 0.5],
                                [line_w, line_w],
                                color,
                                0,
                            ));
                        }
                    }

                    // Uç noktası düğmesi
                    let node_sz = if is_pos { 18.0 } else { 12.0 };
                    instances.push(QuadInstance::new(
                        [sp.x - node_sz * 0.5, sp.y - node_sz * 0.5],
                        [node_sz, node_sz],
                        color,
                        0,
                    ));

                    // Pozitif eksenlerde X, Y, Z harf etiketleri
                    if is_pos {
                        font::draw_text(
                            instances,
                            label,
                            sp.x - 3.0,
                            sp.y - 4.5,
                            1.0,
                            [255, 255, 255, 255],
                        );
                    }
                }
            }

            // 3D Seçili Varlık Gizmosu (Görsel 1 Tarzı: Translate Okları, Rotate Çemberleri, Scale Tutamaçları)
            if let Some(se) = selected_e {
                if let Some(t3) = self.registry.get::<Transform3D>(se) {
                    let aspect = vp_w / vp_canvas_h.max(1.0);
                    let render_cam = self.editor.camera_3d.to_render_camera(aspect);
                    let vp_mat = render_cam.view_projection();

                    let project_pt = |pt: glam::Vec3| -> Option<Vec2> {
                        let clip = vp_mat * glam::Vec4::new(pt.x, pt.y, pt.z, 1.0);
                        if clip.w <= 0.05 {
                            return None;
                        }
                        let ndc_x = clip.x / clip.w;
                        let ndc_y = clip.y / clip.w;
                        if !(-1.2..=1.2).contains(&ndc_x) || !(-1.2..=1.2).contains(&ndc_y) {
                            return None;
                        }
                        let sx = vp_x + (ndc_x * 0.5 + 0.5) * vp_w;
                        let sy = vp_canvas_y + (1.0 - (ndc_y * 0.5 + 0.5)) * vp_canvas_h;
                        Some(Vec2::new(sx, sy))
                    };

                    let center = t3.translation;
                    if let Some(screen_center) = project_pt(center) {
                        let gold = [240, 195, 60, 255];
                        let red = [235, 65, 85, 255];
                        let green = [140, 205, 30, 255];
                        let blue = [45, 140, 245, 255];

                        // Merkez seçim noktası (Altın sarısı)
                        instances.push(QuadInstance::new(
                            [screen_center.x - 5.0, screen_center.y - 5.0],
                            [10.0, 10.0],
                            gold,
                            0,
                        ));

                        let tool = self.editor.active_tool;

                        // 1. DÖNDÜRME ÇEMBERLERİ (Rotate Gizmo - Görsel 1'deki Kırmızı, Yeşil, Mavi Yaylar)
                        if tool == ViewportTool::Rotate || tool == ViewportTool::Select {
                            let radius = 1.25;
                            let segments = 32;

                            // Yaw Çemberi (XZ Düzlemi / Y Ekseni Etrafında - Yeşil Çember)
                            let mut prev_pt: Option<Vec2> = None;
                            for i in 0..=segments {
                                let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                                let pt_world = center
                                    + glam::Vec3::new(
                                        angle.cos() * radius,
                                        0.0,
                                        angle.sin() * radius,
                                    );
                                if let Some(sp) = project_pt(pt_world) {
                                    if let Some(pp) = prev_pt {
                                        let diff = sp - pp;
                                        let len = diff.length();
                                        if len > 0.5 && len < 100.0 {
                                            for step in 0..=3 {
                                                let t = step as f32 / 3.0;
                                                let p = pp + diff * t;
                                                instances.push(QuadInstance::new(
                                                    [p.x - 1.0, p.y - 1.0],
                                                    [2.0, 2.0],
                                                    [green[0], green[1], green[2], 180],
                                                    0,
                                                ));
                                            }
                                        }
                                    }
                                    prev_pt = Some(sp);
                                }
                            }

                            // Pitch Çemberi (YZ Düzlemi / X Ekseni Etrafında - Kırmızı Çember)
                            let mut prev_pt: Option<Vec2> = None;
                            for i in 0..=segments {
                                let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                                let pt_world = center
                                    + glam::Vec3::new(
                                        0.0,
                                        angle.cos() * radius,
                                        angle.sin() * radius,
                                    );
                                if let Some(sp) = project_pt(pt_world) {
                                    if let Some(pp) = prev_pt {
                                        let diff = sp - pp;
                                        let len = diff.length();
                                        if len > 0.5 && len < 100.0 {
                                            for step in 0..=3 {
                                                let t = step as f32 / 3.0;
                                                let p = pp + diff * t;
                                                instances.push(QuadInstance::new(
                                                    [p.x - 1.0, p.y - 1.0],
                                                    [2.0, 2.0],
                                                    [red[0], red[1], red[2], 180],
                                                    0,
                                                ));
                                            }
                                        }
                                    }
                                    prev_pt = Some(sp);
                                }
                            }

                            // Roll Çemberi (XY Düzlemi / Z Ekseni Etrafında - Mavi Çember)
                            let mut prev_pt: Option<Vec2> = None;
                            for i in 0..=segments {
                                let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
                                let pt_world = center
                                    + glam::Vec3::new(
                                        angle.cos() * radius,
                                        angle.sin() * radius,
                                        0.0,
                                    );
                                if let Some(sp) = project_pt(pt_world) {
                                    if let Some(pp) = prev_pt {
                                        let diff = sp - pp;
                                        let len = diff.length();
                                        if len > 0.5 && len < 100.0 {
                                            for step in 0..=3 {
                                                let t = step as f32 / 3.0;
                                                let p = pp + diff * t;
                                                instances.push(QuadInstance::new(
                                                    [p.x - 1.0, p.y - 1.0],
                                                    [2.0, 2.0],
                                                    [blue[0], blue[1], blue[2], 180],
                                                    0,
                                                ));
                                            }
                                        }
                                    }
                                    prev_pt = Some(sp);
                                }
                            }
                        }

                        // 2. TAŞIMA VE BOYUTLANDIRMA EKSENLERİ (Translate & Scale Gizmo Okları/Kutuları)
                        let axis_len = 1.6;

                        // X Ekseni (Kırmızı)
                        if let Some(sx) = project_pt(center + glam::Vec3::X * axis_len) {
                            let diff = sx - screen_center;
                            for step in 1..=12 {
                                let t = step as f32 / 12.0;
                                let p = screen_center + diff * t;
                                instances.push(QuadInstance::new(
                                    [p.x - 1.0, p.y - 1.0],
                                    [2.5, 2.5],
                                    red,
                                    0,
                                ));
                            }
                            if tool == ViewportTool::Scale {
                                instances.push(QuadInstance::new(
                                    [sx.x - 4.5, sx.y - 4.5],
                                    [9.0, 9.0],
                                    red,
                                    0,
                                ));
                            } else {
                                instances.push(QuadInstance::new(
                                    [sx.x - 5.0, sx.y - 5.0],
                                    [10.0, 10.0],
                                    red,
                                    0,
                                ));
                            }
                            font::draw_text(instances, "X", sx.x + 6.0, sx.y - 5.0, 1.1, red);
                        }

                        // Y Ekseni (Yeşil)
                        if let Some(sy) = project_pt(center + glam::Vec3::Y * axis_len) {
                            let diff = sy - screen_center;
                            for step in 1..=12 {
                                let t = step as f32 / 12.0;
                                let p = screen_center + diff * t;
                                instances.push(QuadInstance::new(
                                    [p.x - 1.0, p.y - 1.0],
                                    [2.5, 2.5],
                                    green,
                                    0,
                                ));
                            }
                            if tool == ViewportTool::Scale {
                                instances.push(QuadInstance::new(
                                    [sy.x - 4.5, sy.y - 4.5],
                                    [9.0, 9.0],
                                    green,
                                    0,
                                ));
                            } else {
                                instances.push(QuadInstance::new(
                                    [sy.x - 5.0, sy.y - 5.0],
                                    [10.0, 10.0],
                                    green,
                                    0,
                                ));
                            }
                            font::draw_text(instances, "Y", sy.x + 6.0, sy.y - 5.0, 1.1, green);
                        }

                        // Z Ekseni (Mavi)
                        if let Some(sz) = project_pt(center + glam::Vec3::Z * axis_len) {
                            let diff = sz - screen_center;
                            for step in 1..=12 {
                                let t = step as f32 / 12.0;
                                let p = screen_center + diff * t;
                                instances.push(QuadInstance::new(
                                    [p.x - 1.0, p.y - 1.0],
                                    [2.5, 2.5],
                                    blue,
                                    0,
                                ));
                            }
                            if tool == ViewportTool::Scale {
                                instances.push(QuadInstance::new(
                                    [sz.x - 4.5, sz.y - 4.5],
                                    [9.0, 9.0],
                                    blue,
                                    0,
                                ));
                            } else {
                                instances.push(QuadInstance::new(
                                    [sz.x - 5.0, sz.y - 5.0],
                                    [10.0, 10.0],
                                    blue,
                                    0,
                                ));
                            }
                            font::draw_text(instances, "Z", sz.x + 6.0, sz.y - 5.0, 1.1, blue);
                        }
                    }
                }
            }
        }
    }
}
