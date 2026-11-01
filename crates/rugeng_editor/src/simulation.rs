use crate::types::*;
use crate::App;
use glam::Vec2;
use rugeng_core::{
    compute_world_transform, Animation2DComponent, Collider2D, Entity, Parent,
    ParticleEmitterComponent, ScriptComponent, SpriteTexture, Tilemap2DComponent, Transform2D,
    Velocity2D,
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
        let pool_tr = unsafe { self.registry.storage_mut::<Transform2D>() };

        // 3. Fizik ve Hız Çarpma Mantığı
        let pool_vel = unsafe { self.registry.storage_mut::<Velocity2D>() };
        let (entities, vels) = pool_vel.split_entities_and_components_mut();
        for (idx, &e) in entities.iter().enumerate() {
            let vel = &mut vels[idx];
            if let Some(tr) = pool_tr.get_mut(e) {
                tr.pos += vel.vel * dt;
                if tr.pos.x > 320.0 || tr.pos.x < -320.0 {
                    vel.vel.x = -vel.vel.x;
                }
                if tr.pos.y > 220.0 || tr.pos.y < -220.0 {
                    vel.vel.y = -vel.vel.y;
                }
            }
        }

        // 4. Betik Yaşam Döngüsü Güncellemesi
        let pool_scripts = unsafe { self.registry.storage_mut::<ScriptComponent>() };
        let (script_entities, scripts) = pool_scripts.split_entities_and_components_mut();
        for (idx, &e) in script_entities.iter().enumerate() {
            let script = &mut scripts[idx];
            if let Some(tr) = pool_tr.get_mut(e) {
                script.update(dt, &mut tr.pos, &mut tr.size, &mut tr.rotation);
            }
        }

        // 5. Hiyerarşi (Parent - Child Transform Senkronizasyonu)
        let pool_parent = unsafe { self.registry.storage::<Parent>() };
        for &e in pool_parent.entities() {
            if let Some(parent) = pool_parent.get(e) {
                if let Some(parent_tr) = pool_tr.get(parent.entity).copied() {
                    if let Some(child_tr) = pool_tr.get_mut(e) {
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
        }

        // 6. Katı AABB Çarpışma Ayrıştırması (Penetration Resolution)
        let pool_col = unsafe { self.registry.storage::<Collider2D>() };
        let col_entities: Vec<Entity> = pool_col.entities().to_vec();
        for i in 0..col_entities.len() {
            for j in (i + 1)..col_entities.len() {
                let e_a = col_entities[i];
                let e_b = col_entities[j];
                let col_a = pool_col.get(e_a);
                let col_b = pool_col.get(e_b);

                let is_sensor_a = col_a.map(|c| c.is_sensor).unwrap_or(false);
                let is_sensor_b = col_b.map(|c| c.is_sensor).unwrap_or(false);
                if is_sensor_a || is_sensor_b {
                    continue;
                }

                let is_dyn_a = col_a.map(|c| c.is_dynamic).unwrap_or(false);
                let is_dyn_b = col_b.map(|c| c.is_dynamic).unwrap_or(false);

                if !is_dyn_a && !is_dyn_b {
                    continue;
                }

                let tr_a_opt = pool_tr.get(e_a).copied();
                let tr_b_opt = pool_tr.get(e_b).copied();

                if let (Some(mut tr_a), Some(mut tr_b)) = (tr_a_opt, tr_b_opt) {
                    if resolve_aabb_collision(
                        &mut tr_a.pos,
                        tr_a.size,
                        is_dyn_a,
                        &mut tr_b.pos,
                        tr_b.size,
                        is_dyn_b,
                    ) {
                        if let Some(t) = pool_tr.get_mut(e_a) {
                            t.pos = tr_a.pos;
                        }
                        if let Some(t) = pool_tr.get_mut(e_b) {
                            t.pos = tr_b.pos;
                        }
                    }
                }
            }
        }

        // 7. 2D Sprite Kare Animasyon Güncellemesi
        let pool_anims = unsafe { self.registry.storage_mut::<Animation2DComponent>() };
        let pool_tex = unsafe { self.registry.storage_mut::<SpriteTexture>() };
        let (anim_entities, anims) = pool_anims.split_entities_and_components_mut();
        for (idx, &e) in anim_entities.iter().enumerate() {
            let anim = &mut anims[idx];
            if anim.update(dt) {
                if let Some(tex_name) = anim.current_texture_name() {
                    if let Some(tex_comp) = pool_tex.get_mut(e) {
                        tex_comp.name = tex_name.to_string();
                    }
                }
            }
        }

        // 8. Tilemap Katı Karoları ile Dinamik Nesne Çarpışması
        let pool_tilemaps = unsafe { self.registry.storage::<Tilemap2DComponent>() };
        for &tm_e in pool_tilemaps.entities() {
            if let Some(map) = pool_tilemaps.get(tm_e) {
                let solid_boxes = map.solid_aabb_boxes();
                for &e in &col_entities {
                    let is_dyn = pool_col.get(e).map(|c| c.is_dynamic).unwrap_or(false);
                    if is_dyn {
                        if let Some(tr) = pool_tr.get_mut(e) {
                            for &(tile_center, tile_sz) in &solid_boxes {
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
        }
    }
}
