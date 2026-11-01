use crate::theme::*;
use crate::types::*;
use crate::App;
use glam::{Mat4, Vec2};
use rugeng_core::{
    interpolate_particle_color, Animation2DComponent, Collider2D, Entity, Mesh3DComponent,
    NodeName, Parent, ParticleEmitterComponent, ScriptComponent, SpriteColor, SpriteTexture,
    Text2DComponent, TileType, Tilemap2DComponent, Transform2D, Transform3D, Velocity2D, ZIndex2D,
};
use rugeng_render::font;
use rugeng_render::pipeline_2d::QuadInstance;
use std::time::Instant;
impl App {
    pub(crate) fn render_ui_and_scene(&mut self, screen_w: f32, screen_h: f32) {
        let render_start = Instant::now();
        if self.context.is_none() || self.pipeline_2d.is_none() {
            return;
        }

        self.arena.reset();
        let mut instances = Vec::with_capacity(8192);

        // =========================================================================
        // GODOT 4 DOCK VE YERLEŞİM ÖLÇÜLERİ
        // =========================================================================
        let selected_e = self.editor.selected_entity;
        let menu_h = 30.0;
        let left_w = 260.0;
        let right_w = 300.0;
        let bottom_h = 160.0;

        let left_top_h = (screen_h - menu_h) * 0.52;
        let left_bot_y = menu_h + left_top_h;
        let left_bot_h = (screen_h - menu_h) - left_top_h;

        let vp_x = left_w;
        let vp_w = screen_w - left_w - right_w;
        let scene_tab_h = 26.0;
        let vp_tool_h = 24.0;
        let vp_canvas_y = menu_h + scene_tab_h + vp_tool_h;
        let vp_canvas_h = screen_h - bottom_h - vp_canvas_y;

        // Modern Koyu Ton Renk Teması Sabitleri
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
            &mut instances,
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
            &mut instances,
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
            &mut instances,
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
                ViewportTool::Select => {
                    draw_icon_select(&mut instances, t_x + 4.0, icon_y, icon_col)
                }
                ViewportTool::Move => draw_icon_move(&mut instances, t_x + 4.0, icon_y, icon_col),
                ViewportTool::Rotate => {
                    draw_icon_rotate(&mut instances, t_x + 4.0, icon_y, icon_col)
                }
                ViewportTool::Scale => draw_icon_scale(&mut instances, t_x + 4.0, icon_y, icon_col),
                ViewportTool::TileBrush => {
                    draw_icon_brush(&mut instances, t_x + 4.0, icon_y, icon_col)
                }
                ViewportTool::TileEraser => {
                    draw_icon_eraser(&mut instances, t_x + 4.0, icon_y, icon_col)
                }
            }

            let txt_scale = if label.len() > 6 { 0.9 } else { 0.95 };
            font::draw_text(
                &mut instances,
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
                    &mut instances,
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
            &mut instances,
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
                            &mut instances,
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
                        &mut instances,
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
                    font::draw_text(
                        &mut instances,
                        num,
                        vp_x + 2.0,
                        r_screen_y - 6.0,
                        1.1,
                        text_dim,
                    );
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
                &mut instances,
                "[-]",
                zoom_box_x + 6.0,
                zoom_box_y + 5.0,
                1.3,
                text_dim,
            );
            font::draw_text(
                &mut instances,
                &format!("{:.1} %", cam.zoom * 100.0),
                zoom_box_x + 32.0,
                zoom_box_y + 5.0,
                1.3,
                text_norm,
            );
            font::draw_text(
                &mut instances,
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
                &mut instances,
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
                            &mut instances,
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
                            font::draw_text(&mut instances, "X", sx.x + 6.0, sx.y - 5.0, 1.1, red);
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
                            font::draw_text(
                                &mut instances,
                                "Y",
                                sy.x + 6.0,
                                sy.y - 5.0,
                                1.1,
                                green,
                            );
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
                            font::draw_text(&mut instances, "Z", sz.x + 6.0, sz.y - 5.0, 1.1, blue);
                        }
                    }
                }
            }
        }

        // =========================================================================
        // 2. SOL SÜTUN: ÜST PANEL — SAHNE HIYERARŞISI (SCENE TREE & CREATE ROOT)
        // =========================================================================
        instances.push(QuadInstance::new(
            [0.0, menu_h],
            [left_w, left_top_h],
            bg_dock,
            0,
        ));
        instances.push(QuadInstance::new(
            [left_w - 1.0, menu_h],
            [1.0, left_top_h],
            border_col,
            0,
        ));

        // Sol Üst Sekmeler: [Scene] | [Import]
        let tab_scene_act = self.editor.left_tab == LeftTab::Scene;
        let s_tab_bg = if tab_scene_act {
            bg_header_act
        } else {
            bg_header_inact
        };
        instances.push(QuadInstance::new([0.0, menu_h], [80.0, 24.0], s_tab_bg, 0));
        if tab_scene_act {
            instances.push(QuadInstance::new([0.0, menu_h], [80.0, 1.5], godot_blue, 0));
        }
        font::draw_text(
            &mut instances,
            "Scene",
            16.0,
            menu_h + 6.0,
            1.4,
            if tab_scene_act { text_norm } else { text_dim },
        );

        let i_tab_bg = if !tab_scene_act {
            bg_header_act
        } else {
            bg_header_inact
        };
        instances.push(QuadInstance::new([82.0, menu_h], [80.0, 24.0], i_tab_bg, 0));
        if !tab_scene_act {
            instances.push(QuadInstance::new(
                [82.0, menu_h],
                [80.0, 1.5],
                godot_blue,
                0,
            ));
        }
        font::draw_text(
            &mut instances,
            "Import",
            98.0,
            menu_h + 6.0,
            1.4,
            if !tab_scene_act { text_norm } else { text_dim },
        );

        // Düğüm Arama ve Hızlı Butonlar
        let filter_y = menu_h + 26.0;
        instances.push(QuadInstance::new(
            [6.0, filter_y],
            [24.0, 20.0],
            theme.btn_normal,
            0,
        ));
        instances.push(QuadInstance::new(
            [6.0, filter_y],
            [24.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(&mut instances, "+", 14.0, filter_y + 4.0, 1.5, godot_green);

        instances.push(QuadInstance::new(
            [34.0, filter_y],
            [24.0, 20.0],
            theme.btn_normal,
            0,
        ));
        instances.push(QuadInstance::new(
            [34.0, filter_y],
            [24.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(&mut instances, "@", 42.0, filter_y + 4.0, 1.4, godot_blue);

        instances.push(QuadInstance::new(
            [62.0, filter_y],
            [left_w - 68.0, 20.0],
            theme.input_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [62.0, filter_y],
            [left_w - 68.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            &mut instances,
            "Filter Nodes",
            70.0,
            filter_y + 4.0,
            1.3,
            text_dim,
        );

        // Düğüm Ekleme Preset Butonları (2D vs 3D Bağlamsal Düğmeler)
        let root_y = filter_y + 26.0;
        let is_3d_ws = self.editor.workspace == WorkspaceMode::ThreeD;
        font::draw_text(
            &mut instances,
            if is_3d_ws {
                "3D Mesh / Model Ekle (+):"
            } else {
                "2D Varlik / Dugum Ekle (+):"
            },
            10.0,
            root_y,
            1.3,
            if is_3d_ws { godot_yellow } else { godot_blue },
        );

        let b1_y = root_y + 16.0;
        let b2_y = b1_y + 24.0;
        let b3_y = b2_y + 24.0;
        let b4_y = b3_y + 24.0;
        let b5_y = b4_y + 24.0;

        if is_3d_ws {
            // Satır 1: [+ Kup] ve [+ Kure]
            instances.push(QuadInstance::new(
                [8.0, b1_y],
                [118.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b1_y],
                [118.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Kup 3D",
                28.0,
                b1_y + 4.0,
                1.2,
                godot_yellow,
            );

            instances.push(QuadInstance::new(
                [132.0, b1_y],
                [120.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [132.0, b1_y],
                [120.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Kure 3D",
                146.0,
                b1_y + 4.0,
                1.2,
                godot_yellow,
            );

            // Satır 2: [+ Silindir] ve [+ Koni]
            instances.push(QuadInstance::new(
                [8.0, b2_y],
                [118.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b2_y],
                [118.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Silindir",
                22.0,
                b2_y + 4.0,
                1.2,
                [52, 211, 153, 255],
            );

            instances.push(QuadInstance::new(
                [132.0, b2_y],
                [120.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [132.0, b2_y],
                [120.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Koni 3D",
                146.0,
                b2_y + 4.0,
                1.2,
                [249, 115, 22, 255],
            );

            // Satır 3: [+ Piramit] ve [+ Kapsul]
            instances.push(QuadInstance::new(
                [8.0, b3_y],
                [118.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b3_y],
                [118.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Piramit",
                22.0,
                b3_y + 4.0,
                1.2,
                [234, 179, 8, 255],
            );

            instances.push(QuadInstance::new(
                [132.0, b3_y],
                [120.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [132.0, b3_y],
                [120.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Kapsul",
                150.0,
                b3_y + 4.0,
                1.2,
                [56, 189, 248, 255],
            );

            // Satır 4: [+ Duzlem / Plane] ve [+ Sprite3D]
            instances.push(QuadInstance::new(
                [8.0, b4_y],
                [118.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b4_y],
                [118.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(&mut instances, "+ Duzlem", 24.0, b4_y + 4.0, 1.2, text_norm);

            instances.push(QuadInstance::new(
                [132.0, b4_y],
                [120.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [132.0, b4_y],
                [120.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Sprite3D",
                144.0,
                b4_y + 4.0,
                1.2,
                [192, 132, 252, 255],
            );

            // Satır 5: [+ Isik (Sun)]
            instances.push(QuadInstance::new(
                [8.0, b5_y],
                [left_w - 16.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b5_y],
                [left_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Directional Light 3D",
                42.0,
                b5_y + 4.0,
                1.2,
                godot_yellow,
            );
        } else {
            // Satır 1: [+ Gemi] ve [+ Bocek]
            instances.push(QuadInstance::new(
                [8.0, b1_y],
                [118.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b1_y],
                [118.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(&mut instances, "+ Gemi", 28.0, b1_y + 4.0, 1.2, godot_blue);

            instances.push(QuadInstance::new(
                [132.0, b1_y],
                [120.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [132.0, b1_y],
                [120.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(&mut instances, "+ Bocek", 152.0, b1_y + 4.0, 1.2, godot_red);

            // Satır 2: [+ Kure] ve [+ Kalp]
            instances.push(QuadInstance::new(
                [8.0, b2_y],
                [118.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b2_y],
                [118.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Kure",
                28.0,
                b2_y + 4.0,
                1.2,
                godot_yellow,
            );

            instances.push(QuadInstance::new(
                [132.0, b2_y],
                [120.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [132.0, b2_y],
                [120.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Kalp",
                156.0,
                b2_y + 4.0,
                1.2,
                [244, 114, 182, 255],
            );

            // Satır 3: [+ Bos 2D]
            instances.push(QuadInstance::new(
                [8.0, b3_y],
                [left_w - 16.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b3_y],
                [left_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Bos Dugum (Node2D)",
                52.0,
                b3_y + 4.0,
                1.2,
                godot_green,
            );

            // Satır 4: [+ Anim Gemi] ve [+ Harita]
            instances.push(QuadInstance::new(
                [8.0, b4_y],
                [118.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b4_y],
                [118.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Anim Gemi",
                20.0,
                b4_y + 4.0,
                1.2,
                theme.purple,
            );

            instances.push(QuadInstance::new(
                [132.0, b4_y],
                [120.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [132.0, b4_y],
                [120.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Harita",
                152.0,
                b4_y + 4.0,
                1.2,
                [52, 211, 153, 255],
            );

            // Satır 5: [+ Metin 2D]
            instances.push(QuadInstance::new(
                [8.0, b5_y],
                [left_w - 16.0, 20.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, b5_y],
                [left_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Metin 2D (Label)",
                60.0,
                b5_y + 4.0,
                1.2,
                text_norm,
            );
        }

        // Seçili Düğüm Eylemleri: [KOPYALA] [SIL] [YENI SAHNE]
        let act_bar_y = b5_y + 24.0;
        instances.push(QuadInstance::new(
            [8.0, act_bar_y],
            [74.0, 20.0],
            theme.btn_normal,
            0,
        ));
        instances.push(QuadInstance::new(
            [8.0, act_bar_y],
            [74.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            &mut instances,
            "Kopyala",
            16.0,
            act_bar_y + 4.0,
            1.1,
            text_norm,
        );

        instances.push(QuadInstance::new(
            [86.0, act_bar_y],
            [74.0, 20.0],
            [54, 28, 30, 255],
            0,
        ));
        instances.push(QuadInstance::new(
            [86.0, act_bar_y],
            [74.0, 1.0],
            godot_red,
            0,
        ));
        font::draw_text(
            &mut instances,
            "Sil (Del)",
            94.0,
            act_bar_y + 4.0,
            1.1,
            godot_red,
        );

        instances.push(QuadInstance::new(
            [164.0, act_bar_y],
            [88.0, 20.0],
            theme.btn_normal,
            0,
        ));
        instances.push(QuadInstance::new(
            [164.0, act_bar_y],
            [88.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            &mut instances,
            "Yeni Sahne",
            170.0,
            act_bar_y + 4.0,
            1.1,
            text_dim,
        );

        // Sahnedeki Varlık Listesi
        // Sahnedeki Varlık Listesi
        let mut row_y = act_bar_y + 26.0;
        let mut entity_list = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.registry.view1_ref::<Transform2D, _>(|e, _| {
            if seen.insert(e) {
                entity_list.push(e);
            }
        });
        self.registry.view1_ref::<Transform3D, _>(|e, _| {
            if seen.insert(e) {
                entity_list.push(e);
            }
        });

        for e in entity_list {
            if row_y + 20.0 > left_bot_y - 8.0 {
                break;
            }
            let is_sel = Some(e) == selected_e;
            let bg_color = if is_sel {
                theme.accent_subtle
            } else {
                theme.card_bg
            };
            let has_parent = self.registry.has::<Parent>(e);
            let indent = if has_parent { 14.0 } else { 0.0 };
            let item_w = left_w - 16.0 - indent;

            instances.push(QuadInstance::new(
                [8.0 + indent, row_y],
                [item_w, 20.0],
                bg_color,
                0,
            ));
            if is_sel {
                instances.push(QuadInstance::new(
                    [8.0 + indent, row_y],
                    [2.0, 20.0],
                    godot_blue,
                    0,
                ));
            }

            let is_3d =
                self.registry.has::<Transform3D>(e) || self.registry.has::<Mesh3DComponent>(e);
            let icon_col = if is_3d {
                [224, 169, 66, 255] // Altın sarısı 3D ikonu
            } else if self.registry.has::<Text2DComponent>(e) {
                theme.purple
            } else {
                godot_blue
            };
            instances.push(QuadInstance::new(
                [14.0 + indent, row_y + 5.0],
                [8.0, 8.0],
                icon_col,
                0,
            ));

            let node_prefix = if has_parent { " > " } else { "" };
            let type_prefix = if is_3d {
                if let Some(m) = self.registry.get::<Mesh3DComponent>(e) {
                    format!("Mesh3D_{}", m.mesh_name)
                } else {
                    "Node3D".to_string()
                }
            } else if self.registry.has::<Text2DComponent>(e) {
                "Text2D".to_string()
            } else {
                "Node2D".to_string()
            };

            let editing_here = self
                .editor
                .renaming
                .as_ref()
                .map(|r| r.entity == e)
                .unwrap_or(false);
            let label = if editing_here {
                let r = self.editor.renaming.as_ref().unwrap();
                let cs: Vec<char> = r.buffer.chars().collect();
                let pos = r.caret.min(cs.len());
                let head: String = cs[..pos].iter().collect();
                let tail: String = cs[pos..].iter().collect();
                format!("{}▏{}", head, tail)
            } else if let Some(n) = self.registry.get::<NodeName>(e) {
                format!("{}_{}", node_prefix, n.name)
            } else {
                format!(
                    "{}_{}#{}:{}",
                    node_prefix,
                    type_prefix,
                    e.index(),
                    e.generation()
                )
            };

            font::draw_text(
                &mut instances,
                &label,
                28.0 + indent,
                row_y + 4.0,
                1.2,
                if editing_here {
                    godot_blue
                } else if is_sel {
                    text_norm
                } else {
                    text_dim
                },
            );

            // Sağdaki rozetler
            let badge = if is_3d {
                "[3D]"
            } else if self.registry.has::<Text2DComponent>(e) {
                "[TXT]"
            } else {
                "[2D]"
            };
            font::draw_text(
                &mut instances,
                badge,
                left_w - 38.0,
                row_y + 4.0,
                1.1,
                icon_col,
            );
            row_y += 24.0;
        }

        // Bölme Çizgisi (Splitter)
        instances.push(QuadInstance::new(
            [0.0, left_bot_y],
            [left_w, 2.0],
            border_col,
            0,
        ));

        // =========================================================================
        // 3. SOL SÜTUN: ALT PANEL — DOSYA SISTEMI (FILESYSTEM & RES:// DOCK)
        // =========================================================================
        instances.push(QuadInstance::new(
            [0.0, left_bot_y + 2.0],
            [left_w, left_bot_h - 2.0],
            bg_dock,
            0,
        ));
        instances.push(QuadInstance::new(
            [left_w - 1.0, left_bot_y + 2.0],
            [1.0, left_bot_h - 2.0],
            border_col,
            0,
        ));

        // Başlık Sekmesi: [FileSystem]
        instances.push(QuadInstance::new(
            [0.0, left_bot_y + 2.0],
            [left_w, 24.0],
            bg_header_act,
            0,
        ));
        instances.push(QuadInstance::new(
            [0.0, left_bot_y + 2.0],
            [left_w, 1.5],
            godot_blue,
            0,
        ));
        font::draw_text(
            &mut instances,
            "FileSystem",
            12.0,
            left_bot_y + 8.0,
            1.4,
            text_norm,
        );

        // Yol ve Arama Çubukları
        let fs_bar_y = left_bot_y + 28.0;
        instances.push(QuadInstance::new(
            [6.0, fs_bar_y],
            [left_w - 12.0, 20.0],
            theme.input_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [6.0, fs_bar_y],
            [left_w - 12.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            &mut instances,
            "<  res://",
            10.0,
            fs_bar_y + 4.0,
            1.3,
            text_dim,
        );
        let path_str = self.editor.current_path.to_string_lossy();
        let display_path = if path_str == "." {
            "root".to_string()
        } else {
            path_str.to_string()
        };
        font::draw_text(
            &mut instances,
            &display_path,
            80.0,
            fs_bar_y + 4.0,
            1.3,
            text_norm,
        );

        let fs_filter_y = fs_bar_y + 24.0;
        instances.push(QuadInstance::new(
            [6.0, fs_filter_y],
            [left_w - 12.0, 20.0],
            theme.input_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [6.0, fs_filter_y],
            [left_w - 12.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            &mut instances,
            "Filter Files",
            10.0,
            fs_filter_y + 4.0,
            1.3,
            text_dim,
        );

        // Dosya Ağacı Görünümü
        let tree_y = fs_filter_y + 26.0;
        let mut f_y = tree_y + 4.0;

        // Üst dizine çıkma butonu
        if self.editor.current_path.as_os_str() != "." {
            instances.push(QuadInstance::new(
                [8.0, f_y - 2.0],
                [left_w - 16.0, 18.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [8.0, f_y - 2.0],
                [left_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(&mut instances, "[..]", 14.0, f_y, 1.2, godot_yellow);
            font::draw_text(&mut instances, ".. (Ust Dizin)", 46.0, f_y, 1.3, text_norm);
            f_y += 20.0;
        }

        for item in &self.editor.files_in_dir {
            if f_y + 18.0 > screen_h - 4.0 {
                break;
            }
            let is_f_sel = self.editor.selected_file == item.name;
            let f_bg = if is_f_sel {
                theme.tab_hover
            } else {
                [0, 0, 0, 0]
            };
            if is_f_sel {
                instances.push(QuadInstance::new(
                    [8.0, f_y - 2.0],
                    [left_w - 16.0, 18.0],
                    f_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [8.0, f_y - 2.0],
                    [2.0, 18.0],
                    godot_blue,
                    0,
                ));
            }

            let (ftag, fcol) = if item.is_dir {
                ("[DIR]", godot_yellow)
            } else if item.name.ends_with(".rs") {
                ("[RS]", [249, 115, 22, 255])
            } else if item.name.ends_with(".toml") {
                ("[CFG]", godot_green)
            } else if item.name.ends_with(".md") {
                ("[DOC]", godot_blue)
            } else if item.name.ends_with(".spv") {
                ("[SPV]", theme.purple)
            } else {
                ("[FILE]", text_dim)
            };

            font::draw_text(&mut instances, ftag, 14.0, f_y, 1.1, fcol);
            let d_name = if item.name.len() > 20 {
                &item.name[..20]
            } else {
                &item.name
            };
            font::draw_text(
                &mut instances,
                d_name,
                54.0,
                f_y,
                1.3,
                if is_f_sel { text_norm } else { text_dim },
            );
            f_y += 20.0;
        }

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

            font::draw_text(
                &mut instances,
                &metric_1,
                vp_x + 10.0,
                dbg_y,
                1.3,
                theme.accent,
            );
            dbg_y += 18.0;
            font::draw_text(
                &mut instances,
                &metric_2,
                vp_x + 10.0,
                dbg_y,
                1.3,
                godot_green,
            );
            dbg_y += 18.0;
            font::draw_text(
                &mut instances,
                &metric_3,
                vp_x + 10.0,
                dbg_y,
                1.3,
                godot_yellow,
            );
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
                font::draw_text(
                    &mut instances,
                    &msg.text,
                    vp_x + 10.0,
                    log_y,
                    1.3,
                    lvl_color,
                );
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
            font::draw_text(
                &mut instances,
                lbl,
                b_x + 8.0,
                btm_tab_y + 6.0,
                1.3,
                tab_txt,
            );
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
            &mut instances,
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
            &mut instances,
            &format!("X {}", err_count),
            status_x + 46.0,
            btm_tab_y + 6.0,
            1.3,
            godot_red,
        );

        // Motor Güncel Versiyon Rozeti (Beta)
        font::draw_text(
            &mut instances,
            "RugEng v0.1.0-beta",
            status_x + 86.0,
            btm_tab_y + 6.0,
            1.25,
            theme.accent,
        );

        // =========================================================================
        // 5. SAĞ SÜTUN: TAM BOY DENETÇİ (INSPECTOR DOCK)
        // =========================================================================
        let insp_x = screen_w - right_w;
        let insp_h = screen_h - menu_h;
        instances.push(QuadInstance::new(
            [insp_x, menu_h],
            [right_w, insp_h],
            bg_dock,
            0,
        ));
        instances.push(QuadInstance::new(
            [insp_x, menu_h],
            [1.0, insp_h],
            border_col,
            0,
        ));

        // Denetçi Başlık Sekmesi
        instances.push(QuadInstance::new(
            [insp_x, menu_h],
            [right_w, 24.0],
            bg_header_act,
            0,
        ));
        instances.push(QuadInstance::new(
            [insp_x, menu_h],
            [right_w, 1.5],
            godot_blue,
            0,
        ));
        font::draw_text(
            &mut instances,
            "Inspector",
            insp_x + 14.0,
            menu_h + 6.0,
            1.4,
            text_norm,
        );

        // Arama ve Araç Simgeleri
        let insp_search_y = menu_h + 28.0;
        instances.push(QuadInstance::new(
            [insp_x + 8.0, insp_search_y],
            [right_w - 16.0, 22.0],
            theme.input_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [insp_x + 8.0, insp_search_y],
            [right_w - 16.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            &mut instances,
            "Filter Properties",
            insp_x + 14.0,
            insp_search_y + 5.0,
            1.3,
            text_dim,
        );

        if let Some(e) = selected_e {
            // Düğüm Başlığı: player (CharacterBody2D)
            let card_y = insp_search_y + 28.0;
            instances.push(QuadInstance::new(
                [insp_x + 8.0, card_y],
                [right_w - 16.0, 24.0],
                theme.tab_active,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, card_y],
                [right_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 14.0, card_y + 7.0],
                [10.0, 10.0],
                godot_blue,
                0,
            ));
            let is_3d_ent =
                self.registry.has::<Transform3D>(e) || self.registry.has::<Mesh3DComponent>(e);
            let ent_name = match self.registry.get::<NodeName>(e) {
                Some(n) if !n.name.is_empty() => format!("{} [Active]", n.name),
                _ => {
                    if is_3d_ent {
                        format!("Node3D_#{}:{} [Active]", e.index(), e.generation())
                    } else {
                        format!("Node2D_#{}:{} [Active]", e.index(), e.generation())
                    }
                }
            };
            font::draw_text(
                &mut instances,
                &ent_name,
                insp_x + 30.0,
                card_y + 6.0,
                1.3,
                text_norm,
            );

            let mut cur_insp_y = card_y + 30.0;
            // BİLEŞEN: [ Transform 3D ] (3D Varlıklar İçin)
            if let Some(t3) = self.registry.get::<Transform3D>(e).copied() {
                let c3d_y = cur_insp_y;
                let c3d_h = 160.0;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, c3d_y],
                    [right_w - 16.0, c3d_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, c3d_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, c3d_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "v Transform 3D",
                    insp_x + 14.0,
                    c3d_y + 4.0,
                    1.3,
                    [224, 169, 66, 255],
                );

                // Position X/Y/Z
                font::draw_text(
                    &mut instances,
                    &format!(
                        "Pos X: {:.1}  Y: {:.1}  Z: {:.1}",
                        t3.translation.x, t3.translation.y, t3.translation.z
                    ),
                    insp_x + 14.0,
                    c3d_y + 26.0,
                    1.15,
                    text_norm,
                );
                // X - / +
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c3d_y + 24.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "X-",
                    insp_x + 164.0,
                    c3d_y + 26.0,
                    1.0,
                    godot_red,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 188.0, c3d_y + 24.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "X+",
                    insp_x + 192.0,
                    c3d_y + 26.0,
                    1.0,
                    godot_red,
                );

                // Y - / +
                instances.push(QuadInstance::new(
                    [insp_x + 216.0, c3d_y + 24.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Y-",
                    insp_x + 220.0,
                    c3d_y + 26.0,
                    1.0,
                    godot_green,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 244.0, c3d_y + 24.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Y+",
                    insp_x + 248.0,
                    c3d_y + 26.0,
                    1.0,
                    godot_green,
                );

                // Z - / +
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c3d_y + 44.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Z-",
                    insp_x + 164.0,
                    c3d_y + 46.0,
                    1.0,
                    godot_blue,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 188.0, c3d_y + 44.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Z+",
                    insp_x + 192.0,
                    c3d_y + 46.0,
                    1.0,
                    godot_blue,
                );

                // Rotation Euler
                let (pitch, yaw, roll) = t3.rotation.to_euler(glam::EulerRot::YXZ);
                font::draw_text(
                    &mut instances,
                    &format!(
                        "Rot: P:{:.0} Y:{:.0} R:{:.0}",
                        pitch.to_degrees(),
                        yaw.to_degrees(),
                        roll.to_degrees()
                    ),
                    insp_x + 14.0,
                    c3d_y + 68.0,
                    1.15,
                    text_norm,
                );
                // Pitch - / +
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "P-",
                    insp_x + 164.0,
                    c3d_y + 68.0,
                    1.0,
                    text_dim,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 188.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "P+",
                    insp_x + 192.0,
                    c3d_y + 68.0,
                    1.0,
                    text_dim,
                );
                // Yaw - / +
                instances.push(QuadInstance::new(
                    [insp_x + 216.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Y-",
                    insp_x + 220.0,
                    c3d_y + 68.0,
                    1.0,
                    text_dim,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 244.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Y+",
                    insp_x + 248.0,
                    c3d_y + 68.0,
                    1.0,
                    text_dim,
                );

                // Scale X/Y/Z
                font::draw_text(
                    &mut instances,
                    &format!(
                        "Scale: {:.1}x{:.1}x{:.1}",
                        t3.scale.x, t3.scale.y, t3.scale.z
                    ),
                    insp_x + 14.0,
                    c3d_y + 90.0,
                    1.15,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c3d_y + 88.0],
                    [32.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "S-",
                    insp_x + 168.0,
                    c3d_y + 90.0,
                    1.0,
                    text_dim,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c3d_y + 88.0],
                    [32.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "S+",
                    insp_x + 204.0,
                    c3d_y + 90.0,
                    1.0,
                    text_dim,
                );

                // Mesh 3D Bilgisi & PBR
                if let Some(m3) = self.registry.get::<Mesh3DComponent>(e) {
                    font::draw_text(
                        &mut instances,
                        &format!(
                            "Mesh: {} | PBR Rough:{:.1} Met:{:.1}",
                            m3.mesh_name, m3.roughness, m3.metallic
                        ),
                        insp_x + 14.0,
                        c3d_y + 112.0,
                        1.1,
                        [224, 169, 66, 255],
                    );
                    // Mesh butonları (Cube, Sphere, Plane)
                    let mesh_btns = [("Cube", "cube"), ("Sphere", "sphere"), ("Plane", "plane")];
                    let mut mb_x = insp_x + 14.0;
                    for (lbl, name) in mesh_btns {
                        let is_sel = m3.mesh_name == name;
                        let bg = if is_sel {
                            theme.accent_active
                        } else {
                            theme.btn_normal
                        };
                        instances.push(QuadInstance::new(
                            [mb_x, c3d_y + 130.0],
                            [48.0, 16.0],
                            bg,
                            0,
                        ));
                        font::draw_text(
                            &mut instances,
                            lbl,
                            mb_x + 6.0,
                            c3d_y + 132.0,
                            1.0,
                            text_norm,
                        );
                        mb_x += 52.0;
                    }
                }
                cur_insp_y += c3d_h + 8.0;
            }

            // BİLEŞEN 1: [ Transform 2D ]
            let has_tr3d = self.registry.has::<Transform3D>(e);
            let has_tr2d = self.registry.has::<Transform2D>(e);
            if has_tr2d || !has_tr3d {
                let c1_y = cur_insp_y;
                let c1_h = 136.0;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, c1_y],
                    [right_w - 16.0, c1_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, c1_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, c1_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "v Transform 2D",
                    insp_x + 14.0,
                    c1_y + 4.0,
                    1.3,
                    godot_green,
                );

                let (curr_pos, curr_size, curr_rot) =
                    if let Some(t) = self.registry.get::<Transform2D>(e) {
                        (t.pos, t.size, t.rotation)
                    } else {
                        (Vec2::ZERO, Vec2::ZERO, 0.0)
                    };

                let curr_z = self.registry.get::<ZIndex2D>(e).map(|zi| zi.z).unwrap_or(0);

                // X Satırı
                font::draw_text(
                    &mut instances,
                    "Position X:",
                    insp_x + 14.0,
                    c1_y + 26.0,
                    1.3,
                    text_dim,
                );
                font::draw_text(
                    &mut instances,
                    &format!("{:.1}", curr_pos.x),
                    insp_x + 90.0,
                    c1_y + 26.0,
                    1.3,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 24.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 24.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[-]",
                    insp_x + 168.0,
                    c1_y + 26.0,
                    1.2,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 24.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 24.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[+]",
                    insp_x + 204.0,
                    c1_y + 26.0,
                    1.2,
                    text_norm,
                );

                // Y Satırı
                font::draw_text(
                    &mut instances,
                    "Position Y:",
                    insp_x + 14.0,
                    c1_y + 46.0,
                    1.3,
                    text_dim,
                );
                font::draw_text(
                    &mut instances,
                    &format!("{:.1}", curr_pos.y),
                    insp_x + 90.0,
                    c1_y + 46.0,
                    1.3,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 44.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 44.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[-]",
                    insp_x + 168.0,
                    c1_y + 46.0,
                    1.2,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 44.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 44.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[+]",
                    insp_x + 204.0,
                    c1_y + 46.0,
                    1.2,
                    text_norm,
                );

                // Rotation Satırı
                let rot_deg = curr_rot.to_degrees();
                font::draw_text(
                    &mut instances,
                    "Rotation:",
                    insp_x + 14.0,
                    c1_y + 68.0,
                    1.3,
                    text_dim,
                );
                font::draw_text(
                    &mut instances,
                    &format!("{:.1} deg", rot_deg),
                    insp_x + 90.0,
                    c1_y + 68.0,
                    1.3,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 66.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 66.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "R-",
                    insp_x + 168.0,
                    c1_y + 68.0,
                    1.2,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 66.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 66.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "R+",
                    insp_x + 204.0,
                    c1_y + 68.0,
                    1.2,
                    text_norm,
                );

                // Scale Satırı
                font::draw_text(
                    &mut instances,
                    &format!("Scale: {:.0} x {:.0}", curr_size.x, curr_size.y),
                    insp_x + 14.0,
                    c1_y + 90.0,
                    1.3,
                    text_dim,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 88.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 88.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "W-",
                    insp_x + 168.0,
                    c1_y + 90.0,
                    1.2,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 88.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 88.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "W+",
                    insp_x + 204.0,
                    c1_y + 90.0,
                    1.2,
                    text_norm,
                );

                // Z-Index (Katman Derinliği) Satırı
                font::draw_text(
                    &mut instances,
                    "Z-Index:",
                    insp_x + 14.0,
                    c1_y + 112.0,
                    1.3,
                    text_dim,
                );
                font::draw_text(
                    &mut instances,
                    &format!("{}", curr_z),
                    insp_x + 90.0,
                    c1_y + 112.0,
                    1.3,
                    godot_blue,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 110.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c1_y + 110.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Z-",
                    insp_x + 168.0,
                    c1_y + 112.0,
                    1.2,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 110.0],
                    [32.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c1_y + 110.0],
                    [32.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Z+",
                    insp_x + 204.0,
                    c1_y + 112.0,
                    1.2,
                    text_norm,
                );

                cur_insp_y += c1_h + 8.0;
            }

            let c2_y = cur_insp_y;
            let c2_h = 84.0;
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c2_y],
                [right_w - 16.0, c2_h],
                theme.card_bg,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c2_y],
                [right_w - 16.0, 20.0],
                theme.card_header,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c2_y],
                [right_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "v Sprite 2D",
                insp_x + 14.0,
                c2_y + 4.0,
                1.3,
                godot_blue,
            );

            let cur_tex_name = self
                .registry
                .get::<SpriteTexture>(e)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| "none".to_string());
            font::draw_text(
                &mut instances,
                &format!("Texture: res://{}", cur_tex_name),
                insp_x + 14.0,
                c2_y + 24.0,
                1.2,
                text_norm,
            );

            // Doku Seçim Düğmeleri
            let tex_options = [
                ("ship", "player_ship"),
                ("bug", "enemy_bug"),
                ("orb", "energy_orb"),
                ("heart", "heart_icon"),
                ("none", "none"),
            ];
            let mut opt_x = insp_x + 14.0;
            for (label, key) in tex_options {
                let is_sel = (key == "none" && cur_tex_name == "none") || (key == cur_tex_name);
                let bg_col = if is_sel {
                    theme.accent_active
                } else {
                    theme.btn_normal
                };
                instances.push(QuadInstance::new(
                    [opt_x, c2_y + 40.0],
                    [38.0, 16.0],
                    bg_col,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [opt_x, c2_y + 40.0],
                    [38.0, 1.0],
                    if is_sel {
                        godot_blue
                    } else {
                        theme.border_subtle
                    },
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    label,
                    opt_x + 6.0,
                    c2_y + 42.0,
                    1.1,
                    text_norm,
                );
                opt_x += 42.0;
            }

            let spr_col = self
                .registry
                .get::<SpriteColor>(e)
                .map(|s| s.color)
                .unwrap_or([255, 255, 255, 255]);
            font::draw_text(
                &mut instances,
                "Color:",
                insp_x + 14.0,
                c2_y + 64.0,
                1.2,
                text_dim,
            );
            instances.push(QuadInstance::new(
                [insp_x + 60.0, c2_y + 62.0],
                [24.0, 14.0],
                spr_col,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 60.0, c2_y + 62.0],
                [24.0, 1.0],
                border_col,
                0,
            ));

            let swatches = [
                [71, 140, 191, 255],  // Blue
                [228, 91, 91, 255],   // Red
                [66, 185, 131, 255],  // Green
                [224, 169, 66, 255],  // Yellow
                [168, 85, 247, 255],  // Purple
                [240, 240, 240, 255], // White
            ];
            let mut sw_x = insp_x + 92.0;
            for col in swatches {
                instances.push(QuadInstance::new([sw_x, c2_y + 62.0], [18.0, 14.0], col, 0));
                instances.push(QuadInstance::new(
                    [sw_x, c2_y + 62.0],
                    [18.0, 1.0],
                    border_col,
                    0,
                ));
                sw_x += 22.0;
            }

            // BİLEŞEN 3: [ CollisionShape 2D ]
            let c3_y = c2_y + c2_h + 8.0;
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c3_y],
                [right_w - 16.0, 54.0],
                theme.card_bg,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c3_y],
                [right_w - 16.0, 20.0],
                theme.card_header,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c3_y],
                [right_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "v CollisionShape 2D",
                insp_x + 14.0,
                c3_y + 4.0,
                1.3,
                godot_yellow,
            );

            let col_opt = self.registry.get::<Collider2D>(e).copied();
            if let Some(col) = col_opt {
                font::draw_text(
                    &mut instances,
                    "Shape: RectangleShape2D",
                    insp_x + 14.0,
                    c3_y + 26.0,
                    1.3,
                    text_norm,
                );
                let mode_str = if col.is_sensor {
                    "Sensor (Tetikleyici)"
                } else {
                    "Kati (Carpisma)"
                };
                font::draw_text(
                    &mut instances,
                    &format!("Mod: {}", mode_str),
                    insp_x + 14.0,
                    c3_y + 40.0,
                    1.2,
                    if col.is_sensor {
                        godot_yellow
                    } else {
                        godot_green
                    },
                );
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c3_y + 24.0],
                    [86.0, 20.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c3_y + 24.0],
                    [86.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    if col.is_sensor {
                        "[Kati Yap]"
                    } else {
                        "[Sensor Yap]"
                    },
                    insp_x + 202.0,
                    c3_y + 28.0,
                    1.1,
                    [220, 230, 255, 255],
                );
            } else {
                font::draw_text(
                    &mut instances,
                    "(No CollisionShape assigned)",
                    insp_x + 14.0,
                    c3_y + 30.0,
                    1.2,
                    text_dim,
                );
            }

            // BİLEŞEN 4: [ Script ] (Aşama 5)
            let c4_y = c3_y + 60.0;
            let c4_h = 66.0;
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c4_y],
                [right_w - 16.0, c4_h],
                theme.card_bg,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c4_y],
                [right_w - 16.0, 20.0],
                theme.card_header,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, c4_y],
                [right_w - 16.0, 1.0],
                theme.border_subtle,
                0,
            ));
            font::draw_text(
                &mut instances,
                "v Script (GDScript / Rust)",
                insp_x + 14.0,
                c4_y + 4.0,
                1.3,
                theme.purple,
            );

            let cur_script = self
                .registry
                .get::<ScriptComponent>(e)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "none".to_string());
            font::draw_text(
                &mut instances,
                &format!("Attached: res://{}", cur_script),
                insp_x + 14.0,
                c4_y + 24.0,
                1.2,
                text_norm,
            );

            // Script Butonları: [Rotator] [Patrol] [Pulse] [none]
            let scr_opts = ["Rotator", "Patrol", "Pulse", "none"];
            let mut so_x = insp_x + 14.0;
            for opt in scr_opts {
                let is_sel = (opt == "none" && cur_script == "none")
                    || (opt == "Rotator" && cur_script == "RotatorScript")
                    || (opt == "Patrol" && cur_script == "PatrolScript")
                    || (opt == "Pulse" && cur_script == "PulseScript");
                let bg_col = if is_sel {
                    theme.accent_active
                } else {
                    theme.btn_normal
                };
                instances.push(QuadInstance::new(
                    [so_x, c4_y + 42.0],
                    [54.0, 16.0],
                    bg_col,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [so_x, c4_y + 42.0],
                    [54.0, 1.0],
                    if is_sel {
                        theme.purple
                    } else {
                        theme.border_subtle
                    },
                    0,
                ));
                font::draw_text(&mut instances, opt, so_x + 6.0, c4_y + 44.0, 1.1, text_norm);
                so_x += 58.0;
            }

            cur_insp_y = c4_y + c4_h + 8.0;

            // BİLEŞEN 5: Velocity 2D (varsa göster)
            let vel_opt = self.registry.get::<Velocity2D>(e).copied();
            if let Some(v) = vel_opt {
                let v_h = 52.0;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, v_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "v Velocity 2D (Fizik)",
                    insp_x + 14.0,
                    cur_insp_y + 4.0,
                    1.3,
                    godot_yellow,
                );

                font::draw_text(
                    &mut instances,
                    &format!("Vx: {:.1}", v.vel.x),
                    insp_x + 14.0,
                    cur_insp_y + 26.0,
                    1.2,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 88.0, cur_insp_y + 24.0],
                    [26.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 88.0, cur_insp_y + 24.0],
                    [26.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[-]",
                    insp_x + 94.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 118.0, cur_insp_y + 24.0],
                    [26.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 118.0, cur_insp_y + 24.0],
                    [26.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[+]",
                    insp_x + 124.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_norm,
                );

                font::draw_text(
                    &mut instances,
                    &format!("Vy: {:.1}", v.vel.y),
                    insp_x + 154.0,
                    cur_insp_y + 26.0,
                    1.2,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 228.0, cur_insp_y + 24.0],
                    [26.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 228.0, cur_insp_y + 24.0],
                    [26.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[-]",
                    insp_x + 234.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 258.0, cur_insp_y + 24.0],
                    [26.0, 16.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 258.0, cur_insp_y + 24.0],
                    [26.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[+]",
                    insp_x + 264.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_norm,
                );

                cur_insp_y += v_h + 8.0;
            }

            // BİLEŞEN 6: Animation 2D (varsa göster)
            let anim_opt = self.registry.get::<Animation2DComponent>(e).cloned();
            if let Some(anim) = anim_opt {
                let a_h = 56.0;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, a_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "v Animation 2D (SpriteSheet)",
                    insp_x + 14.0,
                    cur_insp_y + 4.0,
                    1.3,
                    theme.purple,
                );

                let clip_info = if let Some(clip) = anim.clips.get(&anim.current_clip) {
                    format!(
                        "{}: {}/{}",
                        anim.current_clip,
                        anim.current_frame_idx + 1,
                        clip.frames.len()
                    )
                } else {
                    "Yok".to_string()
                };
                font::draw_text(
                    &mut instances,
                    &clip_info,
                    insp_x + 14.0,
                    cur_insp_y + 26.0,
                    1.2,
                    text_norm,
                );

                // Play / Pause Butonu
                let play_lbl = if anim.is_playing {
                    "[Durdur]"
                } else {
                    "[Oynat]"
                };
                instances.push(QuadInstance::new(
                    [insp_x + 130.0, cur_insp_y + 24.0],
                    [56.0, 18.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 130.0, cur_insp_y + 24.0],
                    [56.0, 1.0],
                    godot_blue,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    play_lbl,
                    insp_x + 134.0,
                    cur_insp_y + 26.0,
                    1.1,
                    godot_blue,
                );

                // FPS Butonları
                let cur_fps = anim
                    .clips
                    .get(&anim.current_clip)
                    .map(|c| c.fps)
                    .unwrap_or(4.0);
                font::draw_text(
                    &mut instances,
                    &format!("{:.0} FPS", cur_fps),
                    insp_x + 192.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_dim,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 240.0, cur_insp_y + 24.0],
                    [22.0, 18.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 240.0, cur_insp_y + 24.0],
                    [22.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "-",
                    insp_x + 248.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_norm,
                );
                instances.push(QuadInstance::new(
                    [insp_x + 266.0, cur_insp_y + 24.0],
                    [22.0, 18.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 266.0, cur_insp_y + 24.0],
                    [22.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "+",
                    insp_x + 274.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_norm,
                );

                cur_insp_y += a_h + 8.0;
            }

            // BİLEŞEN 7: Tilemap 2D (Canli Karo Editoru)
            let tilemap_opt = self
                .registry
                .get::<Tilemap2DComponent>(e)
                .map(|tm| (tm.tile_size, tm.tiles.len()));
            if let Some((tsz, count)) = tilemap_opt {
                let tm_h = 104.0;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, tm_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "v Tilemap 2D (Canli Karo Editoru)",
                    insp_x + 14.0,
                    cur_insp_y + 4.0,
                    1.3,
                    [52, 211, 153, 255],
                );

                font::draw_text(
                    &mut instances,
                    &format!("{:.0}px | {} karo", tsz, count),
                    insp_x + 14.0,
                    cur_insp_y + 26.0,
                    1.2,
                    text_norm,
                );

                // Haritayı Temizle butonu
                instances.push(QuadInstance::new(
                    [insp_x + right_w - 96.0, cur_insp_y + 23.0],
                    [80.0, 18.0],
                    [54, 28, 30, 255],
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + right_w - 96.0, cur_insp_y + 23.0],
                    [80.0, 1.0],
                    godot_red,
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "Temizle",
                    insp_x + right_w - 76.0,
                    cur_insp_y + 26.0,
                    1.1,
                    godot_red,
                );

                // Mod Seçimi: [Fırça] ve [Silgi]
                let is_brush = self.editor.active_tool == ViewportTool::TileBrush;
                let is_eraser = self.editor.active_tool == ViewportTool::TileEraser;
                let brush_bg = if is_brush {
                    theme.accent_subtle
                } else {
                    theme.btn_normal
                };
                let eraser_bg = if is_eraser {
                    [54, 28, 30, 255]
                } else {
                    theme.btn_normal
                };
                instances.push(QuadInstance::new(
                    [insp_x + 14.0, cur_insp_y + 48.0],
                    [70.0, 20.0],
                    brush_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 14.0, cur_insp_y + 48.0],
                    [70.0, 1.0],
                    if is_brush {
                        godot_blue
                    } else {
                        theme.border_subtle
                    },
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[Firca]",
                    insp_x + 24.0,
                    cur_insp_y + 52.0,
                    1.1,
                    if is_brush { text_norm } else { text_dim },
                );

                instances.push(QuadInstance::new(
                    [insp_x + 90.0, cur_insp_y + 48.0],
                    [70.0, 20.0],
                    eraser_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 90.0, cur_insp_y + 48.0],
                    [70.0, 1.0],
                    if is_eraser {
                        godot_red
                    } else {
                        theme.border_subtle
                    },
                    0,
                ));
                font::draw_text(
                    &mut instances,
                    "[Silgi]",
                    insp_x + 102.0,
                    cur_insp_y + 52.0,
                    1.1,
                    if is_eraser { text_norm } else { text_dim },
                );

                // Karo Paleti Butonları (Çimen, Taş, Metal, Lav, Su)
                let tile_palette = [
                    ("Cimen", TileType::Grass),
                    ("Tas", TileType::Stone),
                    ("Metal", TileType::Metal),
                    ("Lav", TileType::Lava),
                    ("Su", TileType::Water),
                ];
                let mut pal_x = insp_x + 14.0;
                for (lbl, tt) in tile_palette {
                    let is_sel = is_brush && self.editor.active_tile_type == tt;
                    let c = tt.default_color();
                    let bg_col = if is_sel {
                        [c[0], c[1], c[2], 255]
                    } else {
                        theme.btn_normal
                    };
                    instances.push(QuadInstance::new(
                        [pal_x, cur_insp_y + 74.0],
                        [48.0, 22.0],
                        bg_col,
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [pal_x, cur_insp_y + 74.0],
                        [48.0, 2.0],
                        [c[0], c[1], c[2], 255],
                        0,
                    ));
                    if is_sel {
                        instances.push(QuadInstance::new(
                            [pal_x, cur_insp_y + 74.0],
                            [48.0, 1.0],
                            [255, 255, 255, 255],
                            0,
                        ));
                    }
                    let txt_col = if is_sel { [20, 24, 30, 255] } else { text_norm };
                    font::draw_text(
                        &mut instances,
                        lbl,
                        pal_x + 6.0,
                        cur_insp_y + 78.0,
                        1.1,
                        txt_col,
                    );
                    pal_x += 52.0;
                }

                cur_insp_y += tm_h + 8.0;
            }

            // [+ Add Component / Node] Butonu
            let add_btn_y = cur_insp_y;
            instances.push(QuadInstance::new(
                [insp_x + 8.0, add_btn_y],
                [right_w - 16.0, 24.0],
                theme.btn_normal,
                0,
            ));
            instances.push(QuadInstance::new(
                [insp_x + 8.0, add_btn_y],
                [right_w - 16.0, 1.0],
                godot_blue,
                0,
            ));
            font::draw_text(
                &mut instances,
                "+ Add Component",
                insp_x + 70.0,
                add_btn_y + 6.0,
                1.3,
                godot_blue,
            );

            // Açılır Bileşen Menüsü (Popup)
            if self.editor.show_add_component_popup {
                let pop_y = add_btn_y + 28.0;
                let pop_items = [
                    ("+ Collider2D Ekle", 0),
                    ("+ Velocity2D (Hiz) Ekle", 1),
                    ("+ Script (Rotator) Ekle", 2),
                    ("+ Text2D (Metin) Ekle", 3),
                    ("+ Animation2D (Animasyon) Ekle", 4),
                    ("+ Tilemap2D (Harita) Ekle", 5),
                ];
                let pop_h = (pop_items.len() as f32) * 22.0 + 8.0;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, pop_y],
                    [right_w - 16.0, pop_h],
                    theme.bg_panel_alt,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, pop_y],
                    [right_w - 16.0, 1.5],
                    godot_blue,
                    0,
                ));

                let mut it_y = pop_y + 4.0;
                for (item_lbl, _) in pop_items {
                    instances.push(QuadInstance::new(
                        [insp_x + 12.0, it_y],
                        [right_w - 24.0, 20.0],
                        theme.btn_normal,
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [insp_x + 12.0, it_y],
                        [right_w - 24.0, 1.0],
                        theme.border_subtle,
                        0,
                    ));
                    font::draw_text(
                        &mut instances,
                        item_lbl,
                        insp_x + 20.0,
                        it_y + 4.0,
                        1.2,
                        text_norm,
                    );
                    it_y += 22.0;
                }
            }
        } else {
            font::draw_text(
                &mut instances,
                "Select a node to inspect.",
                insp_x + 16.0,
                insp_search_y + 36.0,
                1.3,
                text_dim,
            );
        }

        // =========================================================================
        // 6. ÜST MENÜ VE ÇALIŞMA ALANI MODLARI (GODOT 4 TOP BAR)
        // =========================================================================
        instances.push(QuadInstance::new(
            [0.0, 0.0],
            [screen_w, menu_h],
            theme.bg_panel,
            0,
        ));
        instances.push(QuadInstance::new(
            [0.0, menu_h - 1.0],
            [screen_w, 1.0],
            border_col,
            0,
        ));

        // Sol Menü: Rugeng Simgesi ve Açılır Menü Başlıkları (Scene, Edit, Project, Debug, Help)
        instances.push(QuadInstance::new([8.0, 6.0], [18.0, 18.0], theme.accent, 0));
        if let Some(ttf) = self.ttf_font.as_ref() {
            ttf.draw_text(&mut instances, "R", 13.0, 7.0, 15.0, [255, 255, 255, 255]);
        } else {
            font::draw_text(&mut instances, "R", 13.0, 9.0, 1.4, [255, 255, 255, 255]);
        }

        let menu_headers = [
            ("Scene", OpenMenu::Scene),
            ("Edit", OpenMenu::Edit),
            ("Add", OpenMenu::Add),
            ("Project", OpenMenu::Project),
            ("Debug", OpenMenu::Debug),
            ("Help", OpenMenu::Help),
        ];

        let mut m_x = 34.0;
        for (name, menu_type) in menu_headers {
            let is_open = self.editor.active_menu == Some(menu_type);
            let tab_w = (name.len() as f32) * 8.0 + 16.0;
            if is_open {
                instances.push(QuadInstance::new(
                    [m_x, 4.0],
                    [tab_w, 22.0],
                    theme.tab_active,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [m_x, 24.0],
                    [tab_w, 2.0],
                    theme.accent,
                    0,
                ));
            }
            if let Some(ttf) = self.ttf_font.as_ref() {
                ttf.draw_text(
                    &mut instances,
                    name,
                    m_x + 8.0,
                    7.0,
                    14.0,
                    if is_open { theme.accent } else { text_norm },
                );
            } else {
                font::draw_text(
                    &mut instances,
                    name,
                    m_x + 8.0,
                    8.0,
                    1.35,
                    if is_open { theme.accent } else { text_norm },
                );
            }
            m_x += tab_w + 4.0;
        }

        // Orta: Çalışma Alanı Seçicileri (2D | 3D | Script | Game | Asset Store)
        let modes = [
            ("2D", WorkspaceMode::TwoD),
            ("3D", WorkspaceMode::ThreeD),
            ("Script", WorkspaceMode::Script),
            ("Game", WorkspaceMode::Game),
            ("Asset Store", WorkspaceMode::AssetStore),
        ];

        let mut ws_x = screen_w * 0.5 - 180.0;
        for (label, mode) in modes {
            let is_cur = self.editor.workspace == mode;
            let tab_w = (label.len() as f32) * 8.0 + 20.0;
            if is_cur {
                instances.push(QuadInstance::new(
                    [ws_x, 4.0],
                    [tab_w, 22.0],
                    theme.tab_active,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [ws_x, 24.0],
                    [tab_w, 2.0],
                    theme.accent,
                    0,
                ));
            }
            if let Some(ttf) = self.ttf_font.as_ref() {
                ttf.draw_text(
                    &mut instances,
                    label,
                    ws_x + 8.0,
                    7.0,
                    14.0,
                    if is_cur { theme.accent } else { text_dim },
                );
            } else {
                font::draw_text(
                    &mut instances,
                    label,
                    ws_x + 8.0,
                    8.0,
                    1.4,
                    if is_cur { theme.accent } else { text_dim },
                );
            }
            ws_x += tab_w + 6.0;
        }

        // Sağ: Hızlı Aksiyon Butonları ([KAYDET], [SARSINTI]) ve Oynatma Butonları ([▶], [❚❚], [■], [⏭])
        let play_x = screen_w - 140.0;
        let is_playing = self.editor.is_playing;
        let has_snapshot = !self.editor.scene_snapshot.is_empty();

        // [KAYDET] Hızlı Buton
        let save_btn_x = play_x - 148.0;
        instances.push(QuadInstance::new(
            [save_btn_x, 4.0],
            [68.0, 22.0],
            [32, 54, 40, 255],
            0,
        ));
        instances.push(QuadInstance::new(
            [save_btn_x, 4.0],
            [68.0, 1.0],
            theme.success,
            0,
        ));
        if let Some(ttf) = self.ttf_font.as_ref() {
            ttf.draw_text(
                &mut instances,
                "KAYDET",
                save_btn_x + 10.0,
                7.5,
                12.0,
                theme.text_primary,
            );
        } else {
            font::draw_text(
                &mut instances,
                "KAYDET",
                save_btn_x + 12.0,
                8.0,
                1.2,
                theme.text_primary,
            );
        }

        // [SARSINTI] Hızlı Buton
        let shake_btn_x = play_x - 76.0;
        instances.push(QuadInstance::new(
            [shake_btn_x, 4.0],
            [72.0, 22.0],
            [62, 34, 34, 255],
            0,
        ));
        instances.push(QuadInstance::new(
            [shake_btn_x, 4.0],
            [72.0, 1.0],
            theme.danger,
            0,
        ));
        if let Some(ttf) = self.ttf_font.as_ref() {
            ttf.draw_text(
                &mut instances,
                "SARSINTI",
                shake_btn_x + 8.0,
                7.5,
                12.0,
                theme.text_primary,
            );
        } else {
            font::draw_text(
                &mut instances,
                "SARSINTI",
                shake_btn_x + 9.0,
                8.0,
                1.2,
                theme.text_primary,
            );
        }

        // 1. Play Butonu (▶ - Yeşil Üçgen İkon)
        let play_bg = if is_playing {
            [26, 52, 36, 255]
        } else {
            theme.btn_normal
        };
        instances.push(QuadInstance::new([play_x, 4.0], [26.0, 22.0], play_bg, 0));
        instances.push(QuadInstance::new(
            [play_x, 4.0],
            [26.0, 1.0],
            if is_playing {
                theme.success
            } else {
                border_col
            },
            0,
        ));
        draw_icon_play(
            &mut instances,
            play_x + 8.0,
            9.0,
            11.0,
            if is_playing {
                theme.success
            } else {
                [74, 222, 128, 200]
            },
        );

        // 2. Pause Butonu (⏸ - İki Dikey Çubuk İkon)
        let is_paused = !is_playing && has_snapshot;
        let pause_bg = if is_paused {
            [50, 42, 22, 255]
        } else {
            theme.btn_normal
        };
        instances.push(QuadInstance::new(
            [play_x + 28.0, 4.0],
            [26.0, 22.0],
            pause_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [play_x + 28.0, 4.0],
            [26.0, 1.0],
            if is_paused { theme.warning } else { border_col },
            0,
        ));
        draw_icon_pause(
            &mut instances,
            play_x + 36.0,
            9.0,
            11.0,
            if is_paused { theme.warning } else { text_dim },
        );

        // 3. Stop Butonu (⏹ - Kırmızı Kare İkon)
        let stop_bg = theme.btn_normal;
        instances.push(QuadInstance::new(
            [play_x + 56.0, 4.0],
            [26.0, 22.0],
            stop_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [play_x + 56.0, 4.0],
            [26.0, 1.0],
            if is_playing || has_snapshot {
                theme.danger
            } else {
                border_col
            },
            0,
        ));
        draw_icon_stop(
            &mut instances,
            play_x + 64.0,
            10.0,
            10.0,
            if is_playing || has_snapshot {
                theme.danger
            } else {
                [160, 80, 80, 180]
            },
        );

        // 4. Step Butonu (⏭ - Dikey Çizgi ve Üçgen İkon)
        let step_bg = theme.btn_normal;
        instances.push(QuadInstance::new(
            [play_x + 84.0, 4.0],
            [26.0, 22.0],
            step_bg,
            0,
        ));
        instances.push(QuadInstance::new(
            [play_x + 84.0, 4.0],
            [26.0, 1.0],
            border_col,
            0,
        ));
        draw_icon_step(&mut instances, play_x + 90.0, 9.0, 11.0, theme.accent);

        // Açık Açılır Menü (Dropdown Menu Rendering) - En üst katmanda çizilir
        if let Some(active_menu) = self.editor.active_menu {
            let (drop_x, items): (f32, Vec<(&str, &str, &str)>) = match active_menu {
                OpenMenu::Scene => (
                    34.0,
                    vec![
                        ("[+] Yeni Sahne", "Ctrl+T", "new"),
                        ("[S] Sahneyi Kaydet", "Ctrl+K", "save"),
                        ("[F] Farkli Kaydet...", "", "save_as"),
                        ("[O] Sahne Ac...", "Ctrl+L", "load"),
                        ("[-] Ayirici", "", ""),
                        ("[X] Cikis", "Esc", "exit"),
                    ],
                ),
                OpenMenu::Edit => (
                    90.0,
                    vec![
                        ("[<] Geri Al", "Ctrl+Z", "undo"),
                        ("[>] Yinele", "Ctrl+Y", "redo"),
                        ("[-] Ayirici", "", ""),
                        ("[D] Varligi Cogalt", "Ctrl+C", "duplicate"),
                        ("[X] Varligi Sil", "Del", "delete"),
                        ("[A] Tumunu Sec", "", "select_all"),
                    ],
                ),
                OpenMenu::Add => (
                    142.0,
                    vec![
                        ("[+] 2D Sprite (Gemi)", "", "add_ship"),
                        ("[+] 2D Dusman (Bug)", "", "add_enemy"),
                        ("[+] 2D Can / Kalp", "", "add_heart"),
                        ("[+] 2D Enerji Kuresi", "", "add_orb"),
                        ("[+] 2D Karo Haritasi (Tilemap)", "", "add_tilemap"),
                        ("[+] 2D Metin (Text2D)", "", "add_text"),
                        ("[-] Ayirici", "", ""),
                        ("[3] 3D Kup (Cube)", "", "add_cube_3d"),
                        ("[3] 3D Kure (Sphere)", "", "add_sphere_3d"),
                        ("[3] 3D Duzlem (Plane)", "", "add_plane_3d"),
                        ("[3] 3D Silindir (Cylinder)", "", "add_cylinder_3d"),
                        ("[3] 3D Koni (Cone)", "", "add_cone_3d"),
                        ("[3] 3D Piramit (Pyramid)", "", "add_pyramid_3d"),
                        ("[3] 3D Kapsul (Capsule)", "", "add_capsule_3d"),
                        ("[3] 3D Sprite (2.5D)", "", "add_sprite_3d"),
                    ],
                ),
                OpenMenu::Project => (
                    196.0,
                    vec![
                        ("[E] Bagimsiz Oyunu Disa Aktar", "", "export"),
                        ("[R] Dosya Agacini Yenile", "", "refresh_files"),
                        ("[S] Proje Ayarlari...", "", "project_settings"),
                    ],
                ),
                OpenMenu::Debug => (
                    266.0,
                    vec![
                        ("[!] Sarsinti & Patlama FX", "X", "shake"),
                        ("[#] Izgara Yapismasi (Snap)", "S", "toggle_snap"),
                        ("[C] Konsol Kayitlarini Temizle", "", "clear_logs"),
                    ],
                ),
                OpenMenu::Help => (
                    326.0,
                    vec![
                        ("[?] Klavye Kisayollari", "", "help_shortcuts"),
                        ("[D] Dokumantasyon (docs/)", "", "help_docs"),
                        ("[i] RugEng Motoru Hakkinda", "", "help_about"),
                    ],
                ),
            };

            let menu_w = 230.0;
            let item_h = 24.0;
            let menu_h_total = items.len() as f32 * item_h + 8.0;
            let drop_y = 30.0;

            // Gölge / Arka plan
            instances.push(QuadInstance::new(
                [drop_x + 2.0, drop_y + 2.0],
                [menu_w, menu_h_total],
                [10, 12, 16, 200],
                0,
            ));
            instances.push(QuadInstance::new(
                [drop_x, drop_y],
                [menu_w, menu_h_total],
                theme.bg_panel,
                0,
            ));
            // Çerçeve
            instances.push(QuadInstance::new(
                [drop_x, drop_y],
                [menu_w, 1.5],
                godot_blue,
                0,
            ));
            instances.push(QuadInstance::new(
                [drop_x, drop_y + menu_h_total - 1.0],
                [menu_w, 1.0],
                theme.border_subtle,
                0,
            ));
            instances.push(QuadInstance::new(
                [drop_x, drop_y],
                [1.0, menu_h_total],
                theme.border_subtle,
                0,
            ));
            instances.push(QuadInstance::new(
                [drop_x + menu_w - 1.0, drop_y],
                [1.0, menu_h_total],
                theme.border_subtle,
                0,
            ));

            let mouse = self.editor.mouse_pos;
            let mut it_y = drop_y + 4.0;
            for (label, shortcut, id) in items {
                if id.is_empty() {
                    // Ayırıcı çizgi
                    instances.push(QuadInstance::new(
                        [drop_x + 8.0, it_y + 11.0],
                        [menu_w - 16.0, 1.0],
                        theme.border_subtle,
                        0,
                    ));
                } else {
                    let is_hover = mouse.x >= drop_x
                        && mouse.x <= drop_x + menu_w
                        && mouse.y >= it_y
                        && mouse.y <= it_y + item_h;
                    if is_hover {
                        instances.push(QuadInstance::new(
                            [drop_x + 4.0, it_y],
                            [menu_w - 8.0, item_h],
                            theme.tab_hover,
                            0,
                        ));
                    }
                    font::draw_text(
                        &mut instances,
                        label,
                        drop_x + 10.0,
                        it_y + 5.0,
                        1.25,
                        if is_hover {
                            [255, 255, 255, 255]
                        } else {
                            text_norm
                        },
                    );
                    if !shortcut.is_empty() {
                        let sc_w = (shortcut.len() as f32) * 7.5;
                        font::draw_text(
                            &mut instances,
                            shortcut,
                            drop_x + menu_w - sc_w - 10.0,
                            it_y + 5.0,
                            1.15,
                            text_dim,
                        );
                    }
                }
                it_y += item_h;
            }
        }

        // Sağ Tık [+] Context Menu (Bağlam Menüsü Rendering) - En üst katmanda çizilir
        if let Some(ctx_menu) = self.editor.context_menu.as_ref() {
            let menu_w = 230.0;
            let item_h = 24.0;
            let menu_h_total = ctx_menu.items.len() as f32 * item_h + 8.0;
            let mut pos_x = ctx_menu.screen_pos.x;
            let mut pos_y = ctx_menu.screen_pos.y;

            // Ekran sınırlarına taşmayı önle
            if pos_x + menu_w > screen_w - 10.0 {
                pos_x = (screen_w - menu_w - 10.0).max(10.0);
            }
            if pos_y + menu_h_total > screen_h - 10.0 {
                pos_y = (screen_h - menu_h_total - 10.0).max(10.0);
            }

            // Gölge
            instances.push(QuadInstance::new(
                [pos_x + 3.0, pos_y + 3.0],
                [menu_w, menu_h_total],
                [8, 10, 14, 220],
                0,
            ));
            // Arka plan paneli
            instances.push(QuadInstance::new(
                [pos_x, pos_y],
                [menu_w, menu_h_total],
                theme.bg_panel,
                0,
            ));
            // Kenarlıklar ve üst Godot mavisi çizgi
            instances.push(QuadInstance::new(
                [pos_x, pos_y],
                [menu_w, 2.0],
                godot_blue,
                0,
            ));
            instances.push(QuadInstance::new(
                [pos_x, pos_y + menu_h_total - 1.0],
                [menu_w, 1.0],
                theme.border_subtle,
                0,
            ));
            instances.push(QuadInstance::new(
                [pos_x, pos_y],
                [1.0, menu_h_total],
                theme.border_subtle,
                0,
            ));
            instances.push(QuadInstance::new(
                [pos_x + menu_w - 1.0, pos_y],
                [1.0, menu_h_total],
                theme.border_subtle,
                0,
            ));

            let mouse = self.editor.mouse_pos;
            let mut it_y = pos_y + 4.0;
            for item in &ctx_menu.items {
                if item.action_id.is_empty() {
                    // Ayırıcı çizgi
                    instances.push(QuadInstance::new(
                        [pos_x + 8.0, it_y + 11.0],
                        [menu_w - 16.0, 1.0],
                        theme.border_subtle,
                        0,
                    ));
                } else {
                    let is_hover = mouse.x >= pos_x
                        && mouse.x <= pos_x + menu_w
                        && mouse.y >= it_y
                        && mouse.y <= it_y + item_h;
                    if is_hover {
                        instances.push(QuadInstance::new(
                            [pos_x + 4.0, it_y],
                            [menu_w - 8.0, item_h],
                            theme.tab_hover,
                            0,
                        ));
                    }
                    font::draw_text(
                        &mut instances,
                        &item.label,
                        pos_x + 10.0,
                        it_y + 5.0,
                        1.25,
                        if is_hover {
                            [255, 255, 255, 255]
                        } else {
                            text_norm
                        },
                    );
                    if !item.shortcut.is_empty() {
                        let sc_w = (item.shortcut.len() as f32) * 7.5;
                        font::draw_text(
                            &mut instances,
                            &item.shortcut,
                            pos_x + menu_w - sc_w - 10.0,
                            it_y + 5.0,
                            1.15,
                            text_dim,
                        );
                    }
                }
                it_y += item_h;
            }
        }

        // =========================================================================
        // VULKAN 1.3 DYNAMIC RENDERING (TEK DRAW CALL)
        // =========================================================================
        let arena_instances = self.arena.alloc_slice_copy(&instances).unwrap();
        let Some(ctx) = self.context.as_mut() else {
            return;
        };
        let Some(pipe) = self.pipeline_2d.as_mut() else {
            return;
        };
        let frame_idx = ctx.current_frame;

        if let Ok(Some((image_index, cmd))) = ctx.begin_frame(screen_w as u32, screen_h as u32) {
            let count = unsafe { pipe.upload_instances(frame_idx, arena_instances) };
            let clear_r = theme.bg_canvas[0] as f32 / 255.0;
            let clear_g = theme.bg_canvas[1] as f32 / 255.0;
            let clear_b = theme.bg_canvas[2] as f32 / 255.0;

            let depth_view = self.depth_buffer.as_ref().map(|db| db.view);
            ctx.begin_rendering_with_depth(
                cmd,
                image_index,
                [clear_r, clear_g, clear_b, 1.0],
                depth_view,
            );

            // 1. 3D Sahne Çizimi (Eğer 3D Workspace Modundaysak)
            if self.editor.workspace == WorkspaceMode::ThreeD {
                if let Some(pipe_3d) = self.pipeline_3d.as_ref() {
                    let aspect = vp_w / vp_canvas_h.max(1.0);
                    let render_cam = self.editor.camera_3d.to_render_camera(aspect);

                    // A. Zemin Izgarası ve Koordinat Eksenleri
                    if let Some(grid_mesh) = self.gpu_grid.as_ref() {
                        pipe_3d.draw_mesh_pbr_viewport(
                            &ctx.device,
                            cmd,
                            &render_cam,
                            grid_mesh,
                            Mat4::IDENTITY,
                            &self.sun,
                            &self.ambient,
                            0.9,
                            0.0,
                            false,
                            vp_x,
                            vp_canvas_y,
                            vp_w,
                            vp_canvas_h,
                        );
                    }

                    // B. Sahnedeki 3D Varlıklar
                    let entities_3d: Vec<(Entity, Transform3D, Mesh3DComponent)> = self
                        .registry
                        .entities::<Transform3D>()
                        .into_iter()
                        .filter_map(|e| {
                            let t3 = self.registry.get::<Transform3D>(e).copied()?;
                            let m3 = self.registry.get::<Mesh3DComponent>(e).cloned()?;
                            Some((e, t3, m3))
                        })
                        .collect();

                    for (ent, t3, m3) in entities_3d {
                        let target_mesh = match m3.mesh_name.as_str() {
                            "sphere" => self.gpu_sphere.as_ref(),
                            "plane" => self.gpu_plane.as_ref(),
                            "cylinder" => self.gpu_cylinder.as_ref(),
                            "cone" => self.gpu_cone.as_ref(),
                            "pyramid" => self.gpu_pyramid.as_ref(),
                            "capsule" => self.gpu_capsule.as_ref(),
                            "sprite_3d" => self.gpu_sprite_3d.as_ref(),
                            _ => self.gpu_cube.as_ref(),
                        };

                        if let Some(mesh) = target_mesh {
                            let is_sel = selected_e == Some(ent);
                            let model_mat = t3.local_matrix();
                            let roughness = if is_sel {
                                (m3.roughness * 0.8).max(0.05)
                            } else {
                                m3.roughness
                            };
                            let metallic = m3.metallic;

                            pipe_3d.draw_mesh_pbr_viewport(
                                &ctx.device,
                                cmd,
                                &render_cam,
                                mesh,
                                model_mat,
                                &self.sun,
                                &self.ambient,
                                roughness,
                                metallic,
                                false,
                                vp_x,
                                vp_canvas_y,
                                vp_w,
                                vp_canvas_h,
                            );
                        }
                    }

                    // C. 2.5D Katman Derinliği Önizlemesi (2D Sprite'ları 3D Uzayda Gösterim)
                    // 2D koordinatları (piksel) 3D dünya birimine (metrik) ölçeklendir: 64 px = 1.0 birim.
                    // Z ekseni katman derinliği: z_index * 0.25 birim.
                    if let Some(sprite_mesh) = self.gpu_sprite_3d.as_ref() {
                        let scale_factor = 1.0 / 64.0;
                        let mut sprites_2d: Vec<(Entity, Transform2D, SpriteColor, i32)> =
                            Vec::new();
                        self.registry
                            .view2_ref_ref::<Transform2D, SpriteColor, _>(|e, tr, spr| {
                                // Yalnızca saf 3D varlık olmayanları veya 2D sprite'ları dahil et
                                if !self.registry.has::<Transform3D>(e) {
                                    let z = self
                                        .registry
                                        .get::<ZIndex2D>(e)
                                        .map(|zi| zi.z)
                                        .unwrap_or(0);
                                    sprites_2d.push((e, *tr, *spr, z));
                                }
                            });

                        for (ent, tr, _spr, z) in sprites_2d {
                            let is_sel = selected_e == Some(ent);
                            let pos_3d = glam::Vec3::new(
                                tr.pos.x * scale_factor,
                                -tr.pos.y * scale_factor,
                                (z as f32) * 0.25,
                            );
                            let size_3d = glam::Vec3::new(
                                tr.size.x * scale_factor,
                                tr.size.y * scale_factor,
                                1.0,
                            );
                            let rot_3d = glam::Quat::from_rotation_z(-tr.rotation);
                            let model_mat = glam::Mat4::from_scale_rotation_translation(
                                size_3d, rot_3d, pos_3d,
                            );

                            let roughness = if is_sel { 0.1 } else { 0.6 };
                            let metallic = if is_sel { 0.4 } else { 0.0 };

                            pipe_3d.draw_mesh_pbr_viewport(
                                &ctx.device,
                                cmd,
                                &render_cam,
                                sprite_mesh,
                                model_mat,
                                &self.sun,
                                &self.ambient,
                                roughness,
                                metallic,
                                false,
                                vp_x,
                                vp_canvas_y,
                                vp_w,
                                vp_canvas_h,
                            );
                        }
                    }
                }
            }

            // 2. 2D UI & Arayüz Katmanı Çizimi
            // Vulkan NDC standart projeksiyonu: y=0 tepe (-1.0), y=screen_h taban (+1.0)
            let view_proj = Mat4::orthographic_rh(0.0, screen_w, 0.0, screen_h, -1.0, 1.0);
            pipe.draw(
                &ctx.device,
                cmd,
                frame_idx,
                count as u32,
                view_proj,
                ctx.swapchain.extent,
            );

            ctx.end_rendering(cmd, image_index);
            let _ = ctx.end_frame(image_index, cmd, screen_w as u32, screen_h as u32);
        }

        self.frame_time_ms = render_start.elapsed().as_secs_f32() * 1000.0;
    }
}
