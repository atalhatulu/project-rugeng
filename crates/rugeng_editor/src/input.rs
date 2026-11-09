use crate::types::*;
use crate::App;
use glam::Vec2;
use rugeng_core::{Entity, Transform2D, ZIndex2D};

impl App {
    pub(crate) fn handle_mouse_click(
        &mut self,
        mouse: Vec2,
        screen_w: f32,
        screen_h: f32,
        double_click: bool,
    ) {
        let menu_h = 30.0;
        let left_w = self.editor.dock_left_w;
        let right_w = self.editor.dock_right_w;
        let bottom_h = self.editor.dock_bottom_h;

        let layout =
            crate::ui::DockLayout::with_custom_sizes(screen_w, screen_h, left_w, right_w, bottom_h);

        // Splitter (Panel Ayırıcı) Tutamaçlarına Tıklama Kontrolü
        if layout.is_over_left_splitter(mouse) {
            self.editor.drag_mode = DragMode::SplitterLeft {
                start_left_w: left_w,
                start_mouse_x: mouse.x,
            };
            return;
        }
        if layout.is_over_right_splitter(mouse) {
            self.editor.drag_mode = DragMode::SplitterRight {
                start_right_w: right_w,
                start_mouse_x: mouse.x,
            };
            return;
        }
        if layout.is_over_bottom_splitter(mouse) {
            self.editor.drag_mode = DragMode::SplitterBottom {
                start_bottom_h: bottom_h,
                start_mouse_y: mouse.y,
            };
            return;
        }

        // Eğer Node Library Dialog açıksa:
        if self.editor.show_node_library_dialog {
            let dlg_w = 320.0;
            let dlg_h = 360.0;
            let dlg_x = (screen_w - dlg_w) * 0.5;
            let dlg_y = (screen_h - dlg_h) * 0.5;

            // Kapatma butonu [X]
            if mouse.x >= dlg_x + dlg_w - 32.0
                && mouse.x <= dlg_x + dlg_w - 8.0
                && mouse.y >= dlg_y + 4.0
                && mouse.y <= dlg_y + 24.0
            {
                self.editor.show_node_library_dialog = false;
                return;
            }

            // Liste öğeleri
            let search_y = dlg_y + 34.0;
            let items: &[NodeLibraryItem] = &[
                NodeLibraryItem {
                    category: "2D",
                    name: "2D Gemi (PlayerShip)",
                    preset_id: "ship",
                    is_3d: false,
                },
                NodeLibraryItem {
                    category: "2D",
                    name: "2D Bocek (EnemyBug)",
                    preset_id: "enemy",
                    is_3d: false,
                },
                NodeLibraryItem {
                    category: "2D",
                    name: "2D Kalp (Heart)",
                    preset_id: "heart",
                    is_3d: false,
                },
                NodeLibraryItem {
                    category: "2D",
                    name: "2D Enerji (Orb)",
                    preset_id: "orb",
                    is_3d: false,
                },
                NodeLibraryItem {
                    category: "2D",
                    name: "2D Karo Haritasi (Tilemap)",
                    preset_id: "tilemap",
                    is_3d: false,
                },
                NodeLibraryItem {
                    category: "2D",
                    name: "2D Metin (Label/Text)",
                    preset_id: "text",
                    is_3d: false,
                },
                NodeLibraryItem {
                    category: "2D",
                    name: "2D Bos Dugum (Node2D)",
                    preset_id: "empty",
                    is_3d: false,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Kup (Mesh3D Cube)",
                    preset_id: "cube_3d",
                    is_3d: true,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Kure (Sphere)",
                    preset_id: "sphere_3d",
                    is_3d: true,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Duzlem (Plane)",
                    preset_id: "plane_3d",
                    is_3d: true,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Silindir (Cylinder)",
                    preset_id: "cylinder_3d",
                    is_3d: true,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Koni (Cone)",
                    preset_id: "cone_3d",
                    is_3d: true,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Piramit (Pyramid)",
                    preset_id: "pyramid_3d",
                    is_3d: true,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Kapsul (Capsule)",
                    preset_id: "capsule_3d",
                    is_3d: true,
                },
                NodeLibraryItem {
                    category: "3D",
                    name: "3D Yonlu Isik (Sun)",
                    preset_id: "directional_light_3d",
                    is_3d: true,
                },
            ];

            let query = self.editor.node_library_search.to_lowercase();
            let mut it_y = search_y + 28.0;
            for item in items {
                if !query.is_empty()
                    && !item.name.to_lowercase().contains(&query)
                    && !item.category.to_lowercase().contains(&query)
                {
                    continue;
                }
                if it_y + 20.0 > dlg_y + dlg_h - 10.0 {
                    break;
                }

                if mouse.x >= dlg_x + 12.0
                    && mouse.x <= dlg_x + dlg_w - 12.0
                    && mouse.y >= it_y
                    && mouse.y <= it_y + 20.0
                {
                    self.spawn_preset_entity(item.preset_id);
                    self.editor.show_node_library_dialog = false;
                    return;
                }
                it_y += 24.0;
            }

            // Diyalog dışına tıklandıysa kapat
            if mouse.x < dlg_x
                || mouse.x > dlg_x + dlg_w
                || mouse.y < dlg_y
                || mouse.y > dlg_y + dlg_h
            {
                self.editor.show_node_library_dialog = false;
            }
            return;
        }

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
                        "save_prefab" => {
                            if let Some(e) = ctx_menu.target_entity {
                                let name = self.display_name_of(e, false);
                                let path = format!("{}.prefab", name);
                                let _ = self.save_selected_as_prefab(&path);
                            }
                        }
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
                            if let Some(pos) = ctx_menu.spawn_world_pos {
                                self.spawn_preset_entity_at(p, pos);
                            } else {
                                self.spawn_preset_entity(p);
                            }
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
                        "add_gltf_sample" => {
                            let path = "assets/models/sample.glb";
                            if std::path::Path::new(path).exists() {
                                let _ = self.import_gltf_scene(path);
                            } else {
                                self.spawn_preset_entity("gltf_sample");
                            }
                        }
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

        // Modüler Panel Delegasyonu (trait Panel)
        use crate::ui::{
            BottomShelfPanel, InspectorPanel, Panel, SceneTreePanel, TopBarPanel, ViewportPanel,
        };

        let mut top_bar = TopBarPanel;
        if top_bar.handle_click(self, mouse, &layout, double_click) {
            return;
        }

        let mut scene_tree = SceneTreePanel;
        if scene_tree.handle_click(self, mouse, &layout, double_click) {
            return;
        }

        let mut bottom_shelf = BottomShelfPanel;
        if bottom_shelf.handle_click(self, mouse, &layout, double_click) {
            return;
        }

        let mut inspector = InspectorPanel;
        if inspector.handle_click(self, mouse, &layout, double_click) {
            return;
        }

        let mut viewport = ViewportPanel;
        let _ = viewport.handle_click(self, mouse, &layout, double_click);
    }
}
