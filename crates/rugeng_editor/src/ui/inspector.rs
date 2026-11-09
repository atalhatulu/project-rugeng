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

                // Position X/Y/Z
                font::draw_text(
                    instances,
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

                // Rotation Euler
                let (pitch, yaw, roll) = t3.rotation.to_euler(glam::EulerRot::YXZ);
                font::draw_text(
                    instances,
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

                // Scale X/Y/Z
                font::draw_text(
                    instances,
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
                font::draw_text(
                    instances,
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
                font::draw_text(
                    instances,
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
                font::draw_text(
                    instances,
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
