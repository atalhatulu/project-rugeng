#![allow(unused_imports)]
use crate::types::*;
use crate::ui::dock_layout::DockLayout;
use crate::App;
use glam::Mat4;
use rugeng_core::{Entity, Mesh3DComponent, SpriteColor, Transform2D, Transform3D, ZIndex2D};
use rugeng_render::pipeline_2d::QuadInstance;

impl App {
    pub(crate) fn render_vulkan_frame(&mut self, instances: &[QuadInstance], layout: &DockLayout) {
        let selected_e = self.editor.selected_entity;
        let screen_w = layout.screen_w;
        let screen_h = layout.screen_h;
        let vp_x = layout.vp_x;
        let vp_w = layout.vp_w;
        let vp_canvas_y = layout.vp_canvas_y;
        let vp_canvas_h = layout.vp_canvas_h;
        let theme = self.editor.theme;

        // =========================================================================
        // VULKAN 1.3 DYNAMIC RENDERING (TEK DRAW CALL)
        // =========================================================================
        let arena_instances = self.arena.alloc_slice_copy(instances).unwrap();
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

                    // B. Sahnedeki Noktasal Işıkları Topla ve SSBO'ya Aktar
                    let point_lights: Vec<(Transform3D, rugeng_core::PointLight)> = self
                        .registry
                        .entities::<rugeng_core::PointLight>()
                        .into_iter()
                        .filter_map(|e| {
                            let pl = self.registry.get::<rugeng_core::PointLight>(e).copied()?;
                            let t3 = self
                                .registry
                                .get::<Transform3D>(e)
                                .copied()
                                .unwrap_or_else(|| Transform3D::from_xyz(0.0, 0.0, 0.0));
                            Some((t3, pl))
                        })
                        .collect();

                    let num_lights = point_lights.len().min(64) as u32;
                    if num_lights > 0 {
                        let mut gpu_lights = [rugeng_render::GpuPointLight {
                            position: [0.0; 4],
                            color: [0.0; 4],
                        }; 64];
                        for (idx, (t3, pl)) in point_lights.iter().take(64).enumerate() {
                            gpu_lights[idx] = rugeng_render::GpuPointLight {
                                position: [
                                    t3.translation.x,
                                    t3.translation.y,
                                    t3.translation.z,
                                    pl.radius,
                                ],
                                color: [pl.color[0], pl.color[1], pl.color[2], pl.intensity],
                            };
                        }
                        pipe_3d
                            .upload_point_lights(&ctx.device, &gpu_lights[0..num_lights as usize]);
                    }

                    // C. Cascaded Shadow Maps (CSM) Verisini Yükle (Binding 5)
                    let csm_splits = rugeng_render::DirectionalLight::calculate_csm_splits(
                        render_cam.z_near,
                        250.0,
                        0.75,
                    );
                    let csm_mats = self.sun.csm_matrices(&render_cam, csm_splits);
                    let mut csm_mat_arrays = [[0.0f32; 16]; 4];
                    for i in 0..4 {
                        csm_mat_arrays[i] = csm_mats[i].to_cols_array();
                    }
                    let gpu_csm = rugeng_render::GpuCsmData {
                        csm_light_space: csm_mat_arrays,
                        split_distances: csm_splits,
                    };
                    pipe_3d.upload_csm_data(&ctx.device, &gpu_csm);

                    // D. Sahnedeki 3D Varlıklar
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
                            name if name == "gltf_sample"
                                || name.starts_with("SampleTriangleMesh")
                                || name.contains("submesh") =>
                            {
                                self.gpu_gltf_sample.as_ref().or(self.gpu_cube.as_ref())
                            }
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

                            pipe_3d.draw_mesh_pbr_viewport_ext_with_lights(
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
                                false,
                                None,
                                num_lights,
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
    }
}
