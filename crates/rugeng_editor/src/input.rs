use crate::types::*;
use crate::App;
use glam::Vec2;
use rugeng_core::{
    Animation2DComponent, AnimationClip, Collider2D, Entity, Mesh3DComponent, ScriptComponent,
    SpriteColor, SpriteTexture, Text2DComponent, TileType, Tilemap2DComponent, Transform2D,
    Transform3D, Velocity2D, ZIndex2D,
};
use std::path::PathBuf;

impl App {
    pub(crate) fn handle_mouse_click(&mut self, mouse: Vec2, screen_w: f32, screen_h: f32) {
        let menu_h = 30.0;
        let left_w = 260.0;
        let right_w = 300.0;
        let bottom_h = 160.0;

        let left_top_h = (screen_h - menu_h) * 0.52;
        let left_bot_y = menu_h + left_top_h;
        let vp_x = left_w;
        let vp_w = screen_w - left_w - right_w;
        let scene_tab_h = 26.0;
        let vp_tool_h = 24.0;
        let vp_canvas_y = menu_h + scene_tab_h + vp_tool_h;
        let vp_canvas_h = screen_h - bottom_h - vp_canvas_y;
        let insp_x = screen_w - right_w;

        // Eğer açık bir Sağ Tık Bağlam Menüsü (Context Menu) varsa:
        if let Some(ctx_menu) = self.editor.context_menu.take() {
            let menu_w = 230.0;
            let item_h = 24.0;
            let menu_h_total = ctx_menu.items.len() as f32 * item_h + 8.0;
            let mut pos_x = ctx_menu.screen_pos.x;
            let mut pos_y = ctx_menu.screen_pos.y;

            if pos_x + menu_w > screen_w - 10.0 {
                pos_x = (screen_w - menu_w - 10.0).max(10.0);
            }
            if pos_y + menu_h_total > screen_h - 10.0 {
                pos_y = (screen_h - menu_h_total - 10.0).max(10.0);
            }

            if mouse.x >= pos_x
                && mouse.x <= pos_x + menu_w
                && mouse.y >= pos_y
                && mouse.y <= pos_y + menu_h_total
            {
                let rel_y = mouse.y - (pos_y + 4.0);
                let idx = (rel_y / item_h) as usize;
                if idx < ctx_menu.items.len() {
                    let action = ctx_menu.items[idx].action_id.clone();
                    match action.as_str() {
                        "duplicate" => self.duplicate_selected_entity(),
                        "delete" => self.delete_selected_entity(),
                        "z_up" => {
                            if let Some(e) = ctx_menu.target_entity {
                                let cur_z =
                                    self.registry.get::<ZIndex2D>(e).map(|z| z.z).unwrap_or(0);
                                self.registry.emplace(e, ZIndex2D::new(cur_z + 1));
                                self.editor.is_scene_dirty = true;
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Varlik #{}:{} Z-Index artirildi -> {}",
                                        e.index(),
                                        e.generation(),
                                        cur_z + 1
                                    ),
                                });
                            }
                        }
                        "z_down" => {
                            if let Some(e) = ctx_menu.target_entity {
                                let cur_z =
                                    self.registry.get::<ZIndex2D>(e).map(|z| z.z).unwrap_or(0);
                                self.registry.emplace(e, ZIndex2D::new(cur_z - 1));
                                self.editor.is_scene_dirty = true;
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Varlik #{}:{} Z-Index azaltildi -> {}",
                                        e.index(),
                                        e.generation(),
                                        cur_z - 1
                                    ),
                                });
                            }
                        }
                        preset if preset.starts_with("add_") => {
                            let p = &preset[4..];
                            self.spawn_preset_entity(p);
                        }
                        _ => {}
                    }
                    self.audio.play("sfx_blip");
                    return;
                }
            }
            // Menü dışına tıklandıysa menü kapatıldı (take() ile alındı), işlem bitti
            return;
        }

        // Eğer açık bir Dropdown Menü varsa, öncelikle onun tıklamalarını ele al:
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

            if mouse.x >= drop_x
                && mouse.x <= drop_x + menu_w
                && mouse.y >= drop_y
                && mouse.y <= drop_y + menu_h_total
            {
                let rel_y = mouse.y - (drop_y + 4.0);
                let idx = (rel_y / item_h) as usize;
                if idx < items.len() {
                    let (_, _, action_id) = items[idx];
                    match action_id {
                        "new" => self.new_scene(),
                        "save" => {
                            let path = self.editor.current_scene_path.clone();
                            self.save_current_scene(&path);
                        }
                        "save_as" => {
                            self.editor.scene_counter += 1;
                            let new_path =
                                format!("scenes/level_{}.scene", self.editor.scene_counter);
                            self.save_current_scene(&new_path);
                        }
                        "load" => {
                            let path = self.editor.current_scene_path.clone();
                            self.load_scene_from_file(&path);
                        }
                        "exit" => {
                            std::process::exit(0);
                        }
                        "undo" => self.undo(),
                        "redo" => self.redo(),
                        "duplicate" => self.duplicate_selected_entity(),
                        "delete" => self.delete_selected_entity(),
                        "select_all" => {
                            let all_e: Vec<Entity> = self.registry.entities::<Transform2D>();
                            self.editor.selected_entities = all_e.clone();
                            self.editor.selected_entity = all_e.first().copied();
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!("Sahnedeki tum varliklar ({}) secildi.", all_e.len()),
                            });
                        }
                        "add_ship" => self.spawn_preset_entity("ship"),
                        "add_enemy" => self.spawn_preset_entity("enemy"),
                        "add_heart" => self.spawn_preset_entity("heart"),
                        "add_orb" => self.spawn_preset_entity("orb"),
                        "add_tilemap" => self.spawn_preset_entity("tilemap"),
                        "add_text" => self.spawn_preset_entity("text"),
                        "add_cube_3d" => self.spawn_preset_entity("cube_3d"),
                        "add_sphere_3d" => self.spawn_preset_entity("sphere_3d"),
                        "add_plane_3d" => self.spawn_preset_entity("plane_3d"),
                        "add_cylinder_3d" => self.spawn_preset_entity("cylinder_3d"),
                        "add_cone_3d" => self.spawn_preset_entity("cone_3d"),
                        "add_pyramid_3d" => self.spawn_preset_entity("pyramid_3d"),
                        "add_capsule_3d" => self.spawn_preset_entity("capsule_3d"),
                        "add_sprite_3d" => self.spawn_preset_entity("sprite_3d"),
                        "export" => {
                            let path = self.editor.current_scene_path.clone();
                            self.save_current_scene(&path);
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: "Nihai Standalone Oyun Disa Aktarildi -> 'cargo run --bin rugeng_standalone'".to_string(),
                            });
                            self.audio.play("sfx_powerup");
                        }
                        "refresh_files" => {
                            self.editor.refresh_files();
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: "Dosya agaci ve varliklar yenilendi.".to_string(),
                            });
                            self.audio.play("sfx_blip");
                        }
                        "project_settings" => {
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: "Proje Ayarlari: Vulkan 1.3 HDR, 2D Pixel Renderer, Dynamic Instancing 8192.".to_string(),
                            });
                        }
                        "shake" => self.trigger_shake_and_explosion(),
                        "toggle_snap" => {
                            self.editor.snap_to_grid = !self.editor.snap_to_grid;
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: if self.editor.snap_to_grid {
                                    "Izgara Yapismasi (Snap) Acildi (16px).".to_string()
                                } else {
                                    "Izgara Yapismasi (Snap) Kapatildi.".to_string()
                                },
                            });
                        }
                        "clear_logs" => {
                            self.editor.logs.clear();
                        }
                        "help_shortcuts" => {
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: "Kisayollar: Z (Geri), Y (Yinele), T (Yeni), C (Kopyala), K (Kaydet), L (Yukle), X (Sarsinti/Patlama), Del (Sil).".to_string(),
                            });
                        }
                        "help_docs" => {
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: "Dokumantasyon: 'docs/README.md' ve tum sistem raporlari hazir durumdadir.".to_string(),
                            });
                        }
                        "help_about" => {
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: "Project Ember (RugEng 2D) - Saf Rust ve Vulkan 1.3 yuksek basarimli oyun motoru.".to_string(),
                            });
                        }
                        _ => {}
                    }
                    self.editor.active_menu = None;
                    self.audio.play("sfx_blip");
                    return;
                }
            }

            // Eğer menü dışına tıklandıysa ve üst bara tıklanmadıysa menüyü kapat
            if mouse.y > menu_h {
                self.editor.active_menu = None;
                return;
            }
        }

        // 1. ÜST MENÜ & ÇALIŞMA ALANI & OYNATMA BUTONLARI
        if mouse.y <= menu_h {
            // Oynatma Butonları (Play, Pause, Stop, Step)
            let play_x = screen_w - 140.0;
            // Play (Oynat - ▶)
            if mouse.x >= play_x && mouse.x <= play_x + 26.0 {
                if !self.editor.is_playing {
                    if self.editor.scene_snapshot.is_empty() {
                        self.registry.view1_ref::<Transform2D, _>(|e, t| {
                            self.editor.scene_snapshot.push((e, *t));
                        });
                    }
                    self.editor.is_playing = true;
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: "Oyun baslatildi (Simulasyon aktif).".to_string(),
                    });
                }
                return;
            }
            // Pause (Duraklat - ⏸)
            if mouse.x >= play_x + 28.0 && mouse.x <= play_x + 54.0 {
                if self.editor.is_playing {
                    self.editor.is_playing = false;
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: "Oyun duraklatildi.".to_string(),
                    });
                }
                return;
            }
            // Stop (Durdur - ⏹)
            if mouse.x >= play_x + 56.0 && mouse.x <= play_x + 82.0 {
                self.editor.is_playing = false;
                for &(e, t) in &self.editor.scene_snapshot {
                    if let Some(cur_t) = self.registry.get_mut::<Transform2D>(e) {
                        *cur_t = t;
                    }
                }
                self.editor.scene_snapshot.clear();
                self.editor.logs.push(LogMessage {
                    level: "INFO",
                    text: "Oyun durduruldu (Sahne ilk haline donduruldu).".to_string(),
                });
                return;
            }
            // Step (Adım İlerlet - ⏭)
            if mouse.x >= play_x + 84.0 && mouse.x <= play_x + 110.0 {
                if self.editor.scene_snapshot.is_empty() {
                    self.registry.view1_ref::<Transform2D, _>(|e, t| {
                        self.editor.scene_snapshot.push((e, *t));
                    });
                }
                self.step_simulation_single_frame(1.0 / 60.0);
                self.editor.logs.push(LogMessage {
                    level: "INFO",
                    text: "1 kare adim ilerletildi (1/60s).".to_string(),
                });
                return;
            }

            // Hızlı Butonlar ([KAYDET], [SARSINTI])
            let save_btn_x = play_x - 148.0;
            if mouse.x >= save_btn_x && mouse.x <= save_btn_x + 68.0 {
                let path = self.editor.current_scene_path.clone();
                self.save_current_scene(&path);
                return;
            }

            let shake_btn_x = play_x - 76.0;
            if mouse.x >= shake_btn_x && mouse.x <= shake_btn_x + 72.0 {
                self.trigger_shake_and_explosion();
                return;
            }

            // Açılır Menü Başlıkları (Scene, Edit, Add, Project, Debug, Help)
            let menu_headers = [
                ("Scene", 34.0, 54.0, OpenMenu::Scene),
                ("Edit", 90.0, 50.0, OpenMenu::Edit),
                ("Add", 142.0, 50.0, OpenMenu::Add),
                ("Project", 196.0, 64.0, OpenMenu::Project),
                ("Debug", 266.0, 56.0, OpenMenu::Debug),
                ("Help", 326.0, 50.0, OpenMenu::Help),
            ];

            for (_name, x, w, m) in menu_headers {
                if mouse.x >= x && mouse.x <= x + w {
                    if self.editor.active_menu == Some(m) {
                        self.editor.active_menu = None;
                    } else {
                        self.editor.active_menu = Some(m);
                        self.audio.play("sfx_blip");
                    }
                    return;
                }
            }

            // Çalışma Alanı Seçicileri
            let mut ws_x = screen_w * 0.5 - 180.0;
            let modes = [
                ("2D", WorkspaceMode::TwoD),
                ("3D", WorkspaceMode::ThreeD),
                ("Script", WorkspaceMode::Script),
                ("Game", WorkspaceMode::Game),
                ("Asset Store", WorkspaceMode::AssetStore),
            ];
            for (lbl, mode) in modes {
                let tab_w = (lbl.len() as f32) * 8.0 + 20.0;
                if mouse.x >= ws_x && mouse.x <= ws_x + tab_w {
                    self.editor.workspace = mode;
                    return;
                }
                ws_x += tab_w + 6.0;
            }
        }

        // 2. SOL SÜTUN — ÜST PANEL (SAHNE HIYERARŞISI)
        if mouse.x <= left_w && mouse.y > menu_h && mouse.y < left_bot_y {
            // Sekmeler (Scene vs Import)
            if mouse.y <= menu_h + 24.0 {
                if mouse.x <= 80.0 {
                    self.editor.left_tab = LeftTab::Scene;
                } else if mouse.x <= 162.0 {
                    self.editor.left_tab = LeftTab::Import;
                }
                return;
            }

            // [+] Düğüm Ekle Butonu
            let filter_y = menu_h + 26.0;
            if mouse.y >= filter_y
                && mouse.y <= filter_y + 20.0
                && mouse.x >= 6.0
                && mouse.x <= 30.0
            {
                self.spawn_entity_at_camera();
                return;
            }

            let root_y = filter_y + 26.0;
            let b1_y = root_y + 16.0;
            let b2_y = b1_y + 24.0;
            let b3_y = b2_y + 24.0;
            let b4_y = b3_y + 24.0;
            let b5_y = b4_y + 24.0;
            let act_bar_y = b5_y + 24.0;

            let is_3d_ws = self.editor.workspace == WorkspaceMode::ThreeD;

            if is_3d_ws {
                // 3D Satır 1: [+ Kup 3D] ve [+ Kure 3D]
                if mouse.y >= b1_y && mouse.y <= b1_y + 20.0 {
                    if mouse.x >= 8.0 && mouse.x <= 126.0 {
                        self.spawn_preset_entity("cube_3d");
                        return;
                    } else if mouse.x >= 132.0 && mouse.x <= 252.0 {
                        self.spawn_preset_entity("sphere_3d");
                        return;
                    }
                }
                // 3D Satır 2: [+ Silindir] ve [+ Koni]
                if mouse.y >= b2_y && mouse.y <= b2_y + 20.0 {
                    if mouse.x >= 8.0 && mouse.x <= 126.0 {
                        self.spawn_preset_entity("cylinder_3d");
                        return;
                    } else if mouse.x >= 132.0 && mouse.x <= 252.0 {
                        self.spawn_preset_entity("cone_3d");
                        return;
                    }
                }
                // 3D Satır 3: [+ Piramit] ve [+ Kapsul]
                if mouse.y >= b3_y && mouse.y <= b3_y + 20.0 {
                    if mouse.x >= 8.0 && mouse.x <= 126.0 {
                        self.spawn_preset_entity("pyramid_3d");
                        return;
                    } else if mouse.x >= 132.0 && mouse.x <= 252.0 {
                        self.spawn_preset_entity("capsule_3d");
                        return;
                    }
                }
                // 3D Satır 4: [+ Duzlem] ve [+ Sprite3D]
                if mouse.y >= b4_y && mouse.y <= b4_y + 20.0 {
                    if mouse.x >= 8.0 && mouse.x <= 126.0 {
                        self.spawn_preset_entity("plane_3d");
                        return;
                    } else if mouse.x >= 132.0 && mouse.x <= 252.0 {
                        self.spawn_preset_entity("sprite_3d");
                        return;
                    }
                }
                // 3D Satır 5: [+ Directional Light 3D]
                if mouse.y >= b5_y
                    && mouse.y <= b5_y + 20.0
                    && mouse.x >= 8.0
                    && mouse.x <= left_w - 8.0
                {
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: "Directional Sun Isik kaynagi guncellendi.".to_string(),
                    });
                    self.audio.play("sfx_blip");
                    return;
                }
            } else {
                // 2D Satır 1: [+ Gemi] ve [+ Bocek]
                if mouse.y >= b1_y && mouse.y <= b1_y + 20.0 {
                    if mouse.x >= 8.0 && mouse.x <= 126.0 {
                        self.spawn_preset_entity("ship");
                        return;
                    } else if mouse.x >= 132.0 && mouse.x <= 252.0 {
                        self.spawn_preset_entity("enemy");
                        return;
                    }
                }
                // 2D Satır 2: [+ Kure] ve [+ Kalp]
                if mouse.y >= b2_y && mouse.y <= b2_y + 20.0 {
                    if mouse.x >= 8.0 && mouse.x <= 126.0 {
                        self.spawn_preset_entity("orb");
                        return;
                    } else if mouse.x >= 132.0 && mouse.x <= 252.0 {
                        self.spawn_preset_entity("heart");
                        return;
                    }
                }
                // 2D Satır 3: [+ Bos 2D]
                if mouse.y >= b3_y
                    && mouse.y <= b3_y + 20.0
                    && mouse.x >= 8.0
                    && mouse.x <= left_w - 8.0
                {
                    self.spawn_preset_entity("empty");
                    return;
                }
                // 2D Satır 4: [+ Anim Gemi] ve [+ Harita]
                if mouse.y >= b4_y && mouse.y <= b4_y + 20.0 {
                    if mouse.x >= 8.0 && mouse.x <= 126.0 {
                        self.spawn_preset_entity("anim_player");
                        return;
                    } else if mouse.x >= 132.0 && mouse.x <= 252.0 {
                        self.spawn_preset_entity("tilemap");
                        return;
                    }
                }
                // 2D Satır 5: [+ Metin 2D]
                if mouse.y >= b5_y
                    && mouse.y <= b5_y + 20.0
                    && mouse.x >= 8.0
                    && mouse.x <= left_w - 8.0
                {
                    self.spawn_preset_entity("text");
                    return;
                }
            }
            // Seçili Düğüm Eylemleri: [KOPYALA] [SIL] [YENI SAHNE]
            if mouse.y >= act_bar_y && mouse.y <= act_bar_y + 20.0 {
                if mouse.x >= 8.0 && mouse.x <= 82.0 {
                    self.duplicate_selected_entity();
                    return;
                } else if mouse.x >= 86.0 && mouse.x <= 160.0 {
                    self.delete_selected_entity();
                    return;
                } else if mouse.x >= 164.0 && mouse.x <= 252.0 {
                    self.new_scene();
                    return;
                }
            }

            // Düğümlere Tıklama (Varlık Seçimi)
            let mut row_y = act_bar_y + 26.0;
            let mut clicked_e = None;
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
                if mouse.y >= row_y && mouse.y <= row_y + 20.0 {
                    clicked_e = Some(e);
                    break;
                }
                row_y += 24.0;
            }
            if let Some(e) = clicked_e {
                self.editor.selected_entity = Some(e);
                self.editor.logs.push(LogMessage {
                    level: "INFO",
                    text: format!("Dugum #{}:{} secildi.", e.index(), e.generation()),
                });
                return;
            }
        }

        // 3. SOL SÜTUN — ALT PANEL (DOSYA SISTEMI)
        if mouse.x <= left_w && mouse.y >= left_bot_y {
            let fs_bar_y = left_bot_y + 28.0;
            // '<' Geri Butonu
            if mouse.y >= fs_bar_y
                && mouse.y <= fs_bar_y + 20.0
                && mouse.x >= 6.0
                && mouse.x <= 28.0
            {
                if self.editor.current_path.as_os_str() != "." {
                    self.editor.current_path.pop();
                    if self.editor.current_path.as_os_str().is_empty() {
                        self.editor.current_path = PathBuf::from(".");
                    }
                    self.editor.refresh_files();
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!("Dizin: res://{}", self.editor.current_path.display()),
                    });
                }
                return;
            }

            let fs_filter_y = fs_bar_y + 24.0;
            let tree_y = fs_filter_y + 26.0;
            let mut f_y = tree_y + 4.0;

            // [..] Üst Dizin Butonuna Tıklama
            if self.editor.current_path.as_os_str() != "." {
                if mouse.y >= f_y - 2.0 && mouse.y <= f_y + 18.0 {
                    self.editor.current_path.pop();
                    if self.editor.current_path.as_os_str().is_empty() {
                        self.editor.current_path = PathBuf::from(".");
                    }
                    self.editor.refresh_files();
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!("Dizin: res://{}", self.editor.current_path.display()),
                    });
                    return;
                }
                f_y += 20.0;
            }

            // Dosya veya Klasör Satırına Tıklama
            let mut clicked_item = None;
            for item in &self.editor.files_in_dir {
                if f_y + 18.0 > screen_h - 4.0 {
                    break;
                }
                if mouse.y >= f_y - 2.0 && mouse.y <= f_y + 18.0 {
                    clicked_item = Some(item.clone());
                    break;
                }
                f_y += 20.0;
            }

            if let Some(item) = clicked_item {
                if item.is_dir {
                    self.editor.current_path.push(&item.name);
                    self.editor.refresh_files();
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!(
                            "Klasore girildi: res://{}",
                            self.editor.current_path.display()
                        ),
                    });
                } else {
                    self.editor.selected_file = item.name.clone();
                    if item.name.ends_with(".scene") {
                        let path = self.editor.current_path.join(&item.name);
                        self.load_scene_from_file(&path.to_string_lossy());
                    } else {
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: format!("Dosya secildi: {}", item.name),
                        });
                    }
                }
                return;
            }
        }

        // 4. ORTA ALAN: VIEWPORT (SAHNE SEKMLERİ, ARAÇLAR, ZOOM)
        if mouse.x >= vp_x
            && mouse.x <= vp_x + vp_w
            && mouse.y > menu_h
            && mouse.y < screen_h - bottom_h
        {
            // Sahne Sekmeleri (Aktif Sahne, player.scene, + Yeni Sahne)
            if mouse.y >= menu_h && mouse.y <= menu_h + scene_tab_h {
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
                let tab2_x = vp_x + 8.0 + tab1_w + 4.0;
                let plus_x = tab2_x + 94.0;

                if mouse.x >= vp_x + 8.0 && mouse.x <= tab2_x {
                    // Aktif sahne
                } else if mouse.x >= tab2_x && mouse.x <= plus_x {
                    self.load_scene_from_file("scenes/main.scene");
                } else if mouse.x >= plus_x && mouse.x <= plus_x + 36.0 {
                    self.new_scene();
                }
                return;
            }

            // Viewport Alt Araç Çubuğu (Select, Move, Rotate, Scale, Brush, Eraser, Palette, Snap)
            if mouse.y >= menu_h + scene_tab_h && mouse.y <= vp_canvas_y {
                let mut t_x = vp_x + 6.0;
                for &(tool, _, btn_w) in &VIEWPORT_TOOLS {
                    if mouse.x >= t_x && mouse.x <= t_x + btn_w {
                        self.editor.active_tool = tool;
                        return;
                    }
                    t_x += btn_w + 4.0;
                }

                // Eğer Fırça veya Silgi aktifse karo paletine tıklama kontrolü
                if self.editor.active_tool == ViewportTool::TileBrush
                    || self.editor.active_tool == ViewportTool::TileEraser
                {
                    t_x += 8.0;
                    for &(lbl, tt, btn_w) in &TILE_PALETTE_ITEMS {
                        if mouse.x >= t_x && mouse.x <= t_x + btn_w {
                            self.editor.active_tile_type = tt;
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!("Aktif Karo Tipi: {} ({:?})", lbl, tt),
                            });
                            return;
                        }
                        t_x += btn_w + 4.0;
                    }
                }

                // Grid Snap Butonu (Sağa hizalı)
                let snap_x = vp_x + vp_w - 74.0;
                if mouse.x >= snap_x && mouse.x <= snap_x + 66.0 {
                    self.editor.snap_to_grid = !self.editor.snap_to_grid;
                    let msg = if self.editor.snap_to_grid {
                        "Izgara Yapismasi (Snap) Acildi (16px)."
                    } else {
                        "Izgara Yapismasi (Snap) Kapatildi."
                    };
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: msg.to_string(),
                    });
                    return;
                }
            }

            // Yüzen Zoom Rozeti ([-] % [+] ) (Sadece 2D Modunda)
            if self.editor.workspace == WorkspaceMode::TwoD {
                let zoom_box_x = vp_x + 28.0;
                let zoom_box_y = vp_canvas_y + 20.0;
                if mouse.x >= zoom_box_x
                    && mouse.x <= zoom_box_x + 116.0
                    && mouse.y >= zoom_box_y
                    && mouse.y <= zoom_box_y + 20.0
                {
                    if mouse.x <= zoom_box_x + 30.0 {
                        // [-] Küçült
                        self.editor.camera.zoom = (self.editor.camera.zoom / 1.15).max(0.2);
                    } else if mouse.x >= zoom_box_x + 86.0 {
                        // [+] Büyüt
                        self.editor.camera.zoom = (self.editor.camera.zoom * 1.15).min(5.0);
                    }
                    return;
                }
            }

            // Viewport Tuval Alanı Tıklama & Nesne Manipülasyonu
            if mouse.x >= vp_x
                && mouse.x <= vp_x + vp_w
                && mouse.y >= vp_canvas_y
                && mouse.y <= vp_canvas_y + vp_canvas_h
            {
                // 3D Workspace Modundayken: 3D Ray / Screen Picking & Gizmo İşlemleri
                if self.editor.workspace == WorkspaceMode::ThreeD {
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

                    // A. Seçili 3D Varlığın Gizmo Eksenlerine Tıklama Kontrolü
                    if let Some(se) = self.editor.selected_entity {
                        if let Some(t3) = self.registry.get::<Transform3D>(se) {
                            let center = t3.translation;
                            if let Some(sc) = project_pt(center) {
                                let tool = self.editor.active_tool;

                                // 1. Rotate Gizmo Çemberleri Kontrolü (Rotate veya Select aracında)
                                if tool == ViewportTool::Rotate || tool == ViewportTool::Select {
                                    let radius = 1.25;
                                    let segments = 24;
                                    // Pitch (X Ekseni etrafında - Kırmızı yay)
                                    let mut hit_pitch = false;
                                    for i in 0..=segments {
                                        let angle =
                                            (i as f32 / segments as f32) * std::f32::consts::TAU;
                                        let pt_w = center
                                            + glam::Vec3::new(
                                                0.0,
                                                angle.cos() * radius,
                                                angle.sin() * radius,
                                            );
                                        if let Some(sp) = project_pt(pt_w) {
                                            if (mouse - sp).length() <= 10.0 {
                                                hit_pitch = true;
                                                break;
                                            }
                                        }
                                    }
                                    if hit_pitch {
                                        self.editor.drag_mode = DragMode::Rotate3D {
                                            axis: 0, // X ekseni etrafında (Pitch)
                                            start_rotation: t3.rotation,
                                            start_mouse: mouse,
                                        };
                                        self.audio.play("sfx_blip");
                                        return;
                                    }

                                    // Yaw (Y Ekseni etrafında - Yeşil yay)
                                    let mut hit_yaw = false;
                                    for i in 0..=segments {
                                        let angle =
                                            (i as f32 / segments as f32) * std::f32::consts::TAU;
                                        let pt_w = center
                                            + glam::Vec3::new(
                                                angle.cos() * radius,
                                                0.0,
                                                angle.sin() * radius,
                                            );
                                        if let Some(sp) = project_pt(pt_w) {
                                            if (mouse - sp).length() <= 10.0 {
                                                hit_yaw = true;
                                                break;
                                            }
                                        }
                                    }
                                    if hit_yaw {
                                        self.editor.drag_mode = DragMode::Rotate3D {
                                            axis: 1, // Y ekseni etrafında (Yaw)
                                            start_rotation: t3.rotation,
                                            start_mouse: mouse,
                                        };
                                        self.audio.play("sfx_blip");
                                        return;
                                    }

                                    // Roll (Z Ekseni etrafında - Mavi yay)
                                    let mut hit_roll = false;
                                    for i in 0..=segments {
                                        let angle =
                                            (i as f32 / segments as f32) * std::f32::consts::TAU;
                                        let pt_w = center
                                            + glam::Vec3::new(
                                                angle.cos() * radius,
                                                angle.sin() * radius,
                                                0.0,
                                            );
                                        if let Some(sp) = project_pt(pt_w) {
                                            if (mouse - sp).length() <= 10.0 {
                                                hit_roll = true;
                                                break;
                                            }
                                        }
                                    }
                                    if hit_roll {
                                        self.editor.drag_mode = DragMode::Rotate3D {
                                            axis: 2, // Z ekseni etrafında (Roll)
                                            start_rotation: t3.rotation,
                                            start_mouse: mouse,
                                        };
                                        self.audio.play("sfx_blip");
                                        return;
                                    }
                                }

                                // 2. Eksen Okları ve Tutamaçları (Translate veya Scale)
                                let axis_len = 1.6;

                                // X Ekseni (Kırmızı)
                                if let Some(sx) = project_pt(center + glam::Vec3::X * axis_len) {
                                    let diff = sx - sc;
                                    let len = diff.length();
                                    if len > 1.0 {
                                        let dir = diff / len;
                                        let to_m = mouse - sc;
                                        let proj = to_m.dot(dir);
                                        let perp = (to_m - dir * proj).length();
                                        if (0.0..=(len + 12.0)).contains(&proj) && perp <= 14.0 {
                                            if tool == ViewportTool::Scale {
                                                self.editor.drag_mode = DragMode::Scale3D {
                                                    axis: 0,
                                                    start_scale: t3.scale,
                                                    start_mouse: mouse,
                                                };
                                            } else {
                                                self.editor.drag_mode = DragMode::Translate3D {
                                                    axis: 0,
                                                    start_translation: t3.translation,
                                                    start_mouse: mouse,
                                                };
                                            }
                                            self.audio.play("sfx_blip");
                                            return;
                                        }
                                    }
                                }

                                // Y Ekseni (Yeşil)
                                if let Some(sy) = project_pt(center + glam::Vec3::Y * axis_len) {
                                    let diff = sy - sc;
                                    let len = diff.length();
                                    if len > 1.0 {
                                        let dir = diff / len;
                                        let to_m = mouse - sc;
                                        let proj = to_m.dot(dir);
                                        let perp = (to_m - dir * proj).length();
                                        if (0.0..=(len + 12.0)).contains(&proj) && perp <= 14.0 {
                                            if tool == ViewportTool::Scale {
                                                self.editor.drag_mode = DragMode::Scale3D {
                                                    axis: 1,
                                                    start_scale: t3.scale,
                                                    start_mouse: mouse,
                                                };
                                            } else {
                                                self.editor.drag_mode = DragMode::Translate3D {
                                                    axis: 1,
                                                    start_translation: t3.translation,
                                                    start_mouse: mouse,
                                                };
                                            }
                                            self.audio.play("sfx_blip");
                                            return;
                                        }
                                    }
                                }

                                // Z Ekseni (Mavi)
                                if let Some(sz) = project_pt(center + glam::Vec3::Z * axis_len) {
                                    let diff = sz - sc;
                                    let len = diff.length();
                                    if len > 1.0 {
                                        let dir = diff / len;
                                        let to_m = mouse - sc;
                                        let proj = to_m.dot(dir);
                                        let perp = (to_m - dir * proj).length();
                                        if (0.0..=(len + 12.0)).contains(&proj) && perp <= 14.0 {
                                            if tool == ViewportTool::Scale {
                                                self.editor.drag_mode = DragMode::Scale3D {
                                                    axis: 2,
                                                    start_scale: t3.scale,
                                                    start_mouse: mouse,
                                                };
                                            } else {
                                                self.editor.drag_mode = DragMode::Translate3D {
                                                    axis: 2,
                                                    start_translation: t3.translation,
                                                    start_mouse: mouse,
                                                };
                                            }
                                            self.audio.play("sfx_blip");
                                            return;
                                        }
                                    }
                                }

                                // 3. Merkez Seçici / Taşıyıcı
                                if (mouse - sc).length() <= 14.0 {
                                    if tool == ViewportTool::Scale {
                                        self.editor.drag_mode = DragMode::Scale3D {
                                            axis: 3, // Uniform Scale
                                            start_scale: t3.scale,
                                            start_mouse: mouse,
                                        };
                                    } else {
                                        self.editor.drag_mode = DragMode::Translate3D {
                                            axis: 0, // Varsayılan serbest taşıma
                                            start_translation: t3.translation,
                                            start_mouse: mouse,
                                        };
                                    }
                                    return;
                                }
                            }
                        }
                    }

                    // B. 3D Sahnedeki Varlıkları Seçme (Raycast / Nearest Projection)
                    let mut hit_3d = None;
                    let mut closest_dist = 36.0; // 36 piksel seçme toleransı

                    let entities_3d: Vec<(Entity, Transform3D)> = self
                        .registry
                        .entities::<Transform3D>()
                        .into_iter()
                        .filter_map(|e| {
                            let t3 = self.registry.get::<Transform3D>(e).copied()?;
                            Some((e, t3))
                        })
                        .collect();

                    for (ent, t3) in entities_3d {
                        if let Some(sp) = project_pt(t3.translation) {
                            let dist = (mouse - sp).length();
                            if dist < closest_dist {
                                closest_dist = dist;
                                hit_3d = Some(ent);
                            }
                        }
                    }

                    if let Some(new_3d) = hit_3d {
                        self.editor.selected_entity = Some(new_3d);
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: format!(
                                "3D Varlik #{}:{} secildi.",
                                new_3d.index(),
                                new_3d.generation()
                            ),
                        });
                        self.audio.play("sfx_blip");
                        return;
                    }

                    // Boş 3D alana tıklandıysa seçimi kaldır
                    self.editor.selected_entity = None;
                    self.editor.drag_mode = DragMode::None;
                    return;
                }

                // 2D Workspace Modu: 2D Tuval Alanı Tıklama & Nesne Manipülasyonu
                let cam = &self.editor.camera;
                let vp_center = Vec2::new(vp_x + vp_w * 0.5, vp_canvas_y + vp_canvas_h * 0.5);
                let world_m = (mouse - vp_center) / cam.zoom + cam.pos;

                // Fırça veya Silgi aktifse doğrudan karo haritasını güncelle ve dön
                if self.editor.active_tool == ViewportTool::TileBrush
                    || self.editor.active_tool == ViewportTool::TileEraser
                {
                    self.apply_tile_tool(world_m);
                    return;
                }

                // 1. Seçili Varlık ve Gizmo Tutamaç Kontrolü
                if let Some(e) = self.editor.selected_entity {
                    if let Some(tr) = self.registry.get::<Transform2D>(e).copied() {
                        self.editor.drag_start_transform = Some(tr);
                        let screen_pos = vp_center + (tr.pos - cam.pos) * cam.zoom;
                        let screen_size = tr.size * cam.zoom;
                        let draw_x = screen_pos.x - screen_size.x * 0.5;
                        let draw_y = screen_pos.y - screen_size.y * 0.5;

                        // A. Döndürme Tutamacı (Heading Tip) Kontrolü
                        let rot_len = 38.0;
                        let tip_x = screen_pos.x + tr.rotation.cos() * rot_len;
                        let tip_y = screen_pos.y + tr.rotation.sin() * rot_len;
                        let dist_rot = (mouse - Vec2::new(tip_x, tip_y)).length();
                        if dist_rot <= 14.0 {
                            let start_angle =
                                (mouse.y - screen_pos.y).atan2(mouse.x - screen_pos.x);
                            self.editor.drag_mode = DragMode::Rotate {
                                center: tr.pos,
                                start_rotation: tr.rotation,
                                start_angle,
                            };
                            return;
                        }

                        // B. 4 Köşe Büyütme/Küçültme Tutamacı (Scale Handles) Kontrolü
                        let corners = [
                            Vec2::new(draw_x, draw_y),
                            Vec2::new(draw_x + screen_size.x, draw_y),
                            Vec2::new(draw_x, draw_y + screen_size.y),
                            Vec2::new(draw_x + screen_size.x, draw_y + screen_size.y),
                        ];
                        let mut hit_corner = false;
                        for c in corners {
                            if (mouse - c).length() <= 12.0 {
                                hit_corner = true;
                                break;
                            }
                        }
                        if hit_corner {
                            self.editor.drag_mode = DragMode::Scale {
                                start_size: tr.size,
                                start_world_mouse: world_m,
                            };
                            return;
                        }

                        // C. Gövde Tıklaması (Aktif araca göre işlem)
                        if mouse.x >= draw_x
                            && mouse.x <= draw_x + screen_size.x
                            && mouse.y >= draw_y
                            && mouse.y <= draw_y + screen_size.y
                        {
                            match self.editor.active_tool {
                                ViewportTool::Rotate => {
                                    let start_angle =
                                        (mouse.y - screen_pos.y).atan2(mouse.x - screen_pos.x);
                                    self.editor.drag_mode = DragMode::Rotate {
                                        center: tr.pos,
                                        start_rotation: tr.rotation,
                                        start_angle,
                                    };
                                }
                                ViewportTool::Scale => {
                                    self.editor.drag_mode = DragMode::Scale {
                                        start_size: tr.size,
                                        start_world_mouse: world_m,
                                    };
                                }
                                ViewportTool::Select
                                | ViewportTool::Move
                                | ViewportTool::TileBrush
                                | ViewportTool::TileEraser => {
                                    self.editor.drag_mode = DragMode::Translate {
                                        start_entity_pos: tr.pos,
                                        start_world_mouse: world_m,
                                    };
                                }
                            }
                            return;
                        }
                    }
                }

                // 2. Sahnedeki Diğer Varlıkları Seçme & Tutup Sürükleme
                let mut hit_entity = None;
                let mut hit_tr = None;
                self.registry
                    .view1_ref::<Transform2D, _>(|other_e, other_tr| {
                        if hit_entity.is_none() {
                            let half_sz = other_tr.size * 0.5;
                            if world_m.x >= other_tr.pos.x - half_sz.x
                                && world_m.x <= other_tr.pos.x + half_sz.x
                                && world_m.y >= other_tr.pos.y - half_sz.y
                                && world_m.y <= other_tr.pos.y + half_sz.y
                            {
                                hit_entity = Some(other_e);
                                hit_tr = Some(*other_tr);
                            }
                        }
                    });

                if let (Some(new_e), Some(new_tr)) = (hit_entity, hit_tr) {
                    self.editor.selected_entity = Some(new_e);
                    self.editor.drag_start_transform = Some(new_tr);
                    self.editor.logs.push(LogMessage {
                        level: "INFO",
                        text: format!("Dugum #{}:{} secildi.", new_e.index(), new_e.generation()),
                    });
                    match self.editor.active_tool {
                        ViewportTool::Rotate => {
                            let other_screen_pos = vp_center + (new_tr.pos - cam.pos) * cam.zoom;
                            let start_angle =
                                (mouse.y - other_screen_pos.y).atan2(mouse.x - other_screen_pos.x);
                            self.editor.drag_mode = DragMode::Rotate {
                                center: new_tr.pos,
                                start_rotation: new_tr.rotation,
                                start_angle,
                            };
                        }
                        ViewportTool::Scale => {
                            self.editor.drag_mode = DragMode::Scale {
                                start_size: new_tr.size,
                                start_world_mouse: world_m,
                            };
                        }
                        ViewportTool::Select
                        | ViewportTool::Move
                        | ViewportTool::TileBrush
                        | ViewportTool::TileEraser => {
                            self.editor.drag_mode = DragMode::Translate {
                                start_entity_pos: new_tr.pos,
                                start_world_mouse: world_m,
                            };
                        }
                    }
                    return;
                }

                // Boş Tuvale Tıklama: Seçimi Kaldır
                self.editor.selected_entity = None;
                self.editor.drag_mode = DragMode::None;
                return;
            }
        }

        // 5. ALT PANEL: KONSOL & ÇIKTI (OUTPUT SHELF)
        if mouse.x >= vp_x && mouse.x <= vp_x + vp_w && mouse.y >= screen_h - bottom_h {
            let btm_tab_y = screen_h - 24.0;
            if mouse.y >= btm_tab_y {
                let mut b_x = vp_x + 8.0;
                let btm_tabs = [BottomShelfTab::Output, BottomShelfTab::Debugger];
                for tab in btm_tabs {
                    if mouse.x >= b_x && mouse.x <= b_x + 78.0 {
                        self.editor.bottom_shelf_tab = tab;
                        return;
                    }
                    b_x += 82.0;
                }
            }
        }

        // 6. SAĞ SÜTUN: DENETÇİ (INSPECTOR)
        if mouse.x >= insp_x && mouse.y > menu_h {
            if let Some(e) = self.editor.selected_entity {
                let insp_search_y = menu_h + 28.0;
                let card_y = insp_search_y + 28.0;

                let mut cur_insp_y = card_y + 30.0;
                // 3D Varlık Kontrolleri (Transform3D & Mesh3DComponent)
                if self.registry.has::<Transform3D>(e) {
                    let c3d_y = cur_insp_y;
                    // Pos X- / X+
                    if mouse.y >= c3d_y + 24.0 && mouse.y <= c3d_y + 39.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 184.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.translation.x -= 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 188.0 && mouse.x <= insp_x + 212.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.translation.x += 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        // Pos Y- / Y+
                        if mouse.x >= insp_x + 216.0 && mouse.x <= insp_x + 240.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.translation.y -= 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 244.0 && mouse.x <= insp_x + 268.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.translation.y += 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                    }
                    // Pos Z- / Z+
                    if mouse.y >= c3d_y + 44.0 && mouse.y <= c3d_y + 59.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 184.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.translation.z -= 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 188.0 && mouse.x <= insp_x + 212.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.translation.z += 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                    }
                    // Pitch P- / P+, Yaw Y- / Y+
                    if mouse.y >= c3d_y + 66.0 && mouse.y <= c3d_y + 81.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 184.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_x(-0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 188.0 && mouse.x <= insp_x + 212.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_x(0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 216.0 && mouse.x <= insp_x + 240.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_y(-0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 244.0 && mouse.x <= insp_x + 268.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_y(0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                    }
                    // Scale S- / S+
                    if mouse.y >= c3d_y + 88.0 && mouse.y <= c3d_y + 103.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.scale =
                                    (t3.scale - glam::Vec3::splat(0.2)).max(glam::Vec3::splat(0.1));
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t3 = self.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = self.registry.get_mut::<Transform3D>(e) {
                                t3.scale += glam::Vec3::splat(0.2);
                            }
                            if let Some(old_t3) = old_t3 {
                                self.push_transform3d_undo(e, old_t3);
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                    }
                    // Mesh Seçim Butonları: Cube, Sphere, Plane
                    if mouse.y >= c3d_y + 130.0 && mouse.y <= c3d_y + 146.0 {
                        let mesh_btns = [
                            ("cube", insp_x + 14.0),
                            ("sphere", insp_x + 66.0),
                            ("plane", insp_x + 118.0),
                        ];
                        for (name, bx) in mesh_btns {
                            if mouse.x >= bx && mouse.x <= bx + 48.0 {
                                if let Some(m3) = self.registry.get_mut::<Mesh3DComponent>(e) {
                                    m3.mesh_name = name.to_string();
                                }
                                self.audio.play("sfx_blip");
                                return;
                            }
                        }
                    }
                    cur_insp_y += 160.0 + 8.0;
                }

                let has_tr3d = self.registry.has::<Transform3D>(e);
                let has_tr2d = self.registry.has::<Transform2D>(e);
                if has_tr2d || !has_tr3d {
                    let c1_y = cur_insp_y;

                    // Position X: [-] ve [+]
                    if mouse.y >= c1_y + 24.0 && mouse.y <= c1_y + 40.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.pos.x -= 10.0;
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.pos.x += 10.0;
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                    }

                    // Position Y: [-] ve [+]
                    if mouse.y >= c1_y + 44.0 && mouse.y <= c1_y + 60.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.pos.y -= 10.0;
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.pos.y += 10.0;
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                    }

                    // Rotation: R- ve R+
                    if mouse.y >= c1_y + 66.0 && mouse.y <= c1_y + 82.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.rotation -= 0.1;
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.rotation += 0.1;
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                    }

                    // Scale: W- ve W+
                    if mouse.y >= c1_y + 88.0 && mouse.y <= c1_y + 104.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.size.x = (t.size.x - 8.0).max(8.0);
                                t.size.y = (t.size.y - 8.0).max(8.0);
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = self.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = self.registry.get_mut::<Transform2D>(e) {
                                t.size.x += 8.0;
                                t.size.y += 8.0;
                            }
                            if let Some(old_t) = old_t {
                                self.push_transform2d_undo(e, old_t);
                            }
                            return;
                        }
                    }

                    // Z-Index: Z- ve Z+
                    if mouse.y >= c1_y + 110.0 && mouse.y <= c1_y + 126.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_z = self.registry.get::<ZIndex2D>(e).copied();
                            let new_z = if let Some(zi) = self.registry.get_mut::<ZIndex2D>(e) {
                                zi.z -= 1;
                                zi.z
                            } else {
                                self.registry.emplace(e, ZIndex2D::new(-1));
                                -1
                            };
                            self.push_zindex_undo(e, old_z);
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!("Dugum #{}: Z-Index {} yapildi.", e.index(), new_z),
                            });
                            self.audio.play("sfx_blip");
                            return;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_z = self.registry.get::<ZIndex2D>(e).copied();
                            let new_z = if let Some(zi) = self.registry.get_mut::<ZIndex2D>(e) {
                                zi.z += 1;
                                zi.z
                            } else {
                                self.registry.emplace(e, ZIndex2D::new(1));
                                1
                            };
                            self.push_zindex_undo(e, old_z);
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!("Dugum #{}: Z-Index {} yapildi.", e.index(), new_z),
                            });
                            self.audio.play("sfx_blip");
                            return;
                        }
                    }
                    cur_insp_y += 136.0 + 8.0;
                }

                // Sprite 2D Doku Butonlarına Tıklama
                let c2_y = cur_insp_y;
                if mouse.y >= c2_y + 40.0 && mouse.y <= c2_y + 56.0 {
                    let tex_options = [
                        ("ship", "player_ship"),
                        ("bug", "enemy_bug"),
                        ("orb", "energy_orb"),
                        ("heart", "heart_icon"),
                        ("none", "none"),
                    ];
                    let mut opt_x = insp_x + 14.0;
                    for (_label, key) in tex_options {
                        if mouse.x >= opt_x && mouse.x <= opt_x + 38.0 {
                            let old_tex = self.registry.get::<SpriteTexture>(e).cloned();
                            if key == "none" {
                                self.registry.remove::<SpriteTexture>(e);
                                self.push_texture_undo(e, old_tex);
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} dokusu kaldirildi.",
                                        e.index(),
                                        e.generation()
                                    ),
                                });
                            } else {
                                self.registry.emplace(
                                    e,
                                    SpriteTexture {
                                        name: key.to_string(),
                                    },
                                );
                                self.push_texture_undo(e, old_tex);
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} dokusu res://{} olarak atandi.",
                                        e.index(),
                                        e.generation(),
                                        key
                                    ),
                                });
                            }
                            return;
                        }
                        opt_x += 42.0;
                    }
                }

                // Sprite 2D Renk Paletine Tıklama
                if mouse.y >= c2_y + 60.0 && mouse.y <= c2_y + 78.0 {
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
                        if mouse.x >= sw_x && mouse.x <= sw_x + 18.0 {
                            let old_col = self.registry.get::<SpriteColor>(e).copied();
                            if let Some(s) = self.registry.get_mut::<SpriteColor>(e) {
                                s.color = col;
                            }
                            self.push_color_undo(e, old_col);
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} rengi guncellendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            return;
                        }
                        sw_x += 22.0;
                    }
                }

                // CollisionShape 2D Mod Değiştirme Butonu ([Sensor / Kati])
                let c3_y = c2_y + 92.0;
                if mouse.y >= c3_y + 24.0
                    && mouse.y <= c3_y + 44.0
                    && mouse.x >= insp_x + 196.0
                    && mouse.x <= insp_x + 282.0
                {
                    let old_col2 = self.registry.get::<Collider2D>(e).copied();
                    if let Some(col) = self.registry.get_mut::<Collider2D>(e) {
                        col.is_sensor = !col.is_sensor;
                        let sensor_now = col.is_sensor;
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: format!(
                                "Dugum #{}:{} carpisma modu: {}",
                                e.index(),
                                e.generation(),
                                if sensor_now { "Sensor" } else { "Kati" }
                            ),
                        });
                        self.push_collider_undo(e, old_col2);
                        self.audio.play("sfx_blip");
                        return;
                    }
                }

                // Script Butonlarına Tıklama (Rotator, Patrol, Pulse, none)
                let c4_y = c3_y + 60.0;
                let c4_h = 66.0;
                if mouse.y >= c4_y + 40.0 && mouse.y <= c4_y + 60.0 {
                    let scr_opts = [
                        ("Rotator", "RotatorScript"),
                        ("Patrol", "PatrolScript"),
                        ("Pulse", "PulseScript"),
                        ("none", "none"),
                    ];
                    let mut so_x = insp_x + 14.0;
                    for (_lbl, key) in scr_opts {
                        if mouse.x >= so_x && mouse.x <= so_x + 54.0 {
                            let old_scr = self.registry.get::<ScriptComponent>(e).cloned();
                            if key == "none" {
                                self.registry.remove::<ScriptComponent>(e);
                                self.push_script_undo(e, old_scr);
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} betigi kaldirildi.",
                                        e.index(),
                                        e.generation()
                                    ),
                                });
                            } else {
                                let comp = match key {
                                    "RotatorScript" => ScriptComponent::new_rotator(1.5),
                                    "PatrolScript" => {
                                        ScriptComponent::new_patrol(0, 80.0, 240.0, 75.0)
                                    }
                                    "PulseScript" => ScriptComponent::new_pulse(20.0, 44.0, 3.0),
                                    _ => ScriptComponent::new_rotator(1.0),
                                };
                                self.registry.emplace(e, comp);
                                self.push_script_undo(e, old_scr);
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} icin res://{} betigi baglandi.",
                                        e.index(),
                                        e.generation(),
                                        key
                                    ),
                                });
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        so_x += 58.0;
                    }
                }

                cur_insp_y = c4_y + c4_h + 8.0;

                // Velocity 2D Butonları ([-] / [+])
                let has_vel = self.registry.has::<Velocity2D>(e);
                if has_vel {
                    let v_y = cur_insp_y;
                    if mouse.y >= v_y + 24.0 && mouse.y <= v_y + 40.0 {
                        // Vx [-]
                        if mouse.x >= insp_x + 88.0 && mouse.x <= insp_x + 114.0 {
                            if let Some(v) = self.registry.get_mut::<Velocity2D>(e) {
                                v.vel.x -= 10.0;
                            }
                            return;
                        }
                        // Vx [+]
                        if mouse.x >= insp_x + 118.0 && mouse.x <= insp_x + 144.0 {
                            if let Some(v) = self.registry.get_mut::<Velocity2D>(e) {
                                v.vel.x += 10.0;
                            }
                            return;
                        }
                        // Vy [-]
                        if mouse.x >= insp_x + 228.0 && mouse.x <= insp_x + 254.0 {
                            if let Some(v) = self.registry.get_mut::<Velocity2D>(e) {
                                v.vel.y -= 10.0;
                            }
                            return;
                        }
                        // Vy [+]
                        if mouse.x >= insp_x + 258.0 && mouse.x <= insp_x + 284.0 {
                            if let Some(v) = self.registry.get_mut::<Velocity2D>(e) {
                                v.vel.y += 10.0;
                            }
                            return;
                        }
                    }
                    cur_insp_y += 52.0 + 8.0;
                }

                // Animation 2D Butonları ([Oynat/Durdur], FPS [-]/[+])
                let has_anim = self.registry.has::<Animation2DComponent>(e);
                if has_anim {
                    let a_y = cur_insp_y;
                    if mouse.y >= a_y + 24.0 && mouse.y <= a_y + 42.0 {
                        // [Oynat/Durdur]
                        if mouse.x >= insp_x + 130.0 && mouse.x <= insp_x + 186.0 {
                            if let Some(anim) = self.registry.get_mut::<Animation2DComponent>(e) {
                                if anim.is_playing {
                                    anim.pause();
                                } else {
                                    anim.resume();
                                }
                            }
                            self.audio.play("sfx_blip");
                            return;
                        }
                        // FPS [-]
                        if mouse.x >= insp_x + 240.0 && mouse.x <= insp_x + 262.0 {
                            if let Some(anim) = self.registry.get_mut::<Animation2DComponent>(e) {
                                let cur_clip = anim.current_clip.clone();
                                if let Some(clip) = anim.clips.get_mut(&cur_clip) {
                                    clip.fps = (clip.fps - 1.0).max(1.0);
                                }
                            }
                            return;
                        }
                        // FPS [+]
                        if mouse.x >= insp_x + 266.0 && mouse.x <= insp_x + 288.0 {
                            if let Some(anim) = self.registry.get_mut::<Animation2DComponent>(e) {
                                let cur_clip = anim.current_clip.clone();
                                if let Some(clip) = anim.clips.get_mut(&cur_clip) {
                                    clip.fps = (clip.fps + 1.0).min(60.0);
                                }
                            }
                            return;
                        }
                    }
                    cur_insp_y += 56.0 + 8.0;
                }

                // Tilemap 2D Butonları (Temizle, Fırça, Silgi, Karo Paleti)
                let has_tilemap = self.registry.has::<Tilemap2DComponent>(e);
                if has_tilemap {
                    let tm_y = cur_insp_y;
                    // [Haritayi Temizle]
                    if mouse.y >= tm_y + 23.0
                        && mouse.y <= tm_y + 41.0
                        && mouse.x >= insp_x + right_w - 96.0
                        && mouse.x <= insp_x + right_w - 16.0
                    {
                        if let Some(map) = self.registry.get_mut::<Tilemap2DComponent>(e) {
                            map.clear();
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} harita karolari temizlendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                        }
                        self.editor.is_scene_dirty = true;
                        self.audio.play("sfx_blip");
                        return;
                    }

                    // [Firca] Modu Butonu
                    if mouse.y >= tm_y + 48.0
                        && mouse.y <= tm_y + 68.0
                        && mouse.x >= insp_x + 14.0
                        && mouse.x <= insp_x + 84.0
                    {
                        self.editor.active_tool = ViewportTool::TileBrush;
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Tilemap Firca Modu Aktif.".to_string(),
                        });
                        self.audio.play("sfx_blip");
                        return;
                    }

                    // [Silgi] Modu Butonu
                    if mouse.y >= tm_y + 48.0
                        && mouse.y <= tm_y + 68.0
                        && mouse.x >= insp_x + 90.0
                        && mouse.x <= insp_x + 160.0
                    {
                        self.editor.active_tool = ViewportTool::TileEraser;
                        self.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Tilemap Silgi Modu Aktif.".to_string(),
                        });
                        self.audio.play("sfx_blip");
                        return;
                    }

                    // Karo Paleti Butonları (Çimen, Taş, Metal, Lav, Su)
                    if mouse.y >= tm_y + 74.0 && mouse.y <= tm_y + 96.0 {
                        let tile_palette = [
                            TileType::Grass,
                            TileType::Stone,
                            TileType::Metal,
                            TileType::Lava,
                            TileType::Water,
                        ];
                        let mut pal_x = insp_x + 14.0;
                        for tt in tile_palette {
                            if mouse.x >= pal_x && mouse.x <= pal_x + 48.0 {
                                self.editor.active_tile_type = tt;
                                self.editor.active_tool = ViewportTool::TileBrush;
                                self.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!("Firca Karosu Secildi: {:?}", tt),
                                });
                                self.audio.play("sfx_blip");
                                return;
                            }
                            pal_x += 52.0;
                        }
                    }

                    cur_insp_y += 104.0 + 8.0;
                }

                // [+ Add Component] Butonu
                let add_btn_y = cur_insp_y;
                if mouse.x >= insp_x + 8.0
                    && mouse.x <= insp_x + right_w - 8.0
                    && mouse.y >= add_btn_y
                    && mouse.y <= add_btn_y + 24.0
                {
                    self.editor.show_add_component_popup = !self.editor.show_add_component_popup;
                    return;
                }

                // Açılır Popup Menüsü Tıklamaları
                if self.editor.show_add_component_popup {
                    let pop_y = add_btn_y + 28.0;
                    if mouse.x >= insp_x + 8.0 && mouse.x <= insp_x + right_w - 8.0 {
                        if mouse.y >= pop_y + 4.0 && mouse.y <= pop_y + 24.0 {
                            self.registry.emplace(
                                e,
                                Collider2D {
                                    is_dynamic: true,
                                    is_sensor: false,
                                },
                            );
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Collider2D eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            self.editor.show_add_component_popup = false;
                            self.audio.play("sfx_blip");
                        } else if mouse.y >= pop_y + 26.0 && mouse.y <= pop_y + 46.0 {
                            self.registry.emplace(
                                e,
                                Velocity2D {
                                    vel: Vec2::new(50.0, 50.0),
                                },
                            );
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Velocity2D eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            self.editor.show_add_component_popup = false;
                            self.audio.play("sfx_blip");
                        } else if mouse.y >= pop_y + 48.0 && mouse.y <= pop_y + 68.0 {
                            self.registry.emplace(e, ScriptComponent::new_rotator(2.0));
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin ScriptComponent (Rotator) eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            self.editor.show_add_component_popup = false;
                            self.audio.play("sfx_blip");
                        } else if mouse.y >= pop_y + 70.0 && mouse.y <= pop_y + 90.0 {
                            self.registry.emplace(
                                e,
                                Text2DComponent {
                                    text: format!("Label_{}", e.index()),
                                    scale: 1.4,
                                    color: [240, 240, 240, 255],
                                },
                            );
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Text2DComponent eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            self.editor.show_add_component_popup = false;
                            self.audio.play("sfx_blip");
                        } else if mouse.y >= pop_y + 92.0 && mouse.y <= pop_y + 112.0 {
                            let clip = AnimationClip::new(
                                "idle",
                                vec!["player_ship".to_string(), "energy_orb".to_string()],
                                4.0,
                                true,
                            );
                            self.registry
                                .emplace(e, Animation2DComponent::new().with_clip(clip));
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Animation2DComponent eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            self.editor.show_add_component_popup = false;
                            self.audio.play("sfx_blip");
                        } else if mouse.y >= pop_y + 114.0 && mouse.y <= pop_y + 134.0 {
                            self.registry.emplace(e, Tilemap2DComponent::new(32.0));
                            self.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Tilemap2DComponent eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            self.editor.show_add_component_popup = false;
                            self.audio.play("sfx_blip");
                        }
                    }
                }
            }
        }
    }
}
