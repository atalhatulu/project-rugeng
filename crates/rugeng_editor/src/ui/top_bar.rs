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
    pub(crate) fn render_top_bar(&self, instances: &mut Vec<QuadInstance>, layout: &DockLayout) {
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
            ttf.draw_text(instances, "R", 13.0, 7.0, 15.0, [255, 255, 255, 255]);
        } else {
            font::draw_text(instances, "R", 13.0, 9.0, 1.4, [255, 255, 255, 255]);
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
                    instances,
                    name,
                    m_x + 8.0,
                    7.0,
                    14.0,
                    if is_open { theme.accent } else { text_norm },
                );
            } else {
                font::draw_text(
                    instances,
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
                    instances,
                    label,
                    ws_x + 8.0,
                    7.0,
                    14.0,
                    if is_cur { theme.accent } else { text_dim },
                );
            } else {
                font::draw_text(
                    instances,
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
                instances,
                "KAYDET",
                save_btn_x + 10.0,
                7.5,
                12.0,
                theme.text_primary,
            );
        } else {
            font::draw_text(
                instances,
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
                instances,
                "SARSINTI",
                shake_btn_x + 8.0,
                7.5,
                12.0,
                theme.text_primary,
            );
        } else {
            font::draw_text(
                instances,
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
            instances,
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
            instances,
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
            instances,
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
        draw_icon_step(instances, play_x + 90.0, 9.0, 11.0, theme.accent);

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
                        ("[D] Varligi Cogalt", "Ctrl+D", "duplicate"),
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
                        ("[M] glTF 3D Model (GLB)", "", "add_gltf_sample"),
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
                        instances,
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
                            instances,
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
                        instances,
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
                            instances,
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
    }
}
