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
    pub(crate) fn render_scene_tree_and_filesystem(
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
            instances,
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
            instances,
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
        font::draw_text(instances, "+", 14.0, filter_y + 4.0, 1.5, godot_green);

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
        font::draw_text(instances, "@", 42.0, filter_y + 4.0, 1.4, godot_blue);

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
            instances,
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
            instances,
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
            font::draw_text(instances, "+ Kup 3D", 28.0, b1_y + 4.0, 1.2, godot_yellow);

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
            font::draw_text(instances, "+ Kure 3D", 146.0, b1_y + 4.0, 1.2, godot_yellow);

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
                instances,
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
                instances,
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
                instances,
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
                instances,
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
            font::draw_text(instances, "+ Duzlem", 24.0, b4_y + 4.0, 1.2, text_norm);

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
                instances,
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
                instances,
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
            font::draw_text(instances, "+ Gemi", 28.0, b1_y + 4.0, 1.2, godot_blue);

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
            font::draw_text(instances, "+ Bocek", 152.0, b1_y + 4.0, 1.2, godot_red);

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
            font::draw_text(instances, "+ Kure", 28.0, b2_y + 4.0, 1.2, godot_yellow);

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
                instances,
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
                instances,
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
                instances,
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
                instances,
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
                instances,
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
        font::draw_text(instances, "Kopyala", 16.0, act_bar_y + 4.0, 1.1, text_norm);

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
            instances,
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
            instances,
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
                instances,
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
            font::draw_text(instances, badge, left_w - 38.0, row_y + 4.0, 1.1, icon_col);
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
            instances,
            "FileSystem",
            12.0,
            left_bot_y + 8.0,
            1.4,
            text_norm,
        );

        // FileSystem Aksiyon Butonları: [+ Klasor] ve [+ Sahne]
        let btn_klasor_x = left_w - 146.0;
        instances.push(QuadInstance::new(
            [btn_klasor_x, left_bot_y + 4.0],
            [68.0, 18.0],
            theme.btn_normal,
            0,
        ));
        instances.push(QuadInstance::new(
            [btn_klasor_x, left_bot_y + 4.0],
            [68.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            instances,
            "+ Klasor",
            btn_klasor_x + 8.0,
            left_bot_y + 7.0,
            1.15,
            godot_yellow,
        );

        let btn_sahne_x = left_w - 72.0;
        instances.push(QuadInstance::new(
            [btn_sahne_x, left_bot_y + 4.0],
            [66.0, 18.0],
            theme.btn_normal,
            0,
        ));
        instances.push(QuadInstance::new(
            [btn_sahne_x, left_bot_y + 4.0],
            [66.0, 1.0],
            theme.border_subtle,
            0,
        ));
        font::draw_text(
            instances,
            "+ Sahne",
            btn_sahne_x + 8.0,
            left_bot_y + 7.0,
            1.15,
            godot_green,
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
        font::draw_text(instances, "<  res://", 10.0, fs_bar_y + 4.0, 1.3, text_dim);
        let path_str = self.editor.current_path.to_string_lossy();
        let display_path = if path_str == "." {
            "root".to_string()
        } else {
            path_str.to_string()
        };
        font::draw_text(
            instances,
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
            instances,
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
            font::draw_text(instances, "[..]", 14.0, f_y, 1.2, godot_yellow);
            font::draw_text(instances, ".. (Ust Dizin)", 46.0, f_y, 1.3, text_norm);
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

            font::draw_text(instances, ftag, 14.0, f_y, 1.1, fcol);
            let d_name = if item.name.len() > 20 {
                &item.name[..20]
            } else {
                &item.name
            };
            font::draw_text(
                instances,
                d_name,
                54.0,
                f_y,
                1.3,
                if is_f_sel { text_norm } else { text_dim },
            );
            f_y += 20.0;
        }

        // =========================================================================
        // 4. DÜĞÜM / VARLIK KÜTÜPHANESİ POPUP (NODE LIBRARY DIALOG)
        // =========================================================================
        if self.editor.show_node_library_dialog {
            let dlg_w = 320.0;
            let dlg_h = 360.0;
            let dlg_x = (screen_w - dlg_w) * 0.5;
            let dlg_y = (screen_h - dlg_h) * 0.5;

            // Arka plan karartması
            instances.push(QuadInstance::new(
                [0.0, 0.0],
                [screen_w, screen_h],
                [10, 12, 16, 180],
                0,
            ));

            // Diyalog kutusu ve gölge
            instances.push(QuadInstance::new(
                [dlg_x + 4.0, dlg_y + 4.0],
                [dlg_w, dlg_h],
                [4, 6, 8, 220],
                0,
            ));
            instances.push(QuadInstance::new(
                [dlg_x, dlg_y],
                [dlg_w, dlg_h],
                theme.bg_panel,
                0,
            ));
            instances.push(QuadInstance::new(
                [dlg_x, dlg_y],
                [dlg_w, 2.0],
                godot_green,
                0,
            ));
            instances.push(QuadInstance::new(
                [dlg_x, dlg_y + dlg_h - 1.0],
                [dlg_w, 1.0],
                border_col,
                0,
            ));
            instances.push(QuadInstance::new(
                [dlg_x, dlg_y],
                [1.0, dlg_h],
                border_col,
                0,
            ));
            instances.push(QuadInstance::new(
                [dlg_x + dlg_w - 1.0, dlg_y],
                [1.0, dlg_h],
                border_col,
                0,
            ));

            // Başlık
            instances.push(QuadInstance::new(
                [dlg_x, dlg_y],
                [dlg_w, 28.0],
                bg_header_act,
                0,
            ));
            font::draw_text(
                instances,
                "Yeni Dugum Olustur (Node Library)",
                dlg_x + 12.0,
                dlg_y + 8.0,
                1.3,
                text_norm,
            );
            // Kapat [X]
            font::draw_text(
                instances,
                "[X]",
                dlg_x + dlg_w - 28.0,
                dlg_y + 8.0,
                1.3,
                godot_red,
            );

            // Arama Kutusu
            let search_y = dlg_y + 34.0;
            instances.push(QuadInstance::new(
                [dlg_x + 12.0, search_y],
                [dlg_w - 24.0, 22.0],
                theme.input_bg,
                0,
            ));
            instances.push(QuadInstance::new(
                [dlg_x + 12.0, search_y],
                [dlg_w - 24.0, 1.0],
                theme.border_subtle,
                0,
            ));
            let s_text = if self.editor.node_library_search.is_empty() {
                "Dugum Ara (Gemi, Kup, Harita, Isik)..."
            } else {
                &self.editor.node_library_search
            };
            font::draw_text(
                instances,
                s_text,
                dlg_x + 18.0,
                search_y + 5.0,
                1.2,
                if self.editor.node_library_search.is_empty() {
                    text_dim
                } else {
                    text_norm
                },
            );

            // Kütüphane Öğeleri Listesi
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

                instances.push(QuadInstance::new(
                    [dlg_x + 12.0, it_y],
                    [dlg_w - 24.0, 20.0],
                    theme.btn_normal,
                    0,
                ));
                let tag_col = if item.is_3d { godot_yellow } else { godot_blue };
                instances.push(QuadInstance::new(
                    [dlg_x + 16.0, it_y + 6.0],
                    [8.0, 8.0],
                    tag_col,
                    0,
                ));
                font::draw_text(
                    instances,
                    item.name,
                    dlg_x + 28.0,
                    it_y + 4.0,
                    1.2,
                    text_norm,
                );
                it_y += 24.0;
            }
        }
    }
}
