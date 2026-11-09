#![allow(unused_imports)]
#![allow(unused_variables, dead_code)]
use crate::theme::*;
use crate::types::*;
use crate::ui::dock_layout::DockLayout;
use crate::App;
use glam::Vec2;
use rugeng_core::{
    interpolate_particle_color, Animation2DComponent, AnimationClip, Collider2D, Entity,
    Mesh3DComponent, NodeName, Parent, ParticleEmitterComponent, ScriptComponent, SpriteColor,
    SpriteTexture, Text2DComponent, TileType, Tilemap2DComponent, Transform2D, Transform3D,
    Velocity2D, ZIndex2D,
};
use rugeng_render::font;
use rugeng_render::pipeline_2d::QuadInstance;

impl App {
    /// Inspector sayısal alan çizim yardımcısı (Odak, geçersizlik ve imleç desteği ile)
    pub(crate) fn render_numeric_field(
        &self,
        instances: &mut Vec<QuadInstance>,
        rect: [f32; 4], // [x, y, w, h]
        label: &str,
        current_val: f32,
        entity: Entity,
        field: NumericFieldId,
    ) {
        let theme = self.editor.theme;
        let is_focused = self
            .editor
            .numeric_edit
            .as_ref()
            .map(|ne| ne.entity == entity && ne.field == field)
            .unwrap_or(false);

        // Arka plan
        instances.push(QuadInstance::new(
            [rect[0], rect[1]],
            [rect[2], rect[3]],
            if is_focused {
                theme.input_bg
            } else {
                theme.card_bg
            },
            0,
        ));

        // Kenarlık: Odaklıysa mavi veya kırmızı (geçersizse), odaklı değilse ince kenarlık
        let (border_col, border_w) = if is_focused {
            let is_inv = self
                .editor
                .numeric_edit
                .as_ref()
                .map(|ne| ne.is_invalid)
                .unwrap_or(false);
            if is_inv {
                (theme.danger, 1.5)
            } else {
                (theme.accent, 1.5)
            }
        } else {
            (theme.border_subtle, 1.0)
        };

        instances.push(QuadInstance::new(
            [rect[0], rect[1]],
            [rect[2], border_w],
            border_col,
            0,
        ));
        instances.push(QuadInstance::new(
            [rect[0], rect[1] + rect[3] - border_w],
            [rect[2], border_w],
            border_col,
            0,
        ));
        instances.push(QuadInstance::new(
            [rect[0], rect[1]],
            [border_w, rect[3]],
            border_col,
            0,
        ));
        instances.push(QuadInstance::new(
            [rect[0] + rect[2] - border_w, rect[1]],
            [border_w, rect[3]],
            border_col,
            0,
        ));

        // Metin içeriği ve imleç
        let text_col = if is_focused {
            let is_inv = self
                .editor
                .numeric_edit
                .as_ref()
                .map(|ne| ne.is_invalid)
                .unwrap_or(false);
            if is_inv {
                theme.danger
            } else {
                theme.text_primary
            }
        } else {
            theme.text_primary
        };

        let display_str = if is_focused {
            let ne = self.editor.numeric_edit.as_ref().unwrap();
            let cs: Vec<char> = ne.buffer.chars().collect();
            let pos = ne.caret.min(cs.len());
            let head: String = cs[..pos].iter().collect();
            let tail: String = cs[pos..].iter().collect();
            format!("{}▏{}", head, tail)
        } else {
            format!("{:.1}", current_val)
        };

        font::draw_text(
            instances,
            &display_str,
            rect[0] + 4.0,
            rect[1] + (rect[3] - 12.0) * 0.5,
            1.15,
            text_col,
        );
    }
    pub(crate) fn render_inspector(&self, instances: &mut Vec<QuadInstance>, layout: &DockLayout) {
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
        // 5. SAĞ SÜTUN: TAM BOY DENETÇİ (INSPECTOR DOCK)
        // =========================================================================
        let insp_x = layout.insp_x;
        let insp_h = layout.insp_h;
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
            instances,
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
            instances,
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
                instances,
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
                    instances,
                    "v Transform 3D",
                    insp_x + 14.0,
                    c3d_y + 4.0,
                    1.3,
                    [224, 169, 66, 255],
                );

                // Position X/Y/Z Numeric Fields
                font::draw_text(
                    instances,
                    "Pos:",
                    insp_x + 14.0,
                    c3d_y + 26.0,
                    1.15,
                    text_dim,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 44.0, c3d_y + 24.0, 34.0, 16.0],
                    "X",
                    t3.translation.x,
                    e,
                    NumericFieldId::Pos3X,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 82.0, c3d_y + 24.0, 34.0, 16.0],
                    "Y",
                    t3.translation.y,
                    e,
                    NumericFieldId::Pos3Y,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 120.0, c3d_y + 24.0, 34.0, 16.0],
                    "Z",
                    t3.translation.z,
                    e,
                    NumericFieldId::Pos3Z,
                );

                // X - / +
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c3d_y + 24.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
                    "Z+",
                    insp_x + 192.0,
                    c3d_y + 46.0,
                    1.0,
                    godot_blue,
                );

                // Rotation Euler Numeric Fields
                let (pitch, yaw, roll) = t3.rotation.to_euler(glam::EulerRot::YXZ);
                font::draw_text(
                    instances,
                    "Rot:",
                    insp_x + 14.0,
                    c3d_y + 68.0,
                    1.15,
                    text_dim,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 44.0, c3d_y + 66.0, 34.0, 16.0],
                    "P",
                    pitch.to_degrees(),
                    e,
                    NumericFieldId::Rot3Pitch,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 82.0, c3d_y + 66.0, 34.0, 16.0],
                    "Y",
                    yaw.to_degrees(),
                    e,
                    NumericFieldId::Rot3Yaw,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 120.0, c3d_y + 66.0, 34.0, 16.0],
                    "R",
                    roll.to_degrees(),
                    e,
                    NumericFieldId::Rot3Roll,
                );

                // Pitch - / +
                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(instances, "P-", insp_x + 164.0, c3d_y + 68.0, 1.0, text_dim);
                instances.push(QuadInstance::new(
                    [insp_x + 188.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(instances, "P+", insp_x + 192.0, c3d_y + 68.0, 1.0, text_dim);
                // Yaw - / +
                instances.push(QuadInstance::new(
                    [insp_x + 216.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(instances, "Y-", insp_x + 220.0, c3d_y + 68.0, 1.0, text_dim);
                instances.push(QuadInstance::new(
                    [insp_x + 244.0, c3d_y + 66.0],
                    [24.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(instances, "Y+", insp_x + 248.0, c3d_y + 68.0, 1.0, text_dim);

                // Scale X/Y/Z Numeric Fields
                font::draw_text(
                    instances,
                    "Scl:",
                    insp_x + 14.0,
                    c3d_y + 90.0,
                    1.15,
                    text_dim,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 44.0, c3d_y + 88.0, 34.0, 16.0],
                    "X",
                    t3.scale.x,
                    e,
                    NumericFieldId::Scale3X,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 82.0, c3d_y + 88.0, 34.0, 16.0],
                    "Y",
                    t3.scale.y,
                    e,
                    NumericFieldId::Scale3Y,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 120.0, c3d_y + 88.0, 34.0, 16.0],
                    "Z",
                    t3.scale.z,
                    e,
                    NumericFieldId::Scale3Z,
                );

                instances.push(QuadInstance::new(
                    [insp_x + 160.0, c3d_y + 88.0],
                    [32.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(instances, "S-", insp_x + 168.0, c3d_y + 90.0, 1.0, text_dim);
                instances.push(QuadInstance::new(
                    [insp_x + 196.0, c3d_y + 88.0],
                    [32.0, 15.0],
                    theme.btn_normal,
                    0,
                ));
                font::draw_text(instances, "S+", insp_x + 204.0, c3d_y + 90.0, 1.0, text_dim);

                // Mesh 3D Bilgisi & PBR
                if let Some(m3) = self.registry.get::<Mesh3DComponent>(e) {
                    font::draw_text(
                        instances,
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
                        font::draw_text(instances, lbl, mb_x + 6.0, c3d_y + 132.0, 1.0, text_norm);
                        mb_x += 52.0;
                    }
                    cur_insp_y += c3d_h + 8.0;
                }

                // BİLEŞEN: [ Directional Light & Shadow ]
                let sl_h = 76.0;
                let sl_y = cur_insp_y;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, sl_y],
                    [right_w - 16.0, sl_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, sl_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, sl_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    instances,
                    "v Directional Light & Shadow (PCF)",
                    insp_x + 14.0,
                    sl_y + 4.0,
                    1.25,
                    [255, 215, 0, 255],
                );
                font::draw_text(
                    instances,
                    &format!(
                        "Dir: ({:.1}, {:.1}, {:.1}) | Res: 2048x2048",
                        self.sun.direction.x, self.sun.direction.y, self.sun.direction.z
                    ),
                    insp_x + 14.0,
                    sl_y + 26.0,
                    1.1,
                    text_norm,
                );
                font::draw_text(
                    instances,
                    "Filter: PCF 3x3 | Slope Bias: 0.0025 | D32_SFLOAT",
                    insp_x + 14.0,
                    sl_y + 48.0,
                    1.05,
                    godot_blue,
                );
                cur_insp_y += sl_h + 8.0;

                // BİLEŞEN: [ Point Light ]
                if let Some(pl) = self.registry.get::<rugeng_core::PointLight>(e) {
                    let pl_h = 72.0;
                    let pl_y = cur_insp_y;
                    instances.push(QuadInstance::new(
                        [insp_x + 8.0, pl_y],
                        [right_w - 16.0, pl_h],
                        theme.card_bg,
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [insp_x + 8.0, pl_y],
                        [right_w - 16.0, 20.0],
                        theme.card_header,
                        0,
                    ));
                    instances.push(QuadInstance::new(
                        [insp_x + 8.0, pl_y],
                        [right_w - 16.0, 1.0],
                        theme.border_subtle,
                        0,
                    ));
                    font::draw_text(
                        instances,
                        "v Point Light (Forward SSBO)",
                        insp_x + 14.0,
                        pl_y + 4.0,
                        1.25,
                        [255, 180, 50, 255],
                    );
                    font::draw_text(
                        instances,
                        &format!(
                            "Col: ({:.2}, {:.2}, {:.2}) | Int: {:.1}",
                            pl.color[0], pl.color[1], pl.color[2], pl.intensity
                        ),
                        insp_x + 14.0,
                        pl_y + 26.0,
                        1.1,
                        text_norm,
                    );
                    font::draw_text(
                        instances,
                        &format!("Radius: {:.1} units | Atten: Inverse-Square", pl.radius),
                        insp_x + 14.0,
                        pl_y + 46.0,
                        1.05,
                        godot_blue,
                    );
                    cur_insp_y += pl_h + 8.0;
                }

                // BİLEŞEN: [ 3D Collider & Ray Picking ]
                let col3d_h = 70.0;
                let col3d_y = cur_insp_y;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, col3d_y],
                    [right_w - 16.0, col3d_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, col3d_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, col3d_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    instances,
                    "v 3D Collider & Ray Picking",
                    insp_x + 14.0,
                    col3d_y + 4.0,
                    1.25,
                    godot_green,
                );
                font::draw_text(
                    instances,
                    "Broadphase: SpatialHashGrid3D | Raycast: Slab/Analytic",
                    insp_x + 14.0,
                    col3d_y + 26.0,
                    1.05,
                    text_norm,
                );
                font::draw_text(
                    instances,
                    "Collider Visualizer: ON | Solver: MTV Deterministic",
                    insp_x + 14.0,
                    col3d_y + 46.0,
                    1.05,
                    [150, 210, 255, 255],
                );
                // BİLEŞEN: [ 3D İskelet Animasyonu ]
                let anim_h = 70.0;
                let anim_y = cur_insp_y;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, anim_y],
                    [right_w - 16.0, anim_h],
                    theme.card_bg,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, anim_y],
                    [right_w - 16.0, 20.0],
                    theme.card_header,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, anim_y],
                    [right_w - 16.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    instances,
                    "v Skeletal Animation (Skinning)",
                    insp_x + 14.0,
                    anim_y + 4.0,
                    1.25,
                    [255, 140, 200, 255],
                );
                font::draw_text(
                    instances,
                    "Bone Palette: 32 Joints | Vertex Skinning: CPU/GPU",
                    insp_x + 14.0,
                    anim_y + 26.0,
                    1.05,
                    text_norm,
                );
                font::draw_text(
                    instances,
                    "glTF Animation: TRS Channels | State: Playing (1.0x)",
                    insp_x + 14.0,
                    anim_y + 46.0,
                    1.05,
                    godot_blue,
                );
                cur_insp_y += anim_h + 8.0;
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
                    instances,
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
                    instances,
                    "Position X:",
                    insp_x + 14.0,
                    c1_y + 26.0,
                    1.3,
                    text_dim,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 90.0, c1_y + 24.0, 60.0, 16.0],
                    "X",
                    curr_pos.x,
                    e,
                    NumericFieldId::PosX,
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
                    instances,
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
                    instances,
                    "[+]",
                    insp_x + 204.0,
                    c1_y + 26.0,
                    1.2,
                    text_norm,
                );

                // Y Satırı
                font::draw_text(
                    instances,
                    "Position Y:",
                    insp_x + 14.0,
                    c1_y + 46.0,
                    1.3,
                    text_dim,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 90.0, c1_y + 44.0, 60.0, 16.0],
                    "Y",
                    curr_pos.y,
                    e,
                    NumericFieldId::PosY,
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
                    instances,
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
                    instances,
                    "[+]",
                    insp_x + 204.0,
                    c1_y + 46.0,
                    1.2,
                    text_norm,
                );

                // Rotation Satırı
                let rot_deg = curr_rot.to_degrees();
                font::draw_text(
                    instances,
                    "Rotation:",
                    insp_x + 14.0,
                    c1_y + 68.0,
                    1.3,
                    text_dim,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 90.0, c1_y + 66.0, 60.0, 16.0],
                    "Rot",
                    rot_deg,
                    e,
                    NumericFieldId::Rot,
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
                font::draw_text(instances, "R-", insp_x + 168.0, c1_y + 68.0, 1.2, text_norm);
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
                font::draw_text(instances, "R+", insp_x + 204.0, c1_y + 68.0, 1.2, text_norm);

                // Scale Satırı
                font::draw_text(
                    instances,
                    "Scale W/H:",
                    insp_x + 14.0,
                    c1_y + 90.0,
                    1.3,
                    text_dim,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 90.0, c1_y + 88.0, 30.0, 16.0],
                    "W",
                    curr_size.x,
                    e,
                    NumericFieldId::SizeX,
                );
                self.render_numeric_field(
                    instances,
                    [insp_x + 124.0, c1_y + 88.0, 30.0, 16.0],
                    "H",
                    curr_size.y,
                    e,
                    NumericFieldId::SizeY,
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
                font::draw_text(instances, "W-", insp_x + 168.0, c1_y + 90.0, 1.2, text_norm);
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
                font::draw_text(instances, "W+", insp_x + 204.0, c1_y + 90.0, 1.2, text_norm);

                // Z-Index (Katman Derinliği) Satırı
                font::draw_text(
                    instances,
                    "Z-Index:",
                    insp_x + 14.0,
                    c1_y + 112.0,
                    1.3,
                    text_dim,
                );
                font::draw_text(
                    instances,
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
                    instances,
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
                    instances,
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
                instances,
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
                instances,
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
                font::draw_text(instances, label, opt_x + 6.0, c2_y + 42.0, 1.1, text_norm);
                opt_x += 42.0;
            }

            let spr_col = self
                .registry
                .get::<SpriteColor>(e)
                .map(|s| s.color)
                .unwrap_or([255, 255, 255, 255]);
            font::draw_text(
                instances,
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
                instances,
                "v CollisionShape 2D",
                insp_x + 14.0,
                c3_y + 4.0,
                1.3,
                godot_yellow,
            );

            let col_opt = self.registry.get::<Collider2D>(e).copied();
            if let Some(col) = col_opt {
                font::draw_text(
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                instances,
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
                instances,
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
                font::draw_text(instances, opt, so_x + 6.0, c4_y + 44.0, 1.1, text_norm);
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
                    instances,
                    "v Velocity 2D (Fizik)",
                    insp_x + 14.0,
                    cur_insp_y + 4.0,
                    1.3,
                    godot_yellow,
                );

                font::draw_text(
                    instances,
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
                    instances,
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
                    instances,
                    "[+]",
                    insp_x + 124.0,
                    cur_insp_y + 26.0,
                    1.1,
                    text_norm,
                );

                font::draw_text(
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
                    "v Tilemap 2D (Canli Karo Editoru)",
                    insp_x + 14.0,
                    cur_insp_y + 4.0,
                    1.3,
                    [52, 211, 153, 255],
                );

                font::draw_text(
                    instances,
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
                    instances,
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
                    instances,
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
                    instances,
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
                    font::draw_text(instances, lbl, pal_x + 6.0, cur_insp_y + 78.0, 1.1, txt_col);
                    pal_x += 52.0;
                }

                cur_insp_y += tm_h + 8.0;
            }

            // BİLEŞEN 8: Parçacık Yayıcı (ParticleEmitterComponent) Kartı
            let particle_opt = self.registry.get::<ParticleEmitterComponent>(e).cloned();
            if let Some(p) = particle_opt {
                let p_h = 76.0;
                instances.push(QuadInstance::new(
                    [insp_x + 8.0, cur_insp_y],
                    [right_w - 16.0, p_h],
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
                    instances,
                    "v Parçacık Yayıcı (Particle)",
                    insp_x + 14.0,
                    cur_insp_y + 4.0,
                    1.3,
                    godot_yellow,
                );

                // Emitting Durumu ve Toggle
                let emit_status = if p.is_active {
                    "[Aktif]"
                } else {
                    "[Durduruldu]"
                };
                let emit_col = if p.is_active { godot_green } else { godot_red };
                font::draw_text(
                    instances,
                    &format!("Durum: {} (Hiz: {:.0})", emit_status, p.speed_min),
                    insp_x + 14.0,
                    cur_insp_y + 26.0,
                    1.2,
                    emit_col,
                );

                // [Patlat / Burst] Butonu
                instances.push(QuadInstance::new(
                    [insp_x + 14.0, cur_insp_y + 48.0],
                    [110.0, 20.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 14.0, cur_insp_y + 48.0],
                    [110.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                font::draw_text(
                    instances,
                    "[Patlat (Burst)]",
                    insp_x + 20.0,
                    cur_insp_y + 52.0,
                    1.1,
                    text_norm,
                );

                // [Ac/Kapat] Butonu
                instances.push(QuadInstance::new(
                    [insp_x + 130.0, cur_insp_y + 48.0],
                    [110.0, 20.0],
                    theme.btn_normal,
                    0,
                ));
                instances.push(QuadInstance::new(
                    [insp_x + 130.0, cur_insp_y + 48.0],
                    [110.0, 1.0],
                    theme.border_subtle,
                    0,
                ));
                let toggle_txt = if p.is_active { "[Durdur]" } else { "[Baslat]" };
                font::draw_text(
                    instances,
                    toggle_txt,
                    insp_x + 146.0,
                    cur_insp_y + 52.0,
                    1.1,
                    text_norm,
                );

                cur_insp_y += p_h + 8.0;
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
                instances,
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
                    ("+ ParticleEmitter (Parcacik) Ekle", 6),
                    ("+ ZIndex2D (Katman Derinligi) Ekle", 7),
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
                        instances,
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
                instances,
                "Select a node to inspect.",
                insp_x + 16.0,
                insp_search_y + 36.0,
                1.3,
                text_dim,
            );
        }
    }
}

impl crate::ui::Panel for crate::ui::InspectorPanel {
    fn handle_click(
        &mut self,
        app: &mut App,
        mouse: Vec2,
        layout: &DockLayout,
        _double_click: bool,
    ) -> bool {
        let menu_h = layout.menu_h;
        let right_w = layout.right_w;
        let insp_x = layout.insp_x;

        if mouse.x >= insp_x && mouse.y > menu_h {
            if let Some(e) = app.editor.selected_entity {
                let insp_search_y = menu_h + 28.0;
                let card_y = insp_search_y + 28.0;

                let mut cur_insp_y = card_y + 30.0;
                // 3D Varlik Kontrolleri (Transform3D & Mesh3DComponent)
                if app.registry.has::<Transform3D>(e) {
                    let c3d_y = cur_insp_y;

                    // 3D Sayisal Alan Odaklanmasi (Position 3D: X, Y, Z)
                    if mouse.y >= c3d_y + 24.0 && mouse.y <= c3d_y + 40.0 {
                        if let Some(t3) = app.registry.get::<Transform3D>(e) {
                            if mouse.x >= insp_x + 44.0 && mouse.x <= insp_x + 78.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Pos3X,
                                    t3.translation.x,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            } else if mouse.x >= insp_x + 82.0 && mouse.x <= insp_x + 116.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Pos3Y,
                                    t3.translation.y,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            } else if mouse.x >= insp_x + 120.0 && mouse.x <= insp_x + 154.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Pos3Z,
                                    t3.translation.z,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            }
                        }
                    }

                    // 3D Sayisal Alan Odaklanmasi (Rotation 3D: Pitch, Yaw, Roll)
                    if mouse.y >= c3d_y + 66.0 && mouse.y <= c3d_y + 82.0 {
                        if let Some(t3) = app.registry.get::<Transform3D>(e) {
                            let (pitch, yaw, roll) = t3.rotation.to_euler(glam::EulerRot::YXZ);
                            if mouse.x >= insp_x + 44.0 && mouse.x <= insp_x + 78.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Rot3Pitch,
                                    pitch.to_degrees(),
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            } else if mouse.x >= insp_x + 82.0 && mouse.x <= insp_x + 116.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Rot3Yaw,
                                    yaw.to_degrees(),
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            } else if mouse.x >= insp_x + 120.0 && mouse.x <= insp_x + 154.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Rot3Roll,
                                    roll.to_degrees(),
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            }
                        }
                    }

                    // 3D Sayisal Alan Odaklanmasi (Scale 3D: X, Y, Z)
                    if mouse.y >= c3d_y + 88.0 && mouse.y <= c3d_y + 104.0 {
                        if let Some(t3) = app.registry.get::<Transform3D>(e) {
                            if mouse.x >= insp_x + 44.0 && mouse.x <= insp_x + 78.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Scale3X,
                                    t3.scale.x,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            } else if mouse.x >= insp_x + 82.0 && mouse.x <= insp_x + 116.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Scale3Y,
                                    t3.scale.y,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            } else if mouse.x >= insp_x + 120.0 && mouse.x <= insp_x + 154.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Scale3Z,
                                    t3.scale.z,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            }
                        }
                    }

                    // Pos X- / X+
                    if mouse.y >= c3d_y + 24.0 && mouse.y <= c3d_y + 39.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 184.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.translation.x -= 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 188.0 && mouse.x <= insp_x + 212.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.translation.x += 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        // Pos Y- / Y+
                        if mouse.x >= insp_x + 216.0 && mouse.x <= insp_x + 240.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.translation.y -= 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 244.0 && mouse.x <= insp_x + 268.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.translation.y += 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                    }
                    // Pos Z- / Z+
                    if mouse.y >= c3d_y + 44.0 && mouse.y <= c3d_y + 59.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 184.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.translation.z -= 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 188.0 && mouse.x <= insp_x + 212.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.translation.z += 0.5;
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                    }
                    // Pitch P- / P+, Yaw Y- / Y+
                    if mouse.y >= c3d_y + 66.0 && mouse.y <= c3d_y + 81.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 184.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_x(-0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 188.0 && mouse.x <= insp_x + 212.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_x(0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 216.0 && mouse.x <= insp_x + 240.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_y(-0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 244.0 && mouse.x <= insp_x + 268.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.rotation *= glam::Quat::from_rotation_y(0.1);
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                    }
                    // Scale S- / S+
                    if mouse.y >= c3d_y + 88.0 && mouse.y <= c3d_y + 103.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.scale =
                                    (t3.scale - glam::Vec3::splat(0.2)).max(glam::Vec3::splat(0.1));
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t3 = app.registry.get::<Transform3D>(e).copied();
                            if let Some(t3) = app.registry.get_mut::<Transform3D>(e) {
                                t3.scale += glam::Vec3::splat(0.2);
                            }
                            if let Some(old_t3) = old_t3 {
                                app.push_transform3d_undo(e, old_t3);
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                    }
                    // Mesh Secim Butonlari: Cube, Sphere, Plane
                    if mouse.y >= c3d_y + 130.0 && mouse.y <= c3d_y + 146.0 {
                        let mesh_btns = [
                            ("cube", insp_x + 14.0),
                            ("sphere", insp_x + 66.0),
                            ("plane", insp_x + 118.0),
                        ];
                        for (name, bx) in mesh_btns {
                            if mouse.x >= bx && mouse.x <= bx + 48.0 {
                                if let Some(m3) = app.registry.get_mut::<Mesh3DComponent>(e) {
                                    m3.mesh_name = name.to_string();
                                }
                                app.audio.play("sfx_blip");
                                return true;
                            }
                        }
                    }
                    cur_insp_y += 160.0 + 8.0;
                }

                let has_tr3d = app.registry.has::<Transform3D>(e);
                let has_tr2d = app.registry.has::<Transform2D>(e);
                if has_tr2d || !has_tr3d {
                    let c1_y = cur_insp_y;

                    // 2D Sayisal Alan Odaklanmasi (Position X, Position Y)
                    if mouse.y >= c1_y + 24.0 && mouse.y <= c1_y + 40.0 {
                        if let Some(t) = app.registry.get::<Transform2D>(e) {
                            if mouse.x >= insp_x + 90.0 && mouse.x <= insp_x + 150.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::PosX,
                                    t.pos.x,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            }
                        }
                    }
                    if mouse.y >= c1_y + 44.0 && mouse.y <= c1_y + 60.0 {
                        if let Some(t) = app.registry.get::<Transform2D>(e) {
                            if mouse.x >= insp_x + 90.0 && mouse.x <= insp_x + 150.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::PosY,
                                    t.pos.y,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            }
                        }
                    }

                    // 2D Sayisal Alan Odaklanmasi (Rotation)
                    if mouse.y >= c1_y + 66.0 && mouse.y <= c1_y + 82.0 {
                        if let Some(t) = app.registry.get::<Transform2D>(e) {
                            if mouse.x >= insp_x + 90.0 && mouse.x <= insp_x + 150.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::Rot,
                                    t.rotation.to_degrees(),
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            }
                        }
                    }

                    // 2D Sayisal Alan Odaklanmasi (Scale W / H)
                    if mouse.y >= c1_y + 88.0 && mouse.y <= c1_y + 104.0 {
                        if let Some(t) = app.registry.get::<Transform2D>(e) {
                            if mouse.x >= insp_x + 90.0 && mouse.x <= insp_x + 120.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::SizeX,
                                    t.size.x,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            } else if mouse.x >= insp_x + 124.0 && mouse.x <= insp_x + 154.0 {
                                app.editor.numeric_edit = Some(InspectorNumericEdit::new(
                                    e,
                                    NumericFieldId::SizeY,
                                    t.size.y,
                                ));
                                if let Some(w) = app.window.as_ref() {
                                    w.set_ime_allowed(true);
                                }
                                return true;
                            }
                        }
                    }

                    // Position X: [-] ve [+]
                    if mouse.y >= c1_y + 24.0 && mouse.y <= c1_y + 40.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.pos.x -= 10.0;
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.pos.x += 10.0;
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                    }

                    // Position Y: [-] ve [+]
                    if mouse.y >= c1_y + 44.0 && mouse.y <= c1_y + 60.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.pos.y -= 10.0;
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.pos.y += 10.0;
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                    }

                    // Rotation: R- ve R+
                    if mouse.y >= c1_y + 66.0 && mouse.y <= c1_y + 82.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.rotation -= 0.1;
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.rotation += 0.1;
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                    }

                    // Scale: W- ve W+
                    if mouse.y >= c1_y + 88.0 && mouse.y <= c1_y + 104.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.size.x = (t.size.x - 8.0).max(8.0);
                                t.size.y = (t.size.y - 8.0).max(8.0);
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_t = app.registry.get::<Transform2D>(e).copied();
                            if let Some(t) = app.registry.get_mut::<Transform2D>(e) {
                                t.size.x += 8.0;
                                t.size.y += 8.0;
                            }
                            if let Some(old_t) = old_t {
                                app.push_transform2d_undo(e, old_t);
                            }
                            return true;
                        }
                    }

                    // Z-Index: Z- ve Z+
                    if mouse.y >= c1_y + 110.0 && mouse.y <= c1_y + 126.0 {
                        if mouse.x >= insp_x + 160.0 && mouse.x <= insp_x + 192.0 {
                            let old_z = app.registry.get::<ZIndex2D>(e).copied();
                            let new_z = if let Some(zi) = app.registry.get_mut::<ZIndex2D>(e) {
                                zi.z -= 1;
                                zi.z
                            } else {
                                app.registry.emplace(e, ZIndex2D::new(-1));
                                -1
                            };
                            app.push_zindex_undo(e, old_z);
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!("Dugum #{}: Z-Index {} yapildi.", e.index(), new_z),
                            });
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        if mouse.x >= insp_x + 196.0 && mouse.x <= insp_x + 228.0 {
                            let old_z = app.registry.get::<ZIndex2D>(e).copied();
                            let new_z = if let Some(zi) = app.registry.get_mut::<ZIndex2D>(e) {
                                zi.z += 1;
                                zi.z
                            } else {
                                app.registry.emplace(e, ZIndex2D::new(1));
                                1
                            };
                            app.push_zindex_undo(e, old_z);
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!("Dugum #{}: Z-Index {} yapildi.", e.index(), new_z),
                            });
                            app.audio.play("sfx_blip");
                            return true;
                        }
                    }
                    cur_insp_y += 136.0 + 8.0;
                }

                // Sprite 2D Doku Butonlarina Tiklama
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
                            let old_tex = app.registry.get::<SpriteTexture>(e).cloned();
                            if key == "none" {
                                app.registry.remove::<SpriteTexture>(e);
                                app.push_texture_undo(e, old_tex);
                                app.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} dokusu kaldirildi.",
                                        e.index(),
                                        e.generation()
                                    ),
                                });
                            } else {
                                app.registry.emplace(
                                    e,
                                    SpriteTexture {
                                        name: key.to_string(),
                                    },
                                );
                                app.push_texture_undo(e, old_tex);
                                app.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} dokusu res://{} olarak atandi.",
                                        e.index(),
                                        e.generation(),
                                        key
                                    ),
                                });
                            }
                            return true;
                        }
                        opt_x += 42.0;
                    }
                }

                // Sprite 2D Renk Paletine Tiklama
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
                            let old_col = app.registry.get::<SpriteColor>(e).copied();
                            if let Some(s) = app.registry.get_mut::<SpriteColor>(e) {
                                s.color = col;
                            }
                            app.push_color_undo(e, old_col);
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} rengi guncellendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            return true;
                        }
                        sw_x += 22.0;
                    }
                }

                // CollisionShape 2D Mod Degistirme Butonu ([Sensor / Kati])
                let c3_y = c2_y + 92.0;
                if mouse.y >= c3_y + 24.0
                    && mouse.y <= c3_y + 44.0
                    && mouse.x >= insp_x + 196.0
                    && mouse.x <= insp_x + 282.0
                {
                    let old_col2 = app.registry.get::<Collider2D>(e).copied();
                    if let Some(col) = app.registry.get_mut::<Collider2D>(e) {
                        col.is_sensor = !col.is_sensor;
                        let sensor_now = col.is_sensor;
                        app.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: format!(
                                "Dugum #{}:{} carpisma modu: {}",
                                e.index(),
                                e.generation(),
                                if sensor_now { "Sensor" } else { "Kati" }
                            ),
                        });
                        app.push_collider_undo(e, old_col2);
                        app.audio.play("sfx_blip");
                        return true;
                    }
                }

                // Script Butonlarina Tiklama (Rotator, Patrol, Pulse, none)
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
                            let old_scr = app.registry.get::<ScriptComponent>(e).cloned();
                            if key == "none" {
                                app.registry.remove::<ScriptComponent>(e);
                                app.push_script_undo(e, old_scr);
                                app.editor.logs.push(LogMessage {
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
                                app.registry.emplace(e, comp);
                                app.push_script_undo(e, old_scr);
                                app.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} icin res://{} betigi baglandi.",
                                        e.index(),
                                        e.generation(),
                                        key
                                    ),
                                });
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        so_x += 58.0;
                    }
                }

                cur_insp_y = c4_y + c4_h + 8.0;

                // Velocity 2D Butonlari ([-] / [+])
                let has_vel = app.registry.has::<Velocity2D>(e);
                if has_vel {
                    let v_y = cur_insp_y;
                    if mouse.y >= v_y + 24.0 && mouse.y <= v_y + 40.0 {
                        // Vx [-]
                        if mouse.x >= insp_x + 88.0 && mouse.x <= insp_x + 114.0 {
                            if let Some(v) = app.registry.get_mut::<Velocity2D>(e) {
                                v.vel.x -= 10.0;
                            }
                            return true;
                        }
                        // Vx [+]
                        if mouse.x >= insp_x + 118.0 && mouse.x <= insp_x + 144.0 {
                            if let Some(v) = app.registry.get_mut::<Velocity2D>(e) {
                                v.vel.x += 10.0;
                            }
                            return true;
                        }
                        // Vy [-]
                        if mouse.x >= insp_x + 228.0 && mouse.x <= insp_x + 254.0 {
                            if let Some(v) = app.registry.get_mut::<Velocity2D>(e) {
                                v.vel.y -= 10.0;
                            }
                            return true;
                        }
                        // Vy [+]
                        if mouse.x >= insp_x + 258.0 && mouse.x <= insp_x + 284.0 {
                            if let Some(v) = app.registry.get_mut::<Velocity2D>(e) {
                                v.vel.y += 10.0;
                            }
                            return true;
                        }
                    }
                    cur_insp_y += 52.0 + 8.0;
                }

                // Animation 2D Butonlari ([Oynat/Durdur], FPS [-]/[+])
                let has_anim = app.registry.has::<Animation2DComponent>(e);
                if has_anim {
                    let a_y = cur_insp_y;
                    if mouse.y >= a_y + 24.0 && mouse.y <= a_y + 42.0 {
                        // [Oynat/Durdur]
                        if mouse.x >= insp_x + 130.0 && mouse.x <= insp_x + 186.0 {
                            if let Some(anim) = app.registry.get_mut::<Animation2DComponent>(e) {
                                if anim.is_playing {
                                    anim.pause();
                                } else {
                                    anim.resume();
                                }
                            }
                            app.audio.play("sfx_blip");
                            return true;
                        }
                        // FPS [-]
                        if mouse.x >= insp_x + 240.0 && mouse.x <= insp_x + 262.0 {
                            if let Some(anim) = app.registry.get_mut::<Animation2DComponent>(e) {
                                let cur_clip = anim.current_clip.clone();
                                if let Some(clip) = anim.clips.get_mut(&cur_clip) {
                                    clip.fps = (clip.fps - 1.0).max(1.0);
                                }
                            }
                            return true;
                        }
                        // FPS [+]
                        if mouse.x >= insp_x + 266.0 && mouse.x <= insp_x + 288.0 {
                            if let Some(anim) = app.registry.get_mut::<Animation2DComponent>(e) {
                                let cur_clip = anim.current_clip.clone();
                                if let Some(clip) = anim.clips.get_mut(&cur_clip) {
                                    clip.fps = (clip.fps + 1.0).min(60.0);
                                }
                            }
                            return true;
                        }
                    }
                    cur_insp_y += 56.0 + 8.0;
                }

                // Tilemap 2D Butonlari (Temizle, Firca, Silgi, Karo Paleti)
                let has_tilemap = app.registry.has::<Tilemap2DComponent>(e);
                if has_tilemap {
                    let tm_y = cur_insp_y;
                    // [Haritayi Temizle]
                    if mouse.y >= tm_y + 23.0
                        && mouse.y <= tm_y + 41.0
                        && mouse.x >= insp_x + right_w - 96.0
                        && mouse.x <= insp_x + right_w - 16.0
                    {
                        if let Some(map) = app.registry.get_mut::<Tilemap2DComponent>(e) {
                            map.clear();
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} harita karolari temizlendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                        }
                        app.editor.is_scene_dirty = true;
                        app.audio.play("sfx_blip");
                        return true;
                    }

                    // [Firca] Modu Butonu
                    if mouse.y >= tm_y + 48.0
                        && mouse.y <= tm_y + 68.0
                        && mouse.x >= insp_x + 14.0
                        && mouse.x <= insp_x + 84.0
                    {
                        app.editor.active_tool = ViewportTool::TileBrush;
                        app.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Tilemap Firca Modu Aktif.".to_string(),
                        });
                        app.audio.play("sfx_blip");
                        return true;
                    }

                    // [Silgi] Modu Butonu
                    if mouse.y >= tm_y + 48.0
                        && mouse.y <= tm_y + 68.0
                        && mouse.x >= insp_x + 90.0
                        && mouse.x <= insp_x + 160.0
                    {
                        app.editor.active_tool = ViewportTool::TileEraser;
                        app.editor.logs.push(LogMessage {
                            level: "INFO",
                            text: "Tilemap Silgi Modu Aktif.".to_string(),
                        });
                        app.audio.play("sfx_blip");
                        return true;
                    }

                    // Karo Paleti Butonlari (Cimen, Tas, Metal, Lav, Su)
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
                                app.editor.active_tile_type = tt;
                                app.editor.active_tool = ViewportTool::TileBrush;
                                app.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!("Firca Karosu Secildi: {:?}", tt),
                                });
                                app.audio.play("sfx_blip");
                                return true;
                            }
                            pal_x += 52.0;
                        }
                    }

                    cur_insp_y += 104.0 + 8.0;
                }

                // BILESEN 8: Parcacik Yayici (ParticleEmitterComponent) Karti Tiklamalari
                if app.registry.has::<ParticleEmitterComponent>(e) {
                    let p_y = cur_insp_y;
                    if mouse.x >= insp_x + 8.0 && mouse.x <= insp_x + right_w - 8.0 {
                        // [Patlat (Burst)] Butonu
                        if mouse.x >= insp_x + 14.0
                            && mouse.x <= insp_x + 124.0
                            && mouse.y >= p_y + 48.0
                            && mouse.y <= p_y + 68.0
                        {
                            let pos = app
                                .registry
                                .get::<Transform2D>(e)
                                .map(|t| t.pos)
                                .unwrap_or(Vec2::ZERO);
                            if let Some(p) = app.registry.get_mut::<ParticleEmitterComponent>(e) {
                                p.burst(20, pos);
                                app.audio.play("sfx_laser");
                                app.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} icin parcacik patlatildi.",
                                        e.index(),
                                        e.generation()
                                    ),
                                });
                            }
                            return true;
                        }
                        // [Baslat / Durdur] Butonu
                        if mouse.x >= insp_x + 130.0
                            && mouse.x <= insp_x + 240.0
                            && mouse.y >= p_y + 48.0
                            && mouse.y <= p_y + 68.0
                        {
                            if let Some(p) = app.registry.get_mut::<ParticleEmitterComponent>(e) {
                                p.is_active = !p.is_active;
                                app.audio.play("sfx_blip");
                                app.editor.logs.push(LogMessage {
                                    level: "INFO",
                                    text: format!(
                                        "Dugum #{}:{} parcacik durumu: {}.",
                                        e.index(),
                                        e.generation(),
                                        if p.is_active { "Aktif" } else { "Durduruldu" }
                                    ),
                                });
                            }
                            return true;
                        }
                    }
                    cur_insp_y += 76.0 + 8.0;
                }

                // [+ Add Component] Butonu
                let add_btn_y = cur_insp_y;
                if mouse.x >= insp_x + 8.0
                    && mouse.x <= insp_x + right_w - 8.0
                    && mouse.y >= add_btn_y
                    && mouse.y <= add_btn_y + 24.0
                {
                    app.editor.show_add_component_popup = !app.editor.show_add_component_popup;
                    return true;
                }

                // Acilir Popup Menusu Tiklamalari
                if app.editor.show_add_component_popup {
                    let pop_y = add_btn_y + 28.0;
                    if mouse.x >= insp_x + 8.0 && mouse.x <= insp_x + right_w - 8.0 {
                        if mouse.y >= pop_y + 4.0 && mouse.y <= pop_y + 24.0 {
                            app.registry.emplace(
                                e,
                                Collider2D {
                                    is_dynamic: true,
                                    is_sensor: false,
                                },
                            );
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Collider2D eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        } else if mouse.y >= pop_y + 26.0 && mouse.y <= pop_y + 46.0 {
                            app.registry.emplace(
                                e,
                                Velocity2D {
                                    vel: Vec2::new(50.0, 50.0),
                                },
                            );
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Velocity2D eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        } else if mouse.y >= pop_y + 48.0 && mouse.y <= pop_y + 68.0 {
                            app.registry.emplace(e, ScriptComponent::new_rotator(2.0));
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin ScriptComponent (Rotator) eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        } else if mouse.y >= pop_y + 70.0 && mouse.y <= pop_y + 90.0 {
                            app.registry.emplace(
                                e,
                                Text2DComponent {
                                    text: format!("Label_{}", e.index()),
                                    scale: 1.4,
                                    color: [240, 240, 240, 255],
                                },
                            );
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Text2DComponent eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        } else if mouse.y >= pop_y + 92.0 && mouse.y <= pop_y + 112.0 {
                            let clip = AnimationClip::new(
                                "idle",
                                vec!["player_ship".to_string(), "energy_orb".to_string()],
                                4.0,
                                true,
                            );
                            app.registry
                                .emplace(e, Animation2DComponent::new().with_clip(clip));
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Animation2DComponent eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        } else if mouse.y >= pop_y + 114.0 && mouse.y <= pop_y + 134.0 {
                            app.registry.emplace(e, Tilemap2DComponent::new(32.0));
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin Tilemap2DComponent eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        } else if mouse.y >= pop_y + 136.0 && mouse.y <= pop_y + 156.0 {
                            let pos = app
                                .registry
                                .get::<Transform2D>(e)
                                .map(|t| t.pos)
                                .unwrap_or(Vec2::ZERO);
                            app.registry
                                .emplace(e, ParticleEmitterComponent::new_explosion(64, pos));
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin ParticleEmitterComponent (Patlama) eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        } else if mouse.y >= pop_y + 158.0 && mouse.y <= pop_y + 178.0 {
                            app.registry.emplace(e, ZIndex2D::new(1));
                            app.editor.logs.push(LogMessage {
                                level: "INFO",
                                text: format!(
                                    "Dugum #{}:{} icin ZIndex2D (Katman 1) eklendi.",
                                    e.index(),
                                    e.generation()
                                ),
                            });
                            app.editor.show_add_component_popup = false;
                            app.audio.play("sfx_blip");
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}
