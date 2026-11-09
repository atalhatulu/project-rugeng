use crate::types::*;
use crate::App;
use glam::Vec2;
use rugeng_core::{
    compute_world_transform, Animation2DComponent, Animation3DComponent, Collider2D, Entity,
    Parent, ParticleEmitterComponent, ScriptComponent, SpriteTexture, Tilemap2DComponent,
    Transform2D, Transform3D, Velocity2D,
};
use rugeng_physics::resolve_aabb_collision;
impl App {
    pub(crate) fn update_simulation(&mut self, dt: f32) {
        // 1. 3D Serbest Uçuş & Flycam Kamerası Hareketi (ThreeD modunda ve SADECE SAĞ TIK BASILI İKEN)
        if self.editor.workspace == WorkspaceMode::ThreeD && self.editor.camera_3d.is_flying {
            let speed = self.editor.camera_3d.effective_speed() * dt;
            let fwd = self.editor.camera_3d.forward();
            let right = self.editor.camera_3d.right();
            let up = glam::Vec3::Y; // Dünya dikey Y ekseni

            // W: İleri (Kameranın baktığı yöne doğru)
            if self.action_map.is_action_pressed("move_up") {
                self.editor.camera_3d.eye += fwd * speed;
            }
            // S: Geri (Kameranın baktığı yönün tersine)
            if self.action_map.is_action_pressed("move_down") {
                self.editor.camera_3d.eye -= fwd * speed;
            }
            // A: Sola kayma (Strafe Left)
            if self.action_map.is_action_pressed("move_left") {
                self.editor.camera_3d.eye -= right * speed;
            }
            // D: Sağa kayma (Strafe Right)
            if self.action_map.is_action_pressed("move_right") {
                self.editor.camera_3d.eye += right * speed;
            }
            // E veya Space: Dikey Yukarı yükselme (Fly Up)
            if self.action_map.is_action_pressed("fly_up")
                || self.action_map.is_action_pressed("action_jump")
            {
                self.editor.camera_3d.eye += up * speed;
            }
            // Q: Dikey Aşağı alçalma (Fly Down)
            if self.action_map.is_action_pressed("fly_down") {
                self.editor.camera_3d.eye -= up * speed;
            }
        }

        // 2. Kamera Takibi ve Ekran Sarsıntısı (Screen Shake) - Her zaman sönümlenir
        let target_pos = if self.editor.is_playing {
            self.editor
                .camera
                .follow_target
                .and_then(|tgt| self.registry.get::<Transform2D>(tgt).map(|t| t.pos))
        } else {
            None
        };
        self.editor.camera.update(dt, target_pos);

        // 2. Parçacık Sistemi Güncellemesi - Parçacıkların pürüzsüz akması için daima güncellenir
        let mut finished_emitters = Vec::new();
        {
            let pool_tr = unsafe { self.registry.storage::<Transform2D>() };
            let pool_emitters = unsafe { self.registry.storage_mut::<ParticleEmitterComponent>() };
            let (emitter_entities, emitters) = pool_emitters.split_entities_and_components_mut();
            for (idx, &e) in emitter_entities.iter().enumerate() {
                let emitter = &mut emitters[idx];
                let origin = pool_tr.get(e).map(|t| t.pos).unwrap_or(Vec2::ZERO);
                emitter.update(dt, origin);
                if emitter.is_burst && emitter.particles.is_empty() {
                    finished_emitters.push(e);
                }
            }
        }
        for fe in finished_emitters {
            self.cmd_buffer.destroy(fe);
        }
        self.cmd_buffer.apply(&mut self.registry);

        if !self.editor.is_playing {
            return;
        }

        self.step_simulation_single_frame(dt);
    }

    pub(crate) fn step_simulation_single_frame(&mut self, dt: f32) {
        // 3. Fizik ve Hız Çarpma Mantığı (safe view2_mut_ref: Transform2D mut, Velocity2D ref)
        // We iterate and update Transform2D based on Velocity2D. For bouncing, we collect bounces or mutate via get_mut.
        let mut bounce_vels = Vec::new();
        self.registry
            .view2_mut_ref::<Transform2D, Velocity2D, _>(|e, tr, vel| {
                tr.pos += vel.vel * dt;
                let mut new_vel = vel.vel;
                let mut bounced = false;
                if tr.pos.x > 320.0 || tr.pos.x < -320.0 {
                    new_vel.x = -new_vel.x;
                    bounced = true;
                }
                if tr.pos.y > 220.0 || tr.pos.y < -220.0 {
                    new_vel.y = -new_vel.y;
                    bounced = true;
                }
                if bounced {
                    bounce_vels.push((e, new_vel));
                }
            });
        for (e, new_v) in bounce_vels {
            if let Some(vel) = self.registry.get_mut::<Velocity2D>(e) {
                vel.vel = new_v;
            }
        }

        // 4. Betik Yaşam Döngüsü Güncellemesi
        let mut script_data: Vec<(Entity, ScriptComponent)> = Vec::new();
        self.registry.view1_ref::<ScriptComponent, _>(|e, s| {
            script_data.push((e, s.clone()));
        });
        for (e, mut script) in script_data {
            if let Some(tr) = self.registry.get_mut::<Transform2D>(e) {
                script.update(dt, &mut tr.pos, &mut tr.size, &mut tr.rotation);
            }
            if let Some(s) = self.registry.get_mut::<ScriptComponent>(e) {
                *s = script;
            }
        }

        // 5. Hiyerarşi (Parent - Child Transform Senkronizasyonu)
        // We read Parent components and update child Transform2D based on parent's Transform2D
        let parent_links: Vec<(Entity, Entity)> = {
            let mut links = Vec::new();
            self.registry.view1_ref::<Parent, _>(|child, p| {
                links.push((child, p.entity));
            });
            links
        };
        for (child, parent) in parent_links {
            if let Some(parent_tr) = self.registry.get::<Transform2D>(parent).copied() {
                if let Some(child_tr) = self.registry.get_mut::<Transform2D>(child) {
                    let (w_pos, w_rot) = compute_world_transform(
                        Vec2::new(24.0, 0.0),
                        0.0,
                        parent_tr.pos,
                        parent_tr.rotation,
                    );
                    child_tr.pos = w_pos;
                    child_tr.rotation = w_rot;
                }
            }
        }

        // 6. Katı AABB Çarpışma Ayrıştırması (Penetration Resolution)
        let col_data: Vec<(Entity, bool, bool, Option<Transform2D>)> = {
            let mut list = Vec::new();
            self.registry.view1_ref::<Collider2D, _>(|e, col| {
                if !col.is_sensor {
                    let tr = self.registry.get::<Transform2D>(e).copied();
                    list.push((e, col.is_sensor, col.is_dynamic, tr));
                }
            });
            list
        };

        for i in 0..col_data.len() {
            for j in (i + 1)..col_data.len() {
                let (e_a, _, is_dyn_a, tr_a_opt) = col_data[i];
                let (e_b, _, is_dyn_b, tr_b_opt) = col_data[j];

                if !is_dyn_a && !is_dyn_b {
                    continue;
                }

                // SAFETY: We verify distinct entities (i != j implies e_a != e_b).
                // Both entity positions are resolved and written back through safe registry getters.
                debug_assert_ne!(e_a, e_b, "Colliders in pair resolution must be distinct");

                let cur_a_opt = self.registry.get::<Transform2D>(e_a).copied().or(tr_a_opt);
                let cur_b_opt = self.registry.get::<Transform2D>(e_b).copied().or(tr_b_opt);

                if let (Some(mut tr_a), Some(mut tr_b)) = (cur_a_opt, cur_b_opt) {
                    if resolve_aabb_collision(
                        &mut tr_a.pos,
                        tr_a.size,
                        is_dyn_a,
                        &mut tr_b.pos,
                        tr_b.size,
                        is_dyn_b,
                    ) {
                        if let Some(t) = self.registry.get_mut::<Transform2D>(e_a) {
                            t.pos = tr_a.pos;
                        }
                        if let Some(t) = self.registry.get_mut::<Transform2D>(e_b) {
                            t.pos = tr_b.pos;
                        }
                    }
                }
            }
        }

        // 7. 2D Sprite Kare Animasyon Güncellemesi
        let mut anim_updates = Vec::new();
        self.registry
            .view1_mut::<Animation2DComponent, _>(|e, anim| {
                if anim.update(dt) {
                    if let Some(tex_name) = anim.current_texture_name() {
                        anim_updates.push((e, tex_name.to_string()));
                    }
                }
            });
        for (e, tex_name) in anim_updates {
            if let Some(tex_comp) = self.registry.get_mut::<SpriteTexture>(e) {
                tex_comp.name = tex_name;
            }
        }

        // 8. Tilemap Katı Karoları ile Dinamik Nesne Çarpışması
        let solid_boxes_list: Vec<Vec<(Vec2, Vec2)>> = {
            let mut list = Vec::new();
            self.registry.view1_ref::<Tilemap2DComponent, _>(|_e, map| {
                list.push(map.solid_aabb_boxes());
            });
            list
        };

        if !solid_boxes_list.is_empty() {
            let dynamic_colliders: Vec<Entity> = {
                let mut list = Vec::new();
                self.registry.view1_ref::<Collider2D, _>(|e, col| {
                    if col.is_dynamic && !col.is_sensor {
                        list.push(e);
                    }
                });
                list
            };

            for e in dynamic_colliders {
                if let Some(tr) = self.registry.get_mut::<Transform2D>(e) {
                    for solid_boxes in &solid_boxes_list {
                        for &(tile_center, tile_sz) in solid_boxes {
                            let mut dummy_tile_pos = tile_center;
                            resolve_aabb_collision(
                                &mut tr.pos,
                                tr.size,
                                true,
                                &mut dummy_tile_pos,
                                tile_sz,
                                false,
                            );
                        }
                    }
                }
            }
        }

        // 9. 3D Simülasyon Adımı (3D Varlıkların Transform Hareketi)
        self.registry.view1_mut::<Transform3D, _>(|_e, tr3d| {
            let delta_rot = glam::Quat::from_rotation_y(0.5 * dt);
            tr3d.rotation = (tr3d.rotation * delta_rot).normalize();
            tr3d.translation.y += 0.1 * dt;
        });

        // 10. 3D İskelet Animasyonu Güncellemesi
        self.registry
            .view1_mut::<Animation3DComponent, _>(|_e, anim3d| {
                anim3d.update(dt);
            });
    }

    /// Simülasyonu başlatır ve mevcut sahne durumunun (2D ve 3D) snapshot'ını alır
    pub(crate) fn start_simulation(&mut self) {
        if self.editor.scene_snapshot.is_empty()
            && self.editor.scene_snapshot_3d.is_empty()
            && self.editor.scene_snapshot_anim_3d.is_empty()
        {
            self.registry.view1_ref::<Transform2D, _>(|e, t| {
                self.editor.scene_snapshot.push((e, *t));
            });
            self.registry.view1_ref::<Transform3D, _>(|e, t| {
                self.editor.scene_snapshot_3d.push((e, *t));
            });
            self.registry.view1_ref::<Animation3DComponent, _>(|e, a| {
                self.editor.scene_snapshot_anim_3d.push((e, a.clone()));
            });
        }
        self.editor.is_playing = true;
    }

    /// Simülasyonu durdurur ve sahne durumunu (2D ve 3D) snapshot anına geri yükler (rollback)
    pub(crate) fn stop_simulation(&mut self) {
        self.editor.is_playing = false;
        for &(e, t) in &self.editor.scene_snapshot {
            if let Some(cur_t) = self.registry.get_mut::<Transform2D>(e) {
                *cur_t = t;
            }
        }
        self.editor.scene_snapshot.clear();
        for &(e, t) in &self.editor.scene_snapshot_3d {
            if let Some(cur_t) = self.registry.get_mut::<Transform3D>(e) {
                *cur_t = t;
            }
        }
        self.editor.scene_snapshot_3d.clear();
        for (e, a) in self.editor.scene_snapshot_anim_3d.drain(..) {
            if let Some(cur_a) = self.registry.get_mut::<Animation3DComponent>(e) {
                *cur_a = a;
            }
        }
    }
}
